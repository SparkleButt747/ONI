import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface ProgressBarProps {
  label: string;
  value: number; // 0-1
  width?: number;
  fillColor?: string;
  warnAt?: number; // 0-1 threshold for amber
  critAt?: number; // 0-1 threshold for coral
}

export function ProgressBar({
  label,
  value,
  width = 30,
  fillColor,
  warnAt = 0.6,
  critAt = 0.8,
}: ProgressBarProps) {
  const clamped = Math.min(1, Math.max(0, value));
  const filled = Math.round(clamped * width);
  const empty = width - filled;

  let barColor = fillColor ?? color.amber;
  if (clamped >= critAt) barColor = color.coral;
  else if (clamped >= warnAt) barColor = color.warning;

  const pct = Math.round(clamped * 100);

  return (
    <Box flexDirection="column">
      <Text color={color.muted}>
        {label.toUpperCase()} <Text color={color.text}>{pct}%</Text>
      </Text>
      <Text>
        <Text color={barColor}>{"█".repeat(filled)}</Text>
        <Text color={color.dim}>{"░".repeat(empty)}</Text>
      </Text>
    </Box>
  );
}
