import { Command } from "commander";
import chalk from "chalk";
import { setConfigValue, getConfigValue } from "../config.js";

export const configCommand = new Command("config")
  .description("Get or set ONI configuration");

configCommand
  .command("set <key> <value>")
  .description("Set a config value")
  .action((key: string, value: string) => {
    setConfigValue(key, value);
    console.log(chalk.hex("#5a5855")(`${key} = ${value}`));
  });

configCommand
  .command("get <key>")
  .description("Get a config value")
  .action((key: string) => {
    const value = getConfigValue(key);
    if (value === undefined) {
      console.log(chalk.hex("#5a5855")(`${key}: (not set)`));
    } else {
      console.log(`${key} = ${String(value)}`);
    }
  });
