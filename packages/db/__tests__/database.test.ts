import { describe, test, expect, afterEach } from "vitest";
import { createDatabase, closeDatabase, type DB } from "../src/database.js";

describe("Database", () => {
  let db: DB;

  afterEach(() => {
    if (db) closeDatabase(db);
  });

  test("creates in-memory database with tables", () => {
    db = createDatabase(":memory:");
    const tables = db
      .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
      .all() as { name: string }[];
    const names = tables.map((t) => t.name);
    expect(names).toContain("conversations");
    expect(names).toContain("messages");
    expect(names).toContain("tool_events");
  });

  test("migrations are idempotent", () => {
    db = createDatabase(":memory:");
    expect(() => createDatabase(":memory:")).not.toThrow();
  });
});
