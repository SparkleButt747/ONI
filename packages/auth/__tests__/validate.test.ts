import { describe, test, expect } from "vitest";
import { validateApiKey } from "../src/validate.js";

describe("validateApiKey", () => {
  test("rejects empty key", async () => {
    const result = await validateApiKey("");
    expect(result.valid).toBe(false);
  });

  test("rejects malformed key", async () => {
    const result = await validateApiKey("not-a-real-key");
    expect(result.valid).toBe(false);
    expect(result.error).toContain("sk-ant-");
  });

  // Note: can't test real validation without a real API key
  // That's tested in integration/E2E
});
