import { readFileSync } from "node:fs";
import { z } from "zod";
import type { ToolDefinition } from "./types.js";

const schema = z.object({
  path: z.string().describe("Absolute or relative path to the file to read"),
});

export const readFileTool: ToolDefinition = {
  name: "read_file",
  description: "Read the contents of a file. Returns file content with line numbers.",
  inputSchema: schema,
  async execute(args) {
    const { path } = schema.parse(args);
    try {
      const content = readFileSync(path, "utf-8");
      const numbered = content
        .split("\n")
        .map((line, i) => `${String(i + 1).padStart(4)} | ${line}`)
        .join("\n");
      return { output: numbered };
    } catch (err) {
      return { output: `Error reading file: ${(err as Error).message}`, isError: true };
    }
  },
};
