#!/usr/bin/env node
import { Command } from "commander";
import { handleTopLevelError } from "./error-handler.js";
import { loginCommand } from "./commands/login.js";
import { logoutCommand } from "./commands/logout.js";
import { chatCommand } from "./commands/chat.js";
import { askCommand } from "./commands/ask.js";
import { configCommand } from "./commands/config-cmd.js";
import { prefsCommand } from "./commands/prefs.js";
import { initCommand } from "./commands/init.js";
import { doctorCommand } from "./commands/doctor.js";

const program = new Command();

program
  .name("oni")
  .description("ONI \u2014 Onboard Neural Intelligence. Ship or die.")
  .version("0.1.0")
  .addHelpText(
    "after",
    `
Examples:
  $ oni login                       Authenticate with your API key
  $ oni login --from-claude-code    Use Claude Code credentials
  $ oni chat                        Start interactive chat (read-only)
  $ oni chat --write --exec         Chat with file write + bash access
  $ oni chat --agents               Multi-agent mode (Planner/Executor/Critic)
  $ oni ask "explain this file"     One-shot question
  $ cat file.ts | oni ask "review"  Pipe content for review
  $ oni init                        Index project for context retrieval
  $ oni config set model claude-sonnet-4-6
  $ oni doctor                      Check setup and diagnose issues
`,
  );

program.addCommand(loginCommand);
program.addCommand(logoutCommand);
program.addCommand(chatCommand);
program.addCommand(askCommand);
program.addCommand(configCommand);
program.addCommand(prefsCommand);
program.addCommand(initCommand);
program.addCommand(doctorCommand);

try {
  await program.parseAsync();
} catch (err) {
  handleTopLevelError(err);
}
