import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface ToolCallLineProps {
  timestamp: string;
  tool: string;
  args: string;
  latency: string;
  plugin?: string;
  status?: "ok" | "fail";
}

export function ToolCallLine({
  timestamp,
  tool,
  args,
  latency,
  plugin,
  status = "ok",
}: ToolCallLineProps) {
  const isFail = status === "fail";
  const isPlugin = !!plugin;

  // Badge
  let badgeText = "tool";
  let badgeColor: string = color.cyan;
  if (isFail) {
    badgeText = "fail";
    badgeColor = color.coral;
  } else if (isPlugin) {
    badgeText = "plugin";
    badgeColor = color.violet;
  }

  const toolDisplay = plugin ? `${plugin}:${tool}` : tool;
  const toolColor = isFail ? color.coral : isPlugin ? color.violet : color.cyan;

  return (
    <Box gap={1}>
      <Text color={color.dim}>{timestamp}</Text>
      <Text color={badgeColor}>{badgeText.padEnd(6)}</Text>
      <Text color={toolColor}>{toolDisplay.padEnd(20)}</Text>
      <Text color={color.muted} wrap="truncate">
        {args}
      </Text>
      <Box flexGrow={1} />
      <Text color={isFail ? color.coral : color.dim}>{latency}</Text>
    </Box>
  );
}
