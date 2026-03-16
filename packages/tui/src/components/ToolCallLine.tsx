import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface ToolCallLineProps {
  timestamp: string;
  tool: string;
  args: string;
  latency: string;
  plugin?: string;
}

export function ToolCallLine({
  timestamp,
  tool,
  args,
  latency,
  plugin,
}: ToolCallLineProps) {
  const toolDisplay = plugin ? `${plugin}:${tool}` : tool;

  return (
    <Box gap={1}>
      <Text color={color.muted}>{timestamp}</Text>
      <Text color={color.cyan}>{toolDisplay.padEnd(16)}</Text>
      <Text color={color.muted} wrap="truncate">
        {args}
      </Text>
      <Box flexGrow={1} />
      <Text color={color.muted}>{latency}</Text>
    </Box>
  );
}
