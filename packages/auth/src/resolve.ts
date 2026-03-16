import { getApiKey } from "./keychain.js";
import { findClaudeCodeToken } from "./claude-code.js";

export type KeySource = "env" | "keychain" | "claude-code";

export interface ResolvedKey {
  key: string;
  source: KeySource;
}

export async function resolveApiKey(): Promise<ResolvedKey | null> {
  // 1. Environment variable (highest priority — for CI, scripting)
  const envKey = process.env.ANTHROPIC_API_KEY;
  if (envKey && envKey.length > 0) {
    return { key: envKey, source: "env" };
  }

  // 2. OS keychain via keytar
  const keychainKey = await getApiKey();
  if (keychainKey) {
    return { key: keychainKey, source: "keychain" };
  }

  // 3. Claude Code token passthrough (personal use)
  const ccToken = findClaudeCodeToken();
  if (ccToken) {
    return { key: ccToken, source: "claude-code" };
  }

  return null;
}
