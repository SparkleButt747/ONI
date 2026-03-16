import { describe, test, expect } from "vitest";
import { Conversation } from "../src/conversation.js";

describe("Conversation", () => {
  test("adds user and assistant messages", () => {
    const conv = new Conversation();
    conv.addUser("hello");
    conv.addAssistant([{ type: "text", text: "hi" }]);
    expect(conv.getMessages()).toHaveLength(2);
    expect(conv.getMessages()[0].role).toBe("user");
  });

  test("adds tool results as user message", () => {
    const conv = new Conversation();
    conv.addToolResults([{ tool_use_id: "123", content: "file contents" }]);
    expect(conv.getMessages()).toHaveLength(1);
    expect(conv.getMessages()[0].role).toBe("user");
  });

  test("clear resets messages", () => {
    const conv = new Conversation();
    conv.addUser("hello");
    conv.clear();
    expect(conv.length).toBe(0);
  });
});
