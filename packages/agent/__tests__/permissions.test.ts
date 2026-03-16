import { describe, test, expect } from "vitest";
import { isPathInProject, createPermissions } from "../src/permissions.js";

describe("Permissions", () => {
  test("path inside project returns true", () => {
    expect(isPathInProject("/project/src/file.ts", "/project")).toBe(true);
  });

  test("path outside project returns false", () => {
    expect(isPathInProject("/etc/passwd", "/project")).toBe(false);
  });

  test("relative path traversal returns false", () => {
    expect(isPathInProject("/project/../etc/passwd", "/project")).toBe(false);
  });

  test("createPermissions defaults write/exec to false", () => {
    const p = createPermissions({ projectDir: "/tmp" });
    expect(p.allowWrite).toBe(false);
    expect(p.allowExec).toBe(false);
  });
});
