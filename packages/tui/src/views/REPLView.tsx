import React from "react";
import { Box, Text } from "ink";
import { color, subAgent } from "../theme.js";
import {
  SessionHeader,
  MessageBubble,
  ToolCallLine,
  DiffView,
  InputPrompt,
} from "../components/index.js";
import { useONI } from "../context/oni-context.js";

interface REPLViewProps {
  width: number;
}

export function REPLView({ width }: REPLViewProps) {
  const oni = useONI();

  const handleSubmit = (text: string) => {
    // Handle commands
    if (text === ":q" || text === ":quit") {
      process.exit(0);
    }
    if (text === ":mc") {
      oni.setView("mc");
      return;
    }

    // If a real dispatch function is wired up, use it
    if (oni.dispatch) {
      oni.addMessage({
        id: `user-${Date.now()}`,
        role: "user",
        content: text,
      });
      oni.dispatch(text).catch((err: Error) => {
        oni.addMessage({
          id: `err-${Date.now()}`,
          role: "oni",
          content: `Error: ${err.message}`,
        });
      });
      return;
    }

    // Demo fallback: simulate a response
    oni.addMessage({
      id: `user-${Date.now()}`,
      role: "user",
      content: text,
    });

    setTimeout(() => {
      oni.setAgentStates({
        planner: "active",
        executor: "idle",
        critic: "idle",
      });
    }, 300);

    setTimeout(() => {
      oni.addMessage({
        id: `plan-${Date.now()}`,
        role: "oni",
        agent: "planner",
        content: "Decomposing · analysing request · budget: 4 calls",
      });
      oni.setAgentStates({
        planner: "idle",
        executor: "active",
        critic: "idle",
      });
    }, 1200);

    setTimeout(() => {
      oni.addToolCall({
        timestamp: new Date().toLocaleTimeString("en-GB", {
          hour: "2-digit",
          minute: "2-digit",
          second: "2-digit",
        }),
        tool: "read_file",
        args: "analysing context...",
        latency: "8ms",
      });
    }, 1800);

    setTimeout(() => {
      oni.addMessage({
        id: `exec-${Date.now()}`,
        role: "oni",
        agent: "executor",
        content: "Acknowledged. Processing your request.",
      });
      oni.setAgentStates({
        planner: "idle",
        executor: "idle",
        critic: "active",
      });
    }, 2800);

    setTimeout(() => {
      oni.addMessage({
        id: `crit-${Date.now()}`,
        role: "oni",
        agent: "critic",
        content: "Output accepted · clean.",
      });
      oni.setAgentStates({
        planner: "idle",
        executor: "idle",
        critic: "idle",
      });
    }, 3500);
  };

  return (
    <Box flexDirection="column" width={width}>
      {/* Session header */}
      <SessionHeader
        convId={oni.convId}
        model={oni.model}
        tokens={oni.tokens}
        maxTokens={oni.maxTokens}
        burnRate={oni.burnRate}
        syncStatus={oni.syncStatus}
      />

      {/* Message history */}
      <Box flexDirection="column" marginTop={1}>
        {oni.messages.map((msg) => {
          // For executor messages, show as "oni > [lightning]" then content below
          if (msg.role === "oni" && msg.agent === "executor") {
            const execPrefix = subAgent.executor;
            return (
              <Box key={msg.id} flexDirection="column" marginTop={1}>
                <Box gap={0}>
                  <Text color={color.lime}>{"ONI › "}</Text>
                  <Text color={execPrefix.color} bold>{execPrefix.prefix}</Text>
                </Box>
                <Text color={color.text}>{msg.content}</Text>
                {/* Inline diff */}
                {msg.diff && (
                  <Box marginTop={1} marginLeft={2}>
                    <DiffView
                      file={msg.diff.file}
                      additions={msg.diff.additions}
                      deletions={msg.diff.deletions}
                      lines={msg.diff.lines}
                    />
                  </Box>
                )}
                {/* Write prompt after diff */}
                {msg.diff && (
                  <Box marginTop={1}>
                    <Text color={color.muted}>
                      WRITE TO <Text color={color.white}>{msg.diff.file}</Text>?{" "}
                      <Text color={color.lime} bold>[Y]</Text> / N / DIFF
                    </Text>
                  </Box>
                )}
              </Box>
            );
          }

          return (
            <Box key={msg.id} flexDirection="column">
              <MessageBubble
                role={msg.role}
                content={msg.content}
                agent={msg.agent}
              />
              {/* Inline tool calls that came with this message */}
              {msg.toolCalls?.map((tc, i) => (
                <Box key={`${msg.id}-tc-${i}`} marginLeft={2}>
                  <ToolCallLine
                    tool={tc.tool}
                    args={tc.args}
                    latency={tc.latency}
                    plugin={tc.plugin}
                    status={tc.status}
                  />
                </Box>
              ))}
              {/* Inline diff (non-executor messages) */}
              {msg.diff && (
                <Box marginTop={1} marginLeft={2}>
                  <DiffView
                    file={msg.diff.file}
                    additions={msg.diff.additions}
                    deletions={msg.diff.deletions}
                    lines={msg.diff.lines}
                  />
                </Box>
              )}
            </Box>
          );
        })}
      </Box>

      {/* Thinking indicator */}
      {oni.isProcessing && (
        <Box marginTop={1}>
          <Text color={color.lime}>{"ONI › "}</Text>
          <Text color={color.muted}>THINKING...</Text>
        </Box>
      )}

      {/* Separator */}
      <Box marginTop={1}>
        <Text color={color.border}>
          {"─".repeat(Math.min(width, 80))}
        </Text>
      </Box>

      {/* Input */}
      <Box marginTop={1}>
        <InputPrompt onSubmit={handleSubmit} isActive={!oni.isProcessing} />
      </Box>
    </Box>
  );
}
