import Anthropic from "@anthropic-ai/sdk";
import { Conversation } from "./conversation.js";
import { buildSystemPrompt } from "./system-prompt.js";
import { getAllToolSchemas, executeTool } from "./tools/index.js";
import type { PermissionSet } from "./tools/types.js";

export interface AgentConfig {
  model: string;
  apiKey: string;
  projectDir: string;
  permissions: PermissionSet;
  maxToolRounds?: number;
}

export interface StreamEvent {
  type: "text" | "tool_call" | "tool_result" | "done" | "error";
  content?: string;
  tool?: string;
  args?: Record<string, unknown>;
  result?: string;
  isError?: boolean;
}

const MAX_RETRIES = 3;
const RETRY_DELAYS = [2000, 4000, 8000];

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

export async function* runAgent(
  userMessage: string,
  conversation: Conversation,
  config: AgentConfig,
): AsyncGenerator<StreamEvent> {
  const client = new Anthropic({ apiKey: config.apiKey });
  const systemPrompt = buildSystemPrompt(config.projectDir);
  const tools = getAllToolSchemas();

  conversation.addUser(userMessage);

  let toolRounds = 0;
  const maxRounds = config.maxToolRounds ?? 10;

  while (toolRounds < maxRounds) {
    // Retry wrapper for the entire API call + stream consumption
    let finalMessage: Anthropic.Message | null = null;
    let streamedText = "";

    for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
      try {
        streamedText = "";
        const stream = client.messages.stream({
          model: config.model,
          max_tokens: 8192,
          system: systemPrompt,
          messages: conversation.getMessages(),
          tools,
        });

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
        break; // Success — exit retry loop
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
        // Final attempt failed or non-retryable error
        yield {
          type: "error",
          content: `API ERROR: ${(err as Error).message}`,
        };
        return;
      }
    }

    if (!finalMessage) {
      yield { type: "error", content: "API UNAVAILABLE AFTER RETRIES" };
      return;
    }

    // Process the response
    const toolUseBlocks: Array<{ id: string; name: string; input: unknown }> = [];

    for (const block of finalMessage.content) {
      if (block.type === "tool_use") {
        toolUseBlocks.push({ id: block.id, name: block.name, input: block.input });
      }
    }

    conversation.addAssistant(finalMessage.content);

    if (toolUseBlocks.length === 0) {
      yield { type: "done" };
      return;
    }

    // Execute tools
    const results: Array<{ tool_use_id: string; content: string; is_error?: boolean }> = [];
    for (const toolBlock of toolUseBlocks) {
      yield {
        type: "tool_call",
        tool: toolBlock.name,
        args: toolBlock.input as Record<string, unknown>,
      };

      const result = await executeTool(toolBlock.name, toolBlock.input, config.permissions);
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
}
