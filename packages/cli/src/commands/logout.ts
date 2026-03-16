import { Command } from "commander";
import chalk from "chalk";

export const logoutCommand = new Command("logout")
  .description("Remove stored API key from OS keychain")
  .action(async () => {
    const { deleteApiKey, hasApiKey } = await import("@oni/auth");

    const exists = await hasApiKey();
    if (!exists) {
      console.log(chalk.hex("#5a5855")("No stored key found."));
      return;
    }

    await deleteApiKey();
    console.log(chalk.hex("#f5a623")("Logged out.") + " Key removed from OS keychain.");
  });
