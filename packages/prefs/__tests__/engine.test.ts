import { describe, test, expect, beforeEach, afterEach } from "vitest";
import Database from "better-sqlite3";
import { PreferenceEngine } from "../src/engine.js";

describe("PreferenceEngine", () => {
  let db: Database.Database;
  let engine: PreferenceEngine;

  beforeEach(() => {
    db = new Database(":memory:");
    engine = new PreferenceEngine(db);
  });

  afterEach(() => {
    db.close();
  });

  test("default score is 0.5", () => {
    expect(engine.score("bash", "debug")).toBe(0.5);
  });

  test("accept signal increases score", () => {
    engine.record({
      sessionId: "s1",
      toolName: "bash",
      intentKey: "debug",
      outcome: "accepted",
    });
    expect(engine.score("bash", "debug")).toBeGreaterThan(0.5);
  });

  test("reject signal decreases score", () => {
    // Start with an accept to establish a baseline > 0.5
    engine.record({ sessionId: "s1", toolName: "bash", intentKey: "debug", outcome: "accepted" });
    const before = engine.score("bash", "debug");
    engine.record({ sessionId: "s1", toolName: "bash", intentKey: "debug", outcome: "rejected" });
    expect(engine.score("bash", "debug")).toBeLessThan(before);
  });

  test("always sets weight to 1.0", () => {
    engine.record({
      sessionId: "s1",
      toolName: "read_file",
      intentKey: "explore",
      outcome: "always",
    });
    expect(engine.score("read_file", "explore")).toBeCloseTo(1.0, 1);
  });

  test("decision returns auto for high scores", () => {
    engine.record({ sessionId: "s1", toolName: "bash", intentKey: "debug", outcome: "always" });
    expect(engine.decision("bash", "debug")).toBe("auto");
  });

  test("decision returns propose for default scores", () => {
    expect(engine.decision("unknown_tool", "unknown_intent")).toBe("propose");
  });

  test("crystallise creates rules for high-confidence tools", () => {
    // Record 10+ accept events
    for (let i = 0; i < 12; i++) {
      engine.record({
        sessionId: `s${i}`,
        toolName: "bash",
        intentKey: "debug",
        outcome: "accepted",
      });
    }
    const rules = engine.crystallise();
    expect(rules.length).toBeGreaterThanOrEqual(1);
    expect(rules[0].action).toContain("bash");
  });

  test("reset clears preferences", () => {
    engine.record({ sessionId: "s1", toolName: "bash", intentKey: "debug", outcome: "accepted" });
    engine.reset("bash");
    expect(engine.score("bash", "debug")).toBe(0.5);
  });

  test("stats returns counts", () => {
    engine.record({ sessionId: "s1", toolName: "bash", intentKey: "debug", outcome: "accepted" });
    const s = engine.stats();
    expect(s.totalPrefs).toBe(1);
    expect(s.totalEvents).toBe(1);
  });
});
