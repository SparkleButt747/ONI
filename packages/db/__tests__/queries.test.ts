import { describe, test, expect, beforeEach, afterEach } from "vitest";
import { createDatabase, closeDatabase, type DB } from "../src/database.js";
import { createConversation, getConversation, getLatestConversation } from "../src/queries/conversations.js";
import { insertMessage, getMessages } from "../src/queries/messages.js";

describe("Queries", () => {
  let db: DB;

  beforeEach(() => {
    db = createDatabase(":memory:");
  });
  afterEach(() => {
    closeDatabase(db);
  });

  test("creates and retrieves a conversation", () => {
    const conv = createConversation(db, "/tmp/test-project");
    expect(conv.conv_id).toBeTruthy();
    const retrieved = getConversation(db, conv.conv_id);
    expect(retrieved?.project_dir).toBe("/tmp/test-project");
  });

  test("getLatestConversation returns most recent", () => {
    const older = createConversation(db, "/old");
    const newer = createConversation(db, "/new");
    // Force distinct timestamps so ordering is deterministic
    db.prepare("UPDATE conversations SET last_active = ? WHERE conv_id = ?").run(1000, older.conv_id);
    db.prepare("UPDATE conversations SET last_active = ? WHERE conv_id = ?").run(2000, newer.conv_id);
    const latest = getLatestConversation(db);
    expect(latest?.conv_id).toBe(newer.conv_id);
  });

  test("inserts and retrieves messages in order", () => {
    const conv = createConversation(db, "/tmp");
    insertMessage(db, conv.conv_id, "user", "hello");
    insertMessage(db, conv.conv_id, "assistant", "hi there");
    const msgs = getMessages(db, conv.conv_id);
    expect(msgs).toHaveLength(2);
    expect(msgs[0].role).toBe("user");
    expect(msgs[1].role).toBe("assistant");
  });
});
