import { describe, test, expect, beforeEach, afterEach } from "vitest";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { writeFileTool } from "../../src/tools/write-file.js";
import type { PermissionSet } from "../../src/tools/types.js";

describe("write_file tool", () => {
  let dir: string;

  beforeEach(() => { dir = mkdtempSync(join(tmpdir(), "oni-test-")); });
  afterEach(() => { rmSync(dir, { recursive: true }); });

  test("denied without --write flag", async () => {
    const perms: PermissionSet = { allowWrite: false, allowExec: false, projectDir: dir };
    const result = await writeFileTool.execute({ path: join(dir, "out.txt"), content: "hello" }, perms);
    expect(result.isError).toBe(true);
    expect(result.output).toContain("--write");
  });

  test("writes file with --write flag", async () => {
    const perms: PermissionSet = { allowWrite: true, allowExec: false, projectDir: dir };
    const path = join(dir, "out.txt");
    const result = await writeFileTool.execute({ path, content: "hello" }, perms);
    expect(result.isError).toBeFalsy();
    expect(readFileSync(path, "utf-8")).toBe("hello");
  });

  test("blocks writes outside project", async () => {
    const perms: PermissionSet = { allowWrite: true, allowExec: false, projectDir: dir };
    const result = await writeFileTool.execute({ path: "/tmp/evil.txt", content: "bad" }, perms);
    expect(result.isError).toBe(true);
    expect(result.output).toContain("outside project");
  });
});
