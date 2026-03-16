import { readFileSync, existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

/**
 * Attempts to read Claude Code's stored OAuth token.
 * This is for personal use only — the user already owns the Claude Code session.
 * ONI reads the same developer's own credentials from their own tool.
 */

const CREDENTIAL_PATHS = [
  // Claude Code stores credentials in various locations depending on platform/version
  join(homedir(), ".claude", "credentials.json"),
  join(homedir(), ".config", "claude", "credentials.json"),
  join(homedir(), ".claude.json"),
];

interface ClaudeCodeCredentials {
  oauth_token?: string;
  access_token?: string;
  token?: string;
}

export function findClaudeCodeToken(): string | null {
  for (const path of CREDENTIAL_PATHS) {
    try {
      if (!existsSync(path)) continue;
      const raw = readFileSync(path, "utf-8");
      const data = JSON.parse(raw) as ClaudeCodeCredentials;
      const token = data.oauth_token ?? data.access_token ?? data.token;
      if (token && token.length > 0) {
        return token;
      }
    } catch {
      // skip malformed files
    }
  }
  return null;
}

export function hasClaudeCode(): boolean {
  return findClaudeCodeToken() !== null;
}
