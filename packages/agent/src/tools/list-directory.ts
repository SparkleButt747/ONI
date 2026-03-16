import { readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { z } from "zod";
import type { ToolDefinition } from "./types.js";

const schema = z.object({
  path: z.string().describe("Directory path to list"),
});

export const listDirectoryTool: ToolDefinition = {
  name: "list_directory",
  description: "List files and directories in a given path. Shows type and size.",
  inputSchema: schema,
  async execute(args) {
    const { path } = schema.parse(args);
    try {
      const entries = readdirSync(path);
      const lines = entries.map((name) => {
        try {
          const stat = statSync(join(path, name));
          const type = stat.isDirectory() ? "dir " : "file";
          const size = stat.isDirectory() ? "" : ` ${stat.size}b`;
          return `  ${type}  ${name}${size}`;
        } catch {
          return `  ???   ${name}`;
        }
      });
      return { output: `${path}/\n${lines.join("\n")}` };
    } catch (err) {
      return { output: `Error listing directory: ${(err as Error).message}`, isError: true };
    }
  },
};
