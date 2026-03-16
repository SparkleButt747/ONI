import { mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { initIndex, updateIndex } from "../src/indexer.js";

describe("indexer", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = join(tmpdir(), `oni-indexer-test-${Date.now()}`);
    mkdirSync(tmpDir, { recursive: true });
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("T-IDX-1: indexes a project and reports stats", () => {
    writeFileSync(
      join(tmpDir, "index.ts"),
      `export function greet(name: string): string {
  return "hello " + name;
}

export class App {
  run() {}
}
`,
    );
    writeFileSync(join(tmpDir, "util.py"), "def helper():\n    pass\n");

    const result = initIndex(tmpDir);
    expect(result.filesIndexed).toBe(2);
    expect(result.totalFiles).toBe(2);
    expect(result.symbolsFound).toBeGreaterThan(0);
    expect(result.totalTokens).toBeGreaterThan(0);
    expect(result.elapsedMs).toBeGreaterThanOrEqual(0);
  });

  it("T-IDX-2: re-index skips unchanged files", () => {
    writeFileSync(join(tmpDir, "app.ts"), "export const x = 1;");

    const first = initIndex(tmpDir);
    expect(first.filesIndexed).toBe(1);

    const second = updateIndex(tmpDir);
    expect(second.filesSkipped).toBe(1);
    expect(second.filesIndexed).toBe(0);
  });

  it("T-IDX-3: re-index detects changed files", () => {
    writeFileSync(join(tmpDir, "app.ts"), "export const x = 1;");
    initIndex(tmpDir);

    writeFileSync(join(tmpDir, "app.ts"), "export const x = 2; // changed");
    const result = updateIndex(tmpDir);
    expect(result.filesIndexed).toBe(1);
    expect(result.filesSkipped).toBe(0);
  });

  it("T-IDX-4: removes stale files from index", () => {
    writeFileSync(join(tmpDir, "a.ts"), "export const a = 1;");
    writeFileSync(join(tmpDir, "b.ts"), "export const b = 2;");
    const first = initIndex(tmpDir);
    expect(first.totalFiles).toBe(2);

    rmSync(join(tmpDir, "b.ts"));
    const second = updateIndex(tmpDir);
    expect(second.filesRemoved).toBe(1);
    expect(second.totalFiles).toBe(1);
  });
});
