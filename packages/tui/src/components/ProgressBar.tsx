import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface ProgressBarProps {
  label: string;
  value: number;
  width?: number;
  fillColor?: string;
  warnAt?: number;
  critAt?: number;
  valueLabel?: string;
}

export function ProgressBar({
  label,
  value,
  width = 24,
  fillColor,
  warnAt = 0.6,
  critAt = 0.8,
  valueLabel,
}: ProgressBarProps) {
  const clamped = Math.min(1, Math.max(0, value));
  const filled = Math.round(clamped * width);
  const empty = width - filled;

  let barColor = fillColor ?? color.cyan;
  if (clamped >= critAt) barColor = color.coral;
  else if (clamped >= warnAt) barColor = color.warning;

  const pct = valueLabel ?? `${Math.round(clamped * 100)}%`;

  return (
    <Box gap={1}>
      <Text color={color.muted}>{label.padEnd(20)}</Text>
      <Text>
        <Text color={barColor}>{"█".repeat(filled)}</Text>
        <Text color={color.dim}>{"░".repeat(empty)}</Text>
      </Text>
      <Text color={clamped >= warnAt ? barColor : color.muted}>{pct}</Text>
    </Box>
  );
}
