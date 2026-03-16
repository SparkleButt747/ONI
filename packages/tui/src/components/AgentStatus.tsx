import React from "react";
import { Box, Text } from "ink";
import { color, subAgent, type AgentRole } from "../theme.js";

type AgentState = "active" | "idle" | "reviewing";

interface AgentStatusProps {
  states: Record<AgentRole, AgentState>;
}

const stateColor: Record<AgentState, string> = {
  active: color.lime,
  idle: color.muted,
  reviewing: color.warning,
};

export function AgentStatus({ states }: AgentStatusProps) {
  return (
    <Box flexDirection="column" gap={0}>
      {(Object.keys(states) as AgentRole[]).map((role) => {
        const cfg = subAgent[role];
        const state = states[role];
        const isActive = state === "active";
        const borderCol = isActive ? cfg.color : color.border;

        return (
          <Box
            key={role}
            borderStyle="single"
            borderColor={borderCol}
            paddingX={1}
          >
            <Text color={isActive ? cfg.color : color.muted}>
              {cfg.prefix} {cfg.label}
            </Text>
            <Box flexGrow={1} />
            <Text> </Text>
            <Text color={stateColor[state]}>
              {state.toUpperCase()}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}
