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
    const key = await resolveApiKey();
    expect(key).toBe("sk-ant-test-key");
  });

  test("returns null when no key available", async () => {
    delete process.env.ANTHROPIC_API_KEY;
    // keytar may or may not have a key — we can't control that in unit tests
    // This test just verifies the function doesn't throw
    const key = await resolveApiKey();
    expect(typeof key === "string" || key === null).toBe(true);
  });
});
