import { mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { initIndex } from "../src/indexer.js";
import { queryContext } from "../src/retriever.js";

describe("retriever", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = join(tmpdir(), `oni-retriever-test-${Date.now()}`);
    mkdirSync(tmpDir, { recursive: true });

    // Create some test files
    writeFileSync(
      join(tmpDir, "database.ts"),
      `export class Database {
  constructor(private path: string) {}
  query(sql: string) { return []; }
  close() {}
}
`,
    );
    writeFileSync(
      join(tmpDir, "auth.ts"),
      `export function authenticate(token: string): boolean {
  return token.length > 0;
}

export function logout(): void {
  // clear session
}
`,
    );
    writeFileSync(
      join(tmpDir, "config.ts"),
      `export interface AppConfig {
  debug: boolean;
  port: number;
  database: string;
}

export const DEFAULT_CONFIG: AppConfig = {
  debug: false,
  port: 3000,
  database: "app.db",
};
`,
    );

    initIndex(tmpDir);
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("T-RET-1: finds relevant file for a query", () => {
    const result = queryContext(tmpDir, "database query sql");
    expect(result.chunks.length).toBeGreaterThan(0);
    expect(result.chunks[0].path).toBe("database.ts");
  });

  it("T-RET-2: respects token budget", () => {
    // Very small budget
    const result = queryContext(tmpDir, "database", 50);
    expect(result.totalTokens).toBeLessThanOrEqual(50);
  });

  it("T-RET-3: returns scored results", () => {
    const result = queryContext(tmpDir, "authenticate token session");
    expect(result.chunks.length).toBeGreaterThan(0);
    for (const chunk of result.chunks) {
      expect(chunk.score).toBeGreaterThan(0);
      expect(chunk.tokenEstimate).toBeGreaterThan(0);
    }
  });

  it("T-RET-4: returns empty for nonsense query", () => {
    const result = queryContext(tmpDir, "xyzzy_nonexistent_foobar");
    expect(result.chunks).toHaveLength(0);
  });

  it("T-RET-5: returns retrieval time", () => {
    const result = queryContext(tmpDir, "config");
    expect(result.retrievalMs).toBeGreaterThanOrEqual(0);
  });

  it("T-RET-6: handles missing index gracefully", () => {
    const noIndexDir = join(tmpdir(), `oni-no-index-${Date.now()}`);
    mkdirSync(noIndexDir, { recursive: true });
    const result = queryContext(noIndexDir, "anything");
    expect(result.chunks).toHaveLength(0);
    rmSync(noIndexDir, { recursive: true, force: true });
  });
});
