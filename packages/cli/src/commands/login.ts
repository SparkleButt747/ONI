import { Command } from "commander";
import chalk from "chalk";

export const loginCommand = new Command("login")
  .description("Authenticate with your Anthropic API key")
  .option("--key <key>", "API key (alternatively prompted interactively)")
  .action(async (options) => {
    // Dynamic imports to avoid loading heavy deps on --help
    const { storeApiKey, hasApiKey, validateApiKey } = await import("@oni/auth");

    const already = await hasApiKey();
    if (already && !options.key) {
      console.log(chalk.hex("#e8c547")("Already authenticated. Re-run with --key to update."));
      return;
    }

    let key = options.key as string | undefined;

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

    console.log(chalk.hex("#5a5855")("Validating..."));
    const result = await validateApiKey(key);

    if (!result.valid) {
      console.error(chalk.hex("#ff4d2e")(result.error ?? "Invalid key."));
      process.exit(1);
    }

    await storeApiKey(key);
    console.log(chalk.hex("#f5a623")("Authenticated.") + " Key stored in OS keychain.");
  });
