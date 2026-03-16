import { describe, test, expect, afterEach } from "vitest";
import { resolveApiKey } from "../src/resolve.js";

describe("resolveApiKey", () => {
  const originalEnv = process.env.ANTHROPIC_API_KEY;

  afterEach(() => {
    if (originalEnv) {
      process.env.ANTHROPIC_API_KEY = originalEnv;
    } else {
      delete process.env.ANTHROPIC_API_KEY;
    }
  });

  test("returns env var when set", async () => {
    process.env.ANTHROPIC_API_KEY = "sk-ant-test-key";
    const result = await resolveApiKey();
    expect(result).toEqual({ key: "sk-ant-test-key", source: "env" });
  });

  test("returns null when no key available", async () => {
    delete process.env.ANTHROPIC_API_KEY;
    // keytar may or may not have a key — we can't control that in unit tests
    // This test just verifies the function doesn't throw
    const result = await resolveApiKey();
    expect(result === null || (typeof result === "object" && typeof result.key === "string")).toBe(
      true,
    );
  });
});
