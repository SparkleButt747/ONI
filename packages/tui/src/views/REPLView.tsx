import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";
import {
  SessionHeader,
  MessageBubble,
  ToolCallLine,
  DiffView,
  SubAgentLine,
  InputPrompt,
  HazardDivider,
} from "../components/index.js";
import { useONI } from "../context/oni-context.js";

interface REPLViewProps {
  width: number;
}

export function REPLView({ width }: REPLViewProps) {
  const oni = useONI();

  const handleSubmit = (text: string) => {
    oni.addMessage({
      id: `user-${Date.now()}`,
      role: "user",
      content: text,
    });

    // Simulate a quick response after user input
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
        {oni.messages.map((msg) => (
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
                  timestamp={tc.timestamp}
                  tool={tc.tool}
                  args={tc.args}
                  latency={tc.latency}
                  plugin={tc.plugin}
                  status={tc.status}
                />
              </Box>
            ))}
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
          </Box>
        ))}
      </Box>

      {/* Inline tool log (global — tool calls that arrive between messages) */}
      {oni.toolLog.length > 0 && (
        <Box flexDirection="column" marginTop={1}>
          {oni.toolLog.slice(-6).map((tc, i) => (
            <ToolCallLine
              key={`tl-${i}`}
              timestamp={tc.timestamp}
              tool={tc.tool}
              args={tc.args}
              latency={tc.latency}
              plugin={tc.plugin}
              status={tc.status}
            />
          ))}
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
        <InputPrompt onSubmit={handleSubmit} />
      </Box>
    </Box>
  );
}
