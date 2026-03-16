import { Command } from "commander";
import chalk from "chalk";

export const loginCommand = new Command("login")
  .description("Authenticate with your Anthropic API key")
  .option("--key <key>", "API key (alternatively prompted interactively)")
  .option("--from-claude-code", "Read token from Claude Code credentials")
  .addHelpText(
    "after",
    `
Examples:
  $ oni login                       Interactive prompt for API key
  $ oni login --key sk-ant-...      Provide key directly
  $ oni login --from-claude-code    Reuse Claude Code credentials
`,
  )
  .action(async (options) => {
    // Dynamic imports to avoid loading heavy deps on --help
    const { storeApiKey, hasApiKey, validateApiKey, findClaudeCodeToken } =
      await import("@oni/auth");

    const already = await hasApiKey();
    if (already && !options.key && !options.fromClaudeCode) {
      console.log(chalk.hex("#e8c547")("Already authenticated. Re-run with --key to update."));
      return;
    }

    let key = options.key as string | undefined;

    let isClaudeCode = false;

    if (options.fromClaudeCode) {
      const ccToken = findClaudeCodeToken();
      if (!ccToken) {
        console.error(
          chalk.hex("#ff4d2e")(
            "No Claude Code credentials found. Is Claude Code installed and logged in?",
          ),
        );
        process.exit(1);
      }
      key = ccToken;
      isClaudeCode = true;
      console.log(chalk.hex("#00d4c8")("Found Claude Code token."));
    }

    if (!key) {
      // Read from stdin interactively
      const readline = await import("node:readline");
      const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
      key = await new Promise<string>((resolve) => {
        rl.question(chalk.hex("#f5a623")("API key (from platform.anthropic.com): "), (answer) => {
          rl.close();
          resolve(answer.trim());
        });
      });
    }

    if (!key) {
      console.error(chalk.hex("#ff4d2e")("No key provided."));
      process.exit(1);
    }

    // Skip API validation for Claude Code OAuth tokens — they use a different auth path
    if (!isClaudeCode) {
      console.log(chalk.hex("#5a5855")("Validating..."));
      const result = await validateApiKey(key);

      if (!result.valid) {
        console.error(chalk.hex("#ff4d2e")(result.error ?? "Invalid key."));
        process.exit(1);
      }
    }

    await storeApiKey(key);
    const source = isClaudeCode ? "Claude Code token" : "API key";
    console.log(chalk.hex("#f5a623")("Authenticated.") + ` ${source} stored in OS keychain.`);
  });
