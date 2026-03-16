import { describe, test, expect, beforeAll, afterAll } from "vitest";
import { mkdtempSync, writeFileSync, mkdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { listDirectoryTool } from "../../src/tools/list-directory.js";
import type { PermissionSet } from "../../src/tools/types.js";

describe("list_directory tool", () => {
  let dir: string;
  const perms: PermissionSet = { allowWrite: false, allowExec: false, projectDir: "/tmp" };

  beforeAll(() => {
    dir = mkdtempSync(join(tmpdir(), "oni-test-"));
    writeFileSync(join(dir, "file.ts"), "code");
    mkdirSync(join(dir, "subdir"));
  });

  afterAll(() => { rmSync(dir, { recursive: true }); });

  test("lists files and directories", async () => {
    const result = await listDirectoryTool.execute({ path: dir }, perms);
    expect(result.output).toContain("file.ts");
    expect(result.output).toContain("subdir");
    expect(result.output).toContain("dir");
  });

  test("returns error for missing dir", async () => {
    const result = await listDirectoryTool.execute({ path: "/nonexistent/path" }, perms);
    expect(result.isError).toBe(true);
  });
});
