import { describe, test, expect } from "vitest";
import { bashTool } from "../../src/tools/bash.js";
import type { PermissionSet } from "../../src/tools/types.js";

describe("bash tool", () => {
  const permsNo: PermissionSet = { allowWrite: false, allowExec: false, projectDir: "/tmp" };
  const permsYes: PermissionSet = { allowWrite: false, allowExec: true, projectDir: "/tmp" };

  test("denied without --exec flag", async () => {
    const result = await bashTool.execute({ command: "echo hi" }, permsNo);
    expect(result.isError).toBe(true);
    expect(result.output).toContain("--exec");
  });

  test("runs command with --exec flag", async () => {
    const result = await bashTool.execute({ command: "echo hello" }, permsYes);
    expect(result.output.trim()).toBe("hello");
  });

  test("blocks dangerous commands", async () => {
    const result = await bashTool.execute({ command: "rm -rf /" }, permsYes);
    expect(result.isError).toBe(true);
    expect(result.output).toContain("Blocked");
  });
});
