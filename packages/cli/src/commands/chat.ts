import { Command } from "commander";
import chalk from "chalk";
import React from "react";
import { render } from "ink";

export const chatCommand = new Command("chat")
  .description("Start an interactive chat session with ONI")
  .option("--write", "Allow file writes")
  .option("--exec", "Allow bash execution")
  .option("--model <model>", "Model override")
  .option("--agents", "Enable sub-agent loop (Planner \u2192 Executor \u2192 Critic)")
  .option("--budget <tokens>", "Max tokens for this session (e.g. 50000)")
  .option("--monthly-limit <tokens>", "Monthly token limit")
  .addHelpText(
    "after",
    `
Examples:
  $ oni chat                              Read-only interactive session
  $ oni chat --write --exec               Full access mode
  $ oni chat --agents                     Multi-agent planning mode
  $ oni chat --model claude-opus-4-6      Use a specific model
  $ oni chat --budget 50000               Cap session at 50k tokens
`,
  )
  .action(async (options) => {
    // Clear terminal on launch
    process.stdout.write("\x1b[2J\x1b[3J\x1b[H");

    const { resolveApiKey } = await import("@oni/auth");
    const { runAgent } = await import("@oni/agent");
    const { runWithSubAgents } = await import("@oni/agent/sub-agents");
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

    const useAgents = Boolean(options.agents);

    // Dispatch factory: creates a function bound to the ONI context
    // that streams agent events into the TUI state.
    const createDispatch: CreateDispatchFn = (oni: ONIState) => {
      return async (message: string) => {
        insertMessage(db, conv.conv_id, "user", message);
        touchConversation(db, conv.conv_id);

        // Retrieve context if an index exists
        let contextChunks: Array<{ path: string; content: string }> | undefined;
        try {
          const { queryContext } = await import("@oni/context");
          const pack = queryContext(projectDir, message, 4000);
          if (pack.chunks.length > 0) {
            contextChunks = pack.chunks.map((c) => ({ path: c.path, content: c.content }));
          }
        } catch {
          // No index or context unavailable — continue without it
        }

        const agentConfig = {
          model,
          apiKey: resolved.key,
          projectDir,
          permissions,
          contextChunks,
        };

        let fullResponse = "";
        let currentMessageId: string | null = null;

        const handleStreamEvent = (event: { type: string; content?: string; tool?: string; args?: Record<string, unknown>; result?: string; isError?: boolean; agent?: string; reason?: string }) => {
          switch (event.type) {
            case "agent_start": {
              const agent = (event as { agent: string }).agent;
              const states = { planner: "idle" as const, executor: "idle" as const, critic: "idle" as const };
              states[agent as keyof typeof states] = "active";
              oni.setAgentStates(states);

              // Add a task entry for this sub-agent run
              oni.setTasks([
                ...oni.tasks,
                {
                  id: `${agent}-${Date.now()}`,
                  mission: `${agent.toUpperCase()} processing`,
                  status: "RUNNING",
                },
              ]);

              // Start a new message segment for this agent
              fullResponse = "";
              currentMessageId = null;
              break;
            }
            case "agent_end": {
              const agent = (event as { agent: string }).agent;
              const states = { ...oni.agentStates };
              states[agent as keyof typeof states] = "idle";
              oni.setAgentStates(states);

              // Update the latest task for this agent to DONE
              const updatedTasks = [...oni.tasks];
              for (let i = updatedTasks.length - 1; i >= 0; i--) {
                if (
                  updatedTasks[i].mission === `${agent.toUpperCase()} processing` &&
                  updatedTasks[i].status === "RUNNING"
                ) {
                  updatedTasks[i] = { ...updatedTasks[i], status: "DONE" };
                  break;
                }
              }
              oni.setTasks(updatedTasks);

              // Finalise the response for this sub-agent segment
              if (fullResponse) {
                insertMessage(db, conv.conv_id, "assistant", fullResponse);
              }
              break;
            }
            case "blocked": {
              oni.setAgentStates({ planner: "idle", executor: "idle", critic: "idle" });
              oni.addMessage({
                id: `blocked-${Date.now()}`,
                role: "oni",
                content: `BLOCKED: ${(event as { reason: string }).reason}`,
              });
              break;
            }
            case "text": {
              fullResponse += event.content ?? "";
              if (!currentMessageId) {
                currentMessageId = `oni-${Date.now()}`;
                oni.addMessage({
                  id: currentMessageId,
                  role: "oni",
                  content: fullResponse,
                });
              } else {
                oni.updateLastMessage((prev) => ({
                  ...prev,
                  content: fullResponse,
                }));
              }
              break;
            }
            case "tool_call": {
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

                const estimatedTokens = Math.ceil(fullResponse.length / 4);
                const withinBudget = budget.record(estimatedTokens);
                if (!withinBudget) {
                  oni.addMessage({
                    id: `budget-${Date.now()}`,
                    role: "oni",
                    content: `Budget exceeded. ${budget.summary().replace("\n", " | ")}`,
                  });
                }

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
        };

        if (useAgents) {
          for await (const event of runWithSubAgents(message, agentConfig)) {
            handleStreamEvent(event);
          }
        } else {
          for await (const event of runAgent(message, conversation, agentConfig)) {
            handleStreamEvent(event);
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

    // Check if context index exists
    try {
      const { existsSync } = await import("node:fs");
      const indexExists = existsSync(join(projectDir, ".oni", "index.db"));
      if (indexExists) {
        bootSteps.push({
          label: "context",
          detail: "project indexed · FTS5 retrieval active",
          ok: true,
        });
      }
    } catch {
      // Context check failed — skip silently
    }

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
