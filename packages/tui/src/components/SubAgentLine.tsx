import React from "react";
import { Box, Text } from "ink";
import { type AgentRole, subAgent, color } from "../theme.js";

interface SubAgentLineProps {
  agent: AgentRole;
  content: string;
}

export function SubAgentLine({ agent, content }: SubAgentLineProps) {
  const cfg = subAgent[agent];

  return (
    <Box gap={1}>
      <Text color={cfg.color} bold>
        {cfg.prefix}
      </Text>
      <Text color={color.muted}>{content}</Text>
    </Box>
  );
}
