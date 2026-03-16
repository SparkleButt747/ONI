import Anthropic from "@anthropic-ai/sdk";

export interface ValidationResult {
  valid: boolean;
  error?: string;
}

export async function validateApiKey(apiKey: string): Promise<ValidationResult> {
  if (!apiKey || !apiKey.startsWith("sk-ant-")) {
    return { valid: false, error: "Invalid key format. API keys start with 'sk-ant-'" };
  }

  try {
    const client = new Anthropic({ apiKey });
    // Lightweight probe — minimal tokens
    await client.messages.create({
      model: "claude-haiku-4-5-20251001",
      max_tokens: 1,
      messages: [{ role: "user", content: "ping" }],
    });
    return { valid: true };
  } catch (err) {
    const msg = (err as Error).message;
    if (msg.includes("401") || msg.includes("authentication")) {
      return { valid: false, error: "Invalid API key. Check your key at platform.anthropic.com" };
    }
    if (msg.includes("429")) {
      // Rate limited but key is valid
      return { valid: true };
    }
    return { valid: false, error: `Validation failed: ${msg}` };
  }
}
