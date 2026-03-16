import { execSync } from "node:child_process";
import { z } from "zod";
import type { ToolDefinition } from "./types.js";

const schema = z.object({
  command: z.string().describe("The bash command to execute"),
  timeout: z.number().optional().describe("Timeout in milliseconds (default 30000)"),
});

const BLOCKED_PATTERNS = [/rm\s+-rf\s+\//, /rm\s+-rf\s+~/, /mkfs/, /dd\s+if=/];

export const bashTool: ToolDefinition = {
  name: "bash",
  description: "Execute a bash command and return stdout + stderr. Requires --exec flag.",
  inputSchema: schema,
  async execute(args, permissions) {
    const { command, timeout } = schema.parse(args);

    if (!permissions.allowExec) {
      return { output: "Permission denied: bash execution requires --exec flag. Run `oni chat --exec` to enable.", isError: true };
    }

    for (const pattern of BLOCKED_PATTERNS) {
      if (pattern.test(command)) {
        return { output: `Blocked: dangerous command pattern detected in "${command}"`, isError: true };
      }
    }

    try {
      const result = execSync(command, {
        cwd: permissions.projectDir,
        timeout: timeout ?? 30_000,
        encoding: "utf-8",
        maxBuffer: 1024 * 1024,
        stdio: ["pipe", "pipe", "pipe"],
      });
      return { output: result || "(no output)" };
    } catch (err: unknown) {
      const execErr = err as { stdout?: string; stderr?: string; message: string };
      const output = [execErr.stdout, execErr.stderr].filter(Boolean).join("\n") || execErr.message;
      return { output, isError: true };
    }
  },
};
