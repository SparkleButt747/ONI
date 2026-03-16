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
    // Retry logic for transient API errors (500, 529, network)
    let stream: ReturnType<typeof client.messages.stream> | null = null;
    for (let attempt = 0; attempt < 3; attempt++) {
      try {
        stream = client.messages.stream({
          model: config.model,
          max_tokens: 8192,
          system: systemPrompt,
          messages: conversation.getMessages(),
          tools,
        });
        // Test the stream by awaiting the first event boundary
        break;
      } catch (err) {
        const msg = (err as Error).message;
        if (attempt < 2 && (msg.includes("500") || msg.includes("529") || msg.includes("overloaded"))) {
          const delay = (attempt + 1) * 2000;
          yield { type: "text", content: `\n[retry ${attempt + 1}/3 in ${delay / 1000}s...]\n` };
          await new Promise((r) => setTimeout(r, delay));
          continue;
        }
        throw err;
      }
    }
    if (!stream) {
      yield { type: "error", content: "API unavailable after 3 retries" };
      return;
    }

    const assistantContent: Anthropic.ContentBlock[] = [];
    const toolUseBlocks: Array<{ id: string; name: string; input: unknown }> = [];

    try {
      for await (const event of stream) {
        if (event.type === "content_block_delta") {
          const delta = event.delta;
          if ("text" in delta && delta.text) {
            yield { type: "text", content: delta.text };
          }
        }
      }
    } catch (err) {
      const msg = (err as Error).message;
      if (msg.includes("500") || msg.includes("529") || msg.includes("overloaded")) {
        yield { type: "error", content: `API error: ${msg}. Try again.` };
        return;
      }
      throw err;
    }

    const finalMessage = await stream.finalMessage();

    for (const block of finalMessage.content) {
      assistantContent.push(block);
      if (block.type === "tool_use") {
        toolUseBlocks.push({ id: block.id, name: block.name, input: block.input });
      }
    }

    conversation.addAssistant(assistantContent);

    if (toolUseBlocks.length === 0) {
      yield { type: "done" };
      return;
    }

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

  yield { type: "error", content: "Max tool rounds exceeded" };
}
