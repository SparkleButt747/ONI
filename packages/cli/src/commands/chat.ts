import { Command } from "commander";
import chalk from "chalk";
import React from "react";
import { render } from "ink";

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
    const {
      createConversation,
      insertMessage,
      touchConversation,
    } = await import("@oni/db/queries");
    const { loadConfig, getDataDir } = await import("../config.js");
    const { join } = await import("node:path");
    const { App } = await import("@oni/tui/app");

    type ONIState = import("@oni/tui/context").ONIState;
    type CreateDispatchFn = import("@oni/tui/hooks").CreateDispatchFn;

    const resolved = await resolveApiKey();
    if (!resolved) {
      console.error(
        chalk.hex("#ff4d2e")(
          "No API key found. Run `oni login` or set ANTHROPIC_API_KEY.",
        ),
      );
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

    // Dispatch factory: creates a function bound to the ONI context
    // that streams agent events into the TUI state.
    const createDispatch: CreateDispatchFn = (oni: ONIState) => {
      return async (message: string) => {
        insertMessage(db, conv.conv_id, "user", message);
        touchConversation(db, conv.conv_id);

        const agentConfig = {
          model,
          apiKey: resolved.key,
          projectDir,
          permissions,
        };

        let fullResponse = "";
        let currentMessageId: string | null = null;

        for await (const event of runAgent(message, conversation, agentConfig)) {
          switch (event.type) {
            case "text": {
              fullResponse += event.content ?? "";
              if (!currentMessageId) {
                // Start a new assistant message
                currentMessageId = `oni-${Date.now()}`;
                oni.addMessage({
                  id: currentMessageId,
                  role: "oni",
                  content: fullResponse,
                });
              } else {
                // Update existing message with accumulated text
                oni.updateLastMessage((prev) => ({
                  ...prev,
                  content: fullResponse,
                }));
              }
              break;
            }
            case "tool_call": {
              // If we were streaming text, finalise that message segment
              // and reset so the next text chunk starts a new message
              if (currentMessageId) {
                currentMessageId = null;
              }
              const tc = {
                timestamp: new Date().toLocaleTimeString("en-GB", {
                  hour: "2-digit",
                  minute: "2-digit",
                  second: "2-digit",
                }),
                tool: event.tool ?? "",
                args: JSON.stringify(event.args ?? {}).slice(0, 100),
                latency: "...",
              };
              oni.addToolCall(tc);
              break;
            }
            case "tool_result": {
              // Tool result handled internally by the agent loop
              break;
            }
            case "done": {
              if (fullResponse) {
                insertMessage(db, conv.conv_id, "assistant", fullResponse);

                // Budget tracking
                const estimatedTokens = Math.ceil(fullResponse.length / 4);
                const withinBudget = budget.record(estimatedTokens);
                if (!withinBudget) {
                  oni.addMessage({
                    id: `budget-${Date.now()}`,
                    role: "oni",
                    content: `Budget exceeded. ${budget.summary().replace("\n", " | ")}`,
                  });
                }

                // Update token display
                oni.setTokens(budget.sessionUsed);
              }
              break;
            }
            case "error": {
              oni.addMessage({
                id: `err-${Date.now()}`,
                role: "oni",
                content: `Error: ${event.content}`,
              });
              break;
            }
          }
        }
      };
    };

    // Boot steps reflecting real init state
    const bootSteps = [
      {
        label: "auth",
        detail: `API key (${resolved.source})`,
        ok: true,
      },
      {
        label: "db",
        detail: `SQLite ready · ${conv.conv_id.slice(0, 8)}`,
        ok: true,
      },
      {
        label: "tools",
        detail: "4 built-in tools loaded",
        ok: true,
      },
      {
        label: "perms",
        detail: `${options.write ? "write" : "read-only"}${options.exec ? " + exec" : ""}`,
        ok: true,
      },
    ];

    if (sessionLimit > 0 || monthlyLimit > 0) {
      const parts: string[] = [];
      if (sessionLimit > 0) parts.push(`${sessionLimit} tok/session`);
      if (monthlyLimit > 0) parts.push(`${monthlyLimit} tok/month`);
      bootSteps.push({
        label: "budget",
        detail: parts.join(" · "),
        ok: true,
      });
    }

    const { waitUntilExit } = render(
      React.createElement(App, {
        createDispatch,
        convId: conv.conv_id,
        model,
        bootSteps,
      }),
      { exitOnCtrlC: true },
    );

    await waitUntilExit();
    db.close();
  });
