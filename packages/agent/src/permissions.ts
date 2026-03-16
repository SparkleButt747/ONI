import { resolve, relative, isAbsolute } from "node:path";
import type { PermissionSet } from "./tools/types.js";

export function createPermissions(opts: {
  allowWrite?: boolean;
  allowExec?: boolean;
  projectDir: string;
}): PermissionSet {
  return {
    allowWrite: opts.allowWrite ?? false,
    allowExec: opts.allowExec ?? false,
    projectDir: resolve(opts.projectDir),
  };
}

export function isPathInProject(filePath: string, projectDir: string): boolean {
  const resolved = resolve(filePath);
  const rel = relative(projectDir, resolved);
  return !rel.startsWith("..") && !isAbsolute(rel);
}
