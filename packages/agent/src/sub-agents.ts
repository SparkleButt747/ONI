import Anthropic from "@anthropic-ai/sdk";
import { Conversation } from "./conversation.js";
import { getAllToolSchemas, executeTool } from "./tools/index.js";
import type { PermissionSet } from "./tools/types.js";
import type { StreamEvent } from "./client.js";
import {
  type SubAgentRole,
  buildSubAgentPrompt,
  buildPlannerUserMessage,
  buildExecutorUserMessage,
  buildCriticUserMessage,
} from "./sub-agent-prompts.js";

export interface SubAgentConfig {
  model: string;
  apiKey: string;
  projectDir: string;
  permissions: PermissionSet;
  maxReplanCount?: number;
  maxToolRounds?: number;
}

export type SubAgentEvent =
  | { type: "agent_start"; agent: SubAgentRole }
  | { type: "agent_end"; agent: SubAgentRole }
  | { type: "blocked"; reason: string }
  | StreamEvent;

const MAX_RETRIES = 3;
const RETRY_DELAYS = [2000, 4000, 8000];
const MAX_REPLAN = 2;

function isRetryable(err: unknown): boolean {
  const msg = String((err as Error)?.message ?? err);
  return (
    msg.includes("500") ||
    msg.includes("529") ||
    msg.includes("overloaded") ||
    msg.includes("Internal server error") ||
    msg.includes("ECONNRESET") ||
    msg.includes("socket hang up")
  );
}

async function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

/**
 * Run a single sub-agent call (planner, executor, or critic).
 * Yields StreamEvents for text/tool_call/tool_result and returns the
 * accumulated text output from the agent.
 */
async function* runSubAgent(
  role: SubAgentRole,
  userMessage: string,
  config: SubAgentConfig,
): AsyncGenerator<StreamEvent, string> {
  const client = new Anthropic({
    apiKey: config.apiKey,
    maxRetries: 4,
    timeout: 5 * 60 * 1000,
  });
  const systemPrompt = buildSubAgentPrompt(role, config.projectDir);
  const tools = role === "executor" ? getAllToolSchemas() : [];
  const conversation = new Conversation();

  conversation.addUser(userMessage);

  let toolRounds = 0;
  const maxRounds = config.maxToolRounds ?? 10;
  let accumulatedText = "";

  while (toolRounds < maxRounds) {
    let finalMessage: Anthropic.Message | null = null;
    let streamedText = "";

    for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
      try {
        streamedText = "";
        const streamOpts: Record<string, unknown> = {
          model: config.model,
          max_tokens: 8192,
          system: systemPrompt,
          messages: conversation.getMessages(),
        };
        if (tools.length > 0) {
          streamOpts.tools = tools;
        }

        const stream = client.messages.stream(
          streamOpts as Anthropic.MessageStreamParams,
        );

        for await (const event of stream) {
          if (event.type === "content_block_delta") {
            const delta = event.delta;
            if ("text" in delta && delta.text) {
              streamedText += delta.text;
              yield { type: "text", content: delta.text };
            }
          }
        }

        finalMessage = await stream.finalMessage();
        break;
      } catch (err) {
        if (attempt < MAX_RETRIES - 1 && isRetryable(err)) {
          const delay = RETRY_DELAYS[attempt];
          yield {
            type: "text",
            content: `\n[API ERROR — RETRY ${attempt + 1}/${MAX_RETRIES} IN ${delay / 1000}S...]\n`,
          };
          await sleep(delay);
          continue;
        }
        yield {
          type: "error",
          content: `API ERROR: ${(err as Error).message}`,
        };
        return accumulatedText;
      }
    }

    if (!finalMessage) {
      yield { type: "error", content: "API UNAVAILABLE AFTER RETRIES" };
      return accumulatedText;
    }

    accumulatedText += streamedText;

    // Process tool use blocks (executor only)
    const toolUseBlocks: Array<{ id: string; name: string; input: unknown }> = [];
    for (const block of finalMessage.content) {
      if (block.type === "tool_use") {
        toolUseBlocks.push({ id: block.id, name: block.name, input: block.input });
      }
    }

    conversation.addAssistant(finalMessage.content);

    if (toolUseBlocks.length === 0) {
      return accumulatedText;
    }

    // Execute tools
    const results: Array<{ tool_use_id: string; content: string; is_error?: boolean }> = [];
    for (const toolBlock of toolUseBlocks) {
      yield {
        type: "tool_call",
        tool: toolBlock.name,
        args: toolBlock.input as Record<string, unknown>,
      };

      const result = await executeTool(
        toolBlock.name,
        toolBlock.input,
        config.permissions,
      );
      results.push({
        tool_use_id: toolBlock.id,
        content: result.output,
        is_error: result.isError,
      });

      yield {
        type: "tool_result",
        tool: toolBlock.name,
        result: result.output,
        isError: result.isError,
      };
    }

    conversation.addToolResults(results);
    toolRounds++;
  }

  yield { type: "error", content: "MAX TOOL ROUNDS EXCEEDED" };
  return accumulatedText;
}

/**
 * Planner -> Executor -> Critic loop.
 *
 * If the critic rejects, replans up to maxReplanCount (default 2) times.
 * If still rejected after max replans, yields a "blocked" event.
 */
export async function* runWithSubAgents(
  mission: string,
  config: SubAgentConfig,
): AsyncGenerator<SubAgentEvent> {
  const maxReplan = config.maxReplanCount ?? MAX_REPLAN;
  let replanCount = 0;

  while (replanCount <= maxReplan) {
    // --- PLANNER ---
    yield { type: "agent_start", agent: "planner" };
    const plannerGen = runSubAgent(
      "planner",
      buildPlannerUserMessage(mission),
      config,
    );
    let plan = "";
    // Consume the generator, forwarding events and capturing the return
    let planResult = await plannerGen.next();
    while (!planResult.done) {
      yield planResult.value;
      planResult = await plannerGen.next();
    }
    plan = planResult.value;
    yield { type: "agent_end", agent: "planner" };

    if (!plan) {
      yield { type: "error", content: "Planner produced no output" };
      return;
    }

    // --- EXECUTOR ---
    yield { type: "agent_start", agent: "executor" };
    const executorGen = runSubAgent(
      "executor",
      buildExecutorUserMessage(mission, plan),
      config,
    );
    let executorOutput = "";
    let execResult = await executorGen.next();
    while (!execResult.done) {
      yield execResult.value;
      execResult = await executorGen.next();
    }
    executorOutput = execResult.value;
    yield { type: "agent_end", agent: "executor" };

    if (!executorOutput) {
      yield { type: "error", content: "Executor produced no output" };
      return;
    }

    // --- CRITIC ---
    yield { type: "agent_start", agent: "critic" };
    const criticGen = runSubAgent(
      "critic",
      buildCriticUserMessage(mission, executorOutput),
      config,
    );
    let criticOutput = "";
    let criticResult = await criticGen.next();
    while (!criticResult.done) {
      yield criticResult.value;
      criticResult = await criticGen.next();
    }
    criticOutput = criticResult.value;
    yield { type: "agent_end", agent: "critic" };

    // Parse critic verdict
    const accepted = criticOutput.toUpperCase().includes("ACCEPTED");

    if (accepted) {
      yield { type: "done" };
      return;
    }

    // Rejected
    replanCount++;
    if (replanCount > maxReplan) {
      yield {
        type: "blocked",
        reason: `Critic rejected ${replanCount} times. Last feedback:\n${criticOutput}`,
      };
      return;
    }

    // Append rejection context to mission for replan
    yield {
      type: "text",
      content: `\n[⊘ CRITIC REJECTED — REPLAN ${replanCount}/${maxReplan}]\n`,
    };
  }
}
