import { getApiKey } from "./keychain.js";

export async function resolveApiKey(): Promise<string | null> {
  // 1. Environment variable (highest priority — for CI, scripting)
  const envKey = process.env.ANTHROPIC_API_KEY;
  if (envKey && envKey.length > 0) {
    return envKey;
  }

  // 2. OS keychain via keytar
  const keychainKey = await getApiKey();
  if (keychainKey) {
    return keychainKey;
  }

  return null;
}
