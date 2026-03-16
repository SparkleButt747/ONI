import type Anthropic from "@anthropic-ai/sdk";
import { zodToJsonSchema } from "zod-to-json-schema";
import { readFileTool } from "./read-file.js";
import { writeFileTool } from "./write-file.js";
import { bashTool } from "./bash.js";
import { listDirectoryTool } from "./list-directory.js";
import type { ToolDefinition, PermissionSet, ToolResult } from "./types.js";

const TOOLS: ToolDefinition[] = [readFileTool, writeFileTool, bashTool, listDirectoryTool];

export function getAllToolSchemas(): Anthropic.Tool[] {
  return TOOLS.map((t) => ({
    name: t.name,
    description: t.description,
    input_schema: zodToJsonSchema(t.inputSchema) as Anthropic.Tool["input_schema"],
  }));
}

export async function executeTool(
  name: string,
  args: unknown,
  permissions: PermissionSet,
): Promise<ToolResult> {
  const tool = TOOLS.find((t) => t.name === name);
  if (!tool) {
    return { output: `Unknown tool: ${name}`, isError: true };
  }
  return tool.execute(args, permissions);
}

export type { ToolDefinition, PermissionSet, ToolResult };
