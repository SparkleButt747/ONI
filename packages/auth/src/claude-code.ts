import { execSync } from "node:child_process";
import { platform, userInfo } from "node:os";

/**
 * Reads Claude Code's stored OAuth token from the OS keychain.
 * This is for personal use only — the user already owns the Claude Code session.
 *
 * Claude Code stores credentials in the macOS keychain under:
 *   service: "Claude Code-credentials"
 *   account: <username>
 *   value: JSON with { claudeAiOauth: { accessToken: "sk-ant-oat01-..." } }
 */

interface ClaudeCodeKeychainData {
  claudeAiOauth?: {
    accessToken?: string;
    refreshToken?: string;
  };
}

function readFromMacOSKeychain(): string | null {
  try {
    const username = userInfo().username;
    const raw = execSync(
      `security find-generic-password -s "Claude Code-credentials" -a "${username}" -w`,
      { encoding: "utf-8", timeout: 5000, stdio: ["pipe", "pipe", "pipe"] },
    ).trim();

    const data = JSON.parse(raw) as ClaudeCodeKeychainData;
    const token = data.claudeAiOauth?.accessToken;
    if (token && token.length > 0) {
      return token;
    }
  } catch {
    // keychain entry not found or parse error
  }

  // Try without account name (some installs use different account)
  try {
    const raw = execSync(
      'security find-generic-password -s "Claude Code-credentials" -w',
      { encoding: "utf-8", timeout: 5000, stdio: ["pipe", "pipe", "pipe"] },
    ).trim();

    const data = JSON.parse(raw) as ClaudeCodeKeychainData;
    const token = data.claudeAiOauth?.accessToken;
    if (token && token.length > 0) {
      return token;
    }
  } catch {
    // not found
  }

  return null;
}

function readFromLinuxKeyring(): string | null {
  // Linux: Claude Code uses libsecret
  try {
    const raw = execSync(
      'secret-tool lookup service "Claude Code-credentials"',
      { encoding: "utf-8", timeout: 5000, stdio: ["pipe", "pipe", "pipe"] },
    ).trim();

    const data = JSON.parse(raw) as ClaudeCodeKeychainData;
    return data.claudeAiOauth?.accessToken ?? null;
  } catch {
    return null;
  }
}

export function findClaudeCodeToken(): string | null {
  const os = platform();

  if (os === "darwin") {
    return readFromMacOSKeychain();
  }

  if (os === "linux") {
    return readFromLinuxKeyring();
  }

  // Windows: not yet supported for Claude Code passthrough
  return null;
}

export function hasClaudeCode(): boolean {
  return findClaudeCodeToken() !== null;
}
