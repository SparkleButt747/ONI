import { mkdirSync, writeFileSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { walkProject } from "../src/walker.js";

describe("walker", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = join(tmpdir(), `oni-walker-test-${Date.now()}`);
    mkdirSync(tmpDir, { recursive: true });
  });

  afterEach(() => {
    rmSync(tmpDir, { recursive: true, force: true });
  });

  it("T-WALK-1: walks a directory and finds indexed files", () => {
    writeFileSync(join(tmpDir, "index.ts"), "export const x = 1;");
    writeFileSync(join(tmpDir, "util.py"), "def foo(): pass");
    writeFileSync(join(tmpDir, "readme.md"), "# Hello");

    const files = walkProject(tmpDir);
    expect(files).toContain("index.ts");
    expect(files).toContain("util.py");
    expect(files).toContain("readme.md");
  });

  it("T-WALK-2: skips node_modules", () => {
    mkdirSync(join(tmpDir, "node_modules", "pkg"), { recursive: true });
    writeFileSync(join(tmpDir, "node_modules", "pkg", "index.js"), "module.exports = {}");
    writeFileSync(join(tmpDir, "app.ts"), "const x = 1;");

    const files = walkProject(tmpDir);
    expect(files).toEqual(["app.ts"]);
  });

  it("T-WALK-3: respects .gitignore patterns", () => {
    writeFileSync(join(tmpDir, ".gitignore"), "*.log\nsecrets/\n");
    writeFileSync(join(tmpDir, "app.ts"), "const x = 1;");
    writeFileSync(join(tmpDir, "debug.log"), "some log");
    mkdirSync(join(tmpDir, "secrets"), { recursive: true });
    writeFileSync(join(tmpDir, "secrets", "key.json"), "{}");

    const files = walkProject(tmpDir);
    expect(files).toEqual(["app.ts"]);
  });

  it("T-WALK-4: skips non-indexed extensions", () => {
    writeFileSync(join(tmpDir, "image.png"), "binary");
    writeFileSync(join(tmpDir, "data.csv"), "a,b,c");
    writeFileSync(join(tmpDir, "app.ts"), "const x = 1;");

    const files = walkProject(tmpDir);
    expect(files).toEqual(["app.ts"]);
  });

  it("T-WALK-5: walks nested directories", () => {
    mkdirSync(join(tmpDir, "src", "utils"), { recursive: true });
    writeFileSync(join(tmpDir, "src", "index.ts"), "export {}");
    writeFileSync(join(tmpDir, "src", "utils", "helpers.ts"), "export {}");

    const files = walkProject(tmpDir);
    expect(files).toContain(join("src", "index.ts"));
    expect(files).toContain(join("src", "utils", "helpers.ts"));
  });
});
