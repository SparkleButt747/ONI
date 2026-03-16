import Anthropic from "@anthropic-ai/sdk";
import { Conversation } from "./conversation.js";
import { buildSystemPrompt, type ContextChunk } from "./system-prompt.js";
import { getAllToolSchemas, executeTool } from "./tools/index.js";
import type { PermissionSet } from "./tools/types.js";

export interface AgentConfig {
  model: string;
  apiKey: string;
  projectDir: string;
  permissions: PermissionSet;
  maxToolRounds?: number;
  contextChunks?: ContextChunk[];
}

export interface StreamEvent {
  type: "text" | "tool_call" | "tool_result" | "done" | "error" | "status";
  content?: string;
  tool?: string;
  args?: Record<string, unknown>;
  result?: string;
  isError?: boolean;
}

// Stream-level retry: SDK handles 429/500/529 for non-streaming calls,
// but streaming can fail mid-response. We retry the whole stream call.
const STREAM_RETRIES = 3;
const STREAM_DELAYS = [1000, 3000, 6000];

function isRetryable(err: unknown): boolean {
  const msg = String((err as Error)?.message ?? err);
  return (
    msg.includes("500") ||
    msg.includes("529") ||
    msg.includes("overloaded") ||
    msg.includes("Internal server error") ||
    msg.includes("ECONNRESET") ||
    msg.includes("socket hang up") ||
    msg.includes("ETIMEDOUT") ||
    msg.includes("fetch failed")
  );
}

export async function* runAgent(
  userMessage: string,
  conversation: Conversation,
  config: AgentConfig,
): AsyncGenerator<StreamEvent> {
  // All tokens sent as apiKey — OAuth tokens (sk-ant-oat*) may get intermittent 500s
  // but there's no official third-party OAuth support from Anthropic
  const client = new Anthropic({
    apiKey: config.apiKey,
    maxRetries: 4,
    timeout: 5 * 60 * 1000,
  });

  const systemPrompt = buildSystemPrompt(config.projectDir, config.contextChunks);
  const tools = getAllToolSchemas();

  conversation.addUser(userMessage);

  let toolRounds = 0;
  const maxRounds = config.maxToolRounds ?? 10;

  while (toolRounds < maxRounds) {
    let finalMessage: Anthropic.Message | null = null;

    // Stream-level retry: the SDK doesn't auto-retry mid-stream failures
    for (let attempt = 0; attempt < STREAM_RETRIES; attempt++) {
      try {
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
              yield { type: "text", content: delta.text };
            }
          }
        }

        finalMessage = await stream.finalMessage();
        break; // Success
      } catch (err) {
        if (attempt < STREAM_RETRIES - 1 && isRetryable(err)) {
          const delay = STREAM_DELAYS[attempt];
          yield {
            type: "status",
            content: `⟳ RETRYING (${attempt + 1}/${STREAM_RETRIES})`,
          };
          await new Promise((r) => setTimeout(r, delay));
          continue;
        }
        yield {
          type: "error",
          content: `${(err as Error).message}`,
        };
        return;
      }
    }

    if (!finalMessage) {
      yield { type: "error", content: "API UNAVAILABLE" };
      return;
    }

    // Process response
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
