import { Command } from "commander";
import chalk from "chalk";

export const askCommand = new Command("ask")
  .description("Ask ONI a question (pipe/stdin mode)")
  .argument("[question...]", "The question to ask")
  .option("--write", "Allow file writes")
  .option("--exec", "Allow bash execution")
  .option("--model <model>", "Model override")
  .action(async (questionParts: string[], options) => {
    const { resolveApiKey } = await import("@oni/auth");
    const { runAgent } = await import("@oni/agent");
    const { Conversation } = await import("@oni/agent/conversation");
    const { createPermissions } = await import("@oni/agent/permissions");
    const { loadConfig } = await import("../config.js");

    const resolved = await resolveApiKey();
    if (!resolved) {
      console.error("No API key. Run `oni login` or set ANTHROPIC_API_KEY.");
      process.exit(1);
    }

    // Read stdin if available
    let stdinContent = "";
    if (!process.stdin.isTTY) {
      const chunks: Buffer[] = [];
      for await (const chunk of process.stdin) {
        chunks.push(chunk);
      }
      stdinContent = Buffer.concat(chunks).toString("utf-8");
    }

    const question = questionParts.join(" ");
    if (!question && !stdinContent) {
      console.error("No input. Provide a question or pipe stdin.");
      process.exit(1);
    }

    const fullMessage = stdinContent
      ? `${stdinContent}\n\n${question}`
      : question;

    const config = loadConfig(process.cwd());
    const model = (options.model as string) ?? config.model;
    const projectDir = process.cwd();

    const permissions = createPermissions({
      allowWrite: options.write as boolean,
      allowExec: options.exec as boolean,
      projectDir,
    });

    const conversation = new Conversation();
    const c = {
      cyan: chalk.hex("#00d4c8"),
      coral: chalk.hex("#ff4d2e"),
      muted: chalk.hex("#5a5855"),
      dim: chalk.hex("#323230"),
    };

    try {
      for await (const event of runAgent(fullMessage, conversation, {
        model,
        apiKey: resolved.key,
        projectDir,
        permissions,
      })) {
        switch (event.type) {
          case "text":
            process.stdout.write(event.content ?? "");
            break;
          case "tool_call":
            process.stderr.write(
              `${c.cyan("[tool]")} ${c.cyan(event.tool ?? "")} ${c.muted(JSON.stringify(event.args ?? {}).slice(0, 80))}\n`,
            );
            break;
          case "tool_result":
            if (event.isError) {
              process.stderr.write(c.coral(`  error: ${(event.result ?? "").slice(0, 200)}\n`));
            }
            break;
          case "error":
            process.stderr.write(c.coral(`error: ${event.content}\n`));
            break;
          case "done":
            break;
        }
      }
      process.stdout.write("\n");
    } catch (err) {
      console.error(`Error: ${(err as Error).message}`);
      process.exit(1);
    }
  });
