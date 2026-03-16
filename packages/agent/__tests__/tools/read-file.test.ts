import { describe, test, expect, beforeAll, afterAll } from "vitest";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { readFileTool } from "../../src/tools/read-file.js";
import type { PermissionSet } from "../../src/tools/types.js";

describe("read_file tool", () => {
  let dir: string;
  let perms: PermissionSet;

  beforeAll(() => {
    dir = mkdtempSync(join(tmpdir(), "oni-test-"));
    writeFileSync(join(dir, "test.ts"), "const x = 1;\nconst y = 2;\n");
    perms = { allowWrite: false, allowExec: false, projectDir: dir };
  });

  afterAll(() => { rmSync(dir, { recursive: true }); });

  test("reads file content with line numbers", async () => {
    const result = await readFileTool.execute({ path: join(dir, "test.ts") }, perms);
    expect(result.output).toContain("const x = 1");
    expect(result.output).toContain("1 |");
    expect(result.isError).toBeFalsy();
  });

  test("returns error for missing file", async () => {
    const result = await readFileTool.execute({ path: join(dir, "nope.ts") }, perms);
    expect(result.isError).toBe(true);
  });
});
