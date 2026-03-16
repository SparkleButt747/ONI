import { describe, test, expect } from "vitest";
import { loadConfig } from "../src/config.js";

describe("Config", () => {
  test("returns defaults when no config files exist", () => {
    const config = loadConfig("/nonexistent/path");
    expect(config.model).toBe("claude-sonnet-4-6");
    expect(config.dryRunDefault).toBe(true);
    expect(config.colors).toBe(true);
  });
});
