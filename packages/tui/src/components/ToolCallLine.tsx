import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface ToolCallLineProps {
  timestamp?: string;
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

  let badgeText: string;
  let badgeColor: string;
  if (isFail) {
    badgeText = "fail";
    badgeColor = color.coral;
  } else if (isPlugin) {
    badgeText = "plugin";
    badgeColor = color.violet;
  } else {
    badgeText = "tool";
    badgeColor = color.cyan;
  }

  const toolDisplay = plugin ? `${plugin}:${tool}` : tool;
  const toolColor = isFail ? color.coral : isPlugin ? color.violet : color.cyan;

  return (
    <Box gap={1}>
      {timestamp && <Text color={color.dim}>{timestamp}</Text>}
      <Text color={badgeColor}>[{badgeText}]</Text>
      <Text color={toolColor} bold>{toolDisplay}</Text>
      <Text color={color.muted}>{args}</Text>
      <Box flexGrow={1} />
      <Text color={isFail ? color.coral : color.dim}>{latency}</Text>
    </Box>
  );
}
