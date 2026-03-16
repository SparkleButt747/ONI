#!/usr/bin/env node
import { Command } from "commander";
import { loginCommand } from "./commands/login.js";
import { logoutCommand } from "./commands/logout.js";
import { chatCommand } from "./commands/chat.js";
import { askCommand } from "./commands/ask.js";
import { configCommand } from "./commands/config-cmd.js";
import { prefsCommand } from "./commands/prefs.js";
import { initCommand } from "./commands/init.js";

const program = new Command();

program
  .name("oni")
  .description("ONI — Onboard Neural Intelligence. Ship or die.")
  .version("0.1.0");

program.addCommand(loginCommand);
program.addCommand(logoutCommand);
program.addCommand(chatCommand);
program.addCommand(askCommand);
program.addCommand(configCommand);
program.addCommand(prefsCommand);
program.addCommand(initCommand);

program.parse();
