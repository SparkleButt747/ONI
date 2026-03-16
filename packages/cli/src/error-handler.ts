import chalk from "chalk";

const DEBUG = process.env.ONI_DEBUG === "1";

interface AnthropicApiError {
  status?: number;
  error?: { type?: string; message?: string };
  message?: string;
}

function isAnthropicApiError(err: unknown): err is AnthropicApiError {
  return (
    typeof err === "object" &&
    err !== null &&
    "status" in err &&
    typeof (err as AnthropicApiError).status === "number"
  );
}

function formatApiError(err: AnthropicApiError): string {
  const status = err.status ?? 0;
  const msg = err.error?.message ?? err.message ?? "Unknown API error";

  switch (status) {
    case 401:
      return "Invalid API key. Run `oni login` to re-authenticate.";
    case 403:
      return "API key lacks permission for this request.";
    case 429:
      return "Rate limited. Wait a moment and try again.";
    case 500:
    case 502:
    case 503:
      return `Anthropic API is having issues (HTTP ${status}). Try again shortly.`;
    default:
      return `API error (HTTP ${status}): ${msg}`;
  }
}

function formatError(err: unknown): string {
  if (isAnthropicApiError(err)) {
    return formatApiError(err);
  }

  const error = err as Error;
  const msg = error.message ?? String(err);

  // Keychain errors
  if (msg.includes("keytar") || msg.includes("keychain") || msg.includes("secret_service")) {
    return "Keychain unavailable. Store your key via ANTHROPIC_API_KEY env var instead.";
  }

  // SQLite errors
  if (msg.includes("SQLITE") || msg.includes("better-sqlite3") || msg.includes("database")) {
    if (msg.includes("SQLITE_READONLY") || msg.includes("readonly")) {
      return "Database is read-only. Check file permissions on ~/.local/share/oni/oni.db";
    }
    if (msg.includes("SQLITE_CORRUPT")) {
      return "Database is corrupt. Delete ~/.local/share/oni/oni.db and retry.";
    }
    return `Database error: ${msg}`;
  }

  // Module not found
  if (msg.includes("Cannot find module") || msg.includes("MODULE_NOT_FOUND")) {
    return `Missing dependency: ${msg}. Try running \`npm install\` in the ONI root.`;
  }

  return msg;
}

export function handleTopLevelError(err: unknown): never {
  const message = formatError(err);
  console.error(chalk.red(`error: ${message}`));

  if (DEBUG && err instanceof Error && err.stack) {
    console.error(chalk.dim(err.stack));
  }

  process.exit(1);
}
