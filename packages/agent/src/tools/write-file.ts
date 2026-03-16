import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";
import { z } from "zod";
import type { ToolDefinition, PermissionSet } from "./types.js";
import { isPathInProject } from "../permissions.js";

const schema = z.object({
  path: z.string().describe("Absolute or relative path to write to"),
  content: z.string().describe("Content to write to the file"),
});

export const writeFileTool: ToolDefinition = {
  name: "write_file",
  description: "Write content to a file. Creates parent directories if needed. Requires --write flag.",
  inputSchema: schema,
  async execute(args, permissions) {
    const { path, content } = schema.parse(args);

    if (!permissions.allowWrite) {
      return { output: "Permission denied: file writes require --write flag. Run `oni chat --write` to enable.", isError: true };
    }

    if (!isPathInProject(path, permissions.projectDir)) {
      return { output: `Permission denied: cannot write outside project directory (${permissions.projectDir})`, isError: true };
    }

    try {
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, content);
      return { output: `Written to ${path} (${content.length} bytes)` };
    } catch (err) {
      return { output: `Error writing file: ${(err as Error).message}`, isError: true };
    }
  },
};
