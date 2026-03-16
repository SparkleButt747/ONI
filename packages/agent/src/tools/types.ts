import type { z } from "zod";

export interface ToolDefinition {
  name: string;
  description: string;
  inputSchema: z.ZodType;
  execute: (args: unknown, permissions: PermissionSet) => Promise<ToolResult>;
}

export interface ToolResult {
  output: string;
  isError?: boolean;
}

export interface PermissionSet {
  allowWrite: boolean;
  allowExec: boolean;
  projectDir: string;
}
