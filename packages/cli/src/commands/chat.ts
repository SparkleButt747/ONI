import { Command } from "commander";
import chalk from "chalk";
import { createInterface } from "node:readline/promises";
import { stdin, stdout } from "node:process";

export const chatCommand = new Command("chat")
  .description("Start an interactive chat session with ONI")
  .option("--write", "Allow file writes")
  .option("--exec", "Allow bash execution")
  .option("--model <model>", "Model override")
  .option("--budget <tokens>", "Max tokens for this session (e.g. 50000)")
  .option("--monthly-limit <tokens>", "Monthly token limit")
  .action(async (options) => {
    const { resolveApiKey } = await import("@oni/auth");
    const { runAgent } = await import("@oni/agent");
    const { Conversation } = await import("@oni/agent/conversation");
    const { createPermissions } = await import("@oni/agent/permissions");
    const { BudgetTracker } = await import("@oni/agent/budget");
    const { createDatabase } = await import("@oni/db");
    const { createConversation, insertMessage, touchConversation } = await import("@oni/db/queries");
    const { loadConfig, getDataDir } = await import("../config.js");
    const { join } = await import("node:path");

    const resolved = await resolveApiKey();
    if (!resolved) {
      console.error(chalk.hex("#ff4d2e")("No API key found. Run `oni login` or set ANTHROPIC_API_KEY."));
      process.exit(1);
    }

    const config = loadConfig(process.cwd());
    const model = (options.model as string) ?? config.model;
    const projectDir = process.cwd();

    const permissions = createPermissions({
      allowWrite: options.write as boolean,
      allowExec: options.exec as boolean,
      projectDir,
    });

    // Budget tracking
    const sessionLimit = options.budget ? Number(options.budget) : 0;
    const monthlyLimit = options.monthlyLimit
      ? Number(options.monthlyLimit)
      : config.monthlyTokenLimit ?? 0;
    const budget = new BudgetTracker({ sessionLimit, monthlyLimit });

    // Init DB
    const dbPath = join(getDataDir(), "oni.db");
    const db = createDatabase(dbPath);
    const conv = createConversation(db, projectDir);
    const conversation = new Conversation();

    // Header
    const c = {
      amber: chalk.hex("#f5a623"),
      cyan: chalk.hex("#00d4c8"),
      muted: chalk.hex("#5a5855"),
      coral: chalk.hex("#ff4d2e"),
      lime: chalk.hex("#b4e033"),
      white: chalk.hex("#f0ede6"),
      dim: chalk.hex("#323230"),
    };

    console.log();
    console.log(c.amber.bold("ONI") + c.muted(" ONBOARD NEURAL INTELLIGENCE"));
    console.log(c.muted(`v0.1.0 · ${model} · ${conv.conv_id.slice(0, 8)}`));
    console.log(c.dim("─".repeat(60)));

    const flags: string[] = [];
    if (options.write) flags.push(c.lime("--write"));
    if (options.exec) flags.push(c.lime("--exec"));
    if (flags.length === 0) flags.push(c.muted("read-only"));
    console.log(c.muted("permissions: ") + flags.join(" "));
    if (sessionLimit > 0) {
      console.log(c.muted(`budget: ${sessionLimit} tok/session`));
    }
    if (monthlyLimit > 0) {
      console.log(c.muted(`monthly limit: ${monthlyLimit} tok`));
    }
    console.log(c.dim("─".repeat(60)));
    console.log(c.muted(":q to quit"));
    console.log();

    const rl = createInterface({ input: stdin, output: stdout });

    while (true) {
      let userInput: string;
      try {
        userInput = await rl.question(c.amber("you › "));
      } catch {
        break; // EOF
      }

      const trimmed = userInput.trim();
      if (!trimmed) continue;
      if (trimmed === ":q" || trimmed === ":quit") break;

      // Save user message to DB
      insertMessage(db, conv.conv_id, "user", trimmed);
      touchConversation(db, conv.conv_id);

      // Stream agent response
      let fullResponse = "";
      try {
        const agentConfig = {
          model,
          apiKey: resolved.key,
          projectDir,
          permissions,
        };

        process.stdout.write("\n");

        for await (const event of runAgent(trimmed, conversation, agentConfig)) {
          switch (event.type) {
            case "text":
              process.stdout.write(event.content ?? "");
              fullResponse += event.content ?? "";
              break;
            case "tool_call":
              process.stdout.write(
                `\n${c.cyan("[tool]")} ${c.cyan.bold(event.tool ?? "")} ${c.muted(JSON.stringify(event.args ?? {}).slice(0, 80))}\n`,
              );
              break;
            case "tool_result":
              if (event.isError) {
                process.stdout.write(c.coral(`  error: ${(event.result ?? "").slice(0, 200)}\n`));
              } else {
                const preview = (event.result ?? "").split("\n").slice(0, 3).join("\n");
                process.stdout.write(c.dim(`  ${preview.slice(0, 200)}\n`));
              }
              break;
            case "error":
              process.stdout.write(c.coral(`\nerror: ${event.content}\n`));
              break;
            case "done":
              break;
          }
        }

        // Estimate tokens from response length as fallback (rough: 1 tok ~= 4 chars)
        if (fullResponse.length > 0) {
          const estimatedTokens = Math.ceil(fullResponse.length / 4);
          const withinBudget = budget.record(estimatedTokens);
          if (!withinBudget) {
            process.stdout.write(
              `\n${c.coral("Budget exceeded.")} ${c.muted(budget.summary().replace("\n", " | "))}\n`,
            );
            process.stdout.write("\n");
            break;
          }
        }

        process.stdout.write("\n\n");

        // Save assistant response to DB
        if (fullResponse) {
          insertMessage(db, conv.conv_id, "assistant", fullResponse);
        }
      } catch (err) {
        console.error(c.coral(`\nError: ${(err as Error).message}`));
      }
    }

    rl.close();
    db.close();
    console.log(c.muted("\nSession ended."));
  });
