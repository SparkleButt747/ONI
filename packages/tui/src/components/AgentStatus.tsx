import React from "react";
import { Box, Text } from "ink";
import { color, subAgent, type AgentRole } from "../theme.js";

type AgentState = "active" | "idle" | "reviewing";

interface AgentStatusProps {
  states: Record<AgentRole, AgentState>;
}

const stateColor: Record<AgentState, string> = {
  active: color.amber,
  idle: color.dim,
  reviewing: color.warning,
};

export function AgentStatus({ states }: AgentStatusProps) {
  return (
    <Box gap={2}>
      {(Object.keys(states) as AgentRole[]).map((role) => {
        const cfg = subAgent[role];
        const state = states[role];
        return (
          <Box key={role} gap={1}>
            <Text color={cfg.color} bold>
              {cfg.prefix}
            </Text>
            <Text color={stateColor[state]}>{state.toUpperCase()}</Text>
          </Box>
        );
      })}
    </Box>
  );
}
