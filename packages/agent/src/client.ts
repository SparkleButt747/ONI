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
    const stream = client.messages.stream({
      model: config.model,
      max_tokens: 8192,
      system: systemPrompt,
      messages: conversation.getMessages(),
      tools,
    });

    const assistantContent: Anthropic.ContentBlock[] = [];
    const toolUseBlocks: Array<{ id: string; name: string; input: unknown }> = [];

    for await (const event of stream) {
      if (event.type === "content_block_delta") {
        const delta = event.delta;
        if ("text" in delta && delta.text) {
          yield { type: "text", content: delta.text };
        }
      }
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
