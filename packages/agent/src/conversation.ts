import type Anthropic from "@anthropic-ai/sdk";

export type Message = Anthropic.MessageParam;
export type ContentBlock = Anthropic.ContentBlock;

export class Conversation {
  private messages: Message[] = [];

  addUser(content: string): void {
    this.messages.push({ role: "user", content });
  }

  addAssistant(content: ContentBlock[]): void {
    this.messages.push({ role: "assistant", content });
  }

  addToolResults(
    results: Array<{ tool_use_id: string; content: string; is_error?: boolean }>,
  ): void {
    this.messages.push({
      role: "user",
      content: results.map((r) => ({
        type: "tool_result" as const,
        tool_use_id: r.tool_use_id,
        content: r.content,
        is_error: r.is_error,
      })),
    });
  }

  getMessages(): Message[] {
    return [...this.messages];
  }

  get length(): number {
    return this.messages.length;
  }

  clear(): void {
    this.messages = [];
  }
}
