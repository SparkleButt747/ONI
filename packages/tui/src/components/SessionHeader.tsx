import React from "react";
import { Box, Text } from "ink";
import { color, syncColor, type SyncStatus } from "../theme.js";

interface SessionHeaderProps {
  convId: string;
  model: string;
  tokens: number;
  maxTokens: number;
  burnRate: number;
  syncStatus: SyncStatus;
}

export function SessionHeader({
  convId,
  model,
  tokens,
  maxTokens,
  burnRate,
  syncStatus,
}: SessionHeaderProps) {
  const tokStr =
    tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : `${tokens}`;
  const maxStr = `${(maxTokens / 1000).toFixed(0)}k`;
  let burnColor: string = color.muted;
  if (burnRate > 5000) burnColor = color.coral;
  else if (burnRate > 2000) burnColor = color.lime;

  return (
    <Box
      paddingBottom={1}
      borderBottom
      borderColor={color.border}
      gap={1}
    >
      <Text color={color.muted}>SESSION</Text>
      <Text color={color.text}>{convId}</Text>
      <Text color={color.dim}>·</Text>
      <Text color={color.cyan}>MODEL {model.toUpperCase()}</Text>
      <Text color={color.dim}>·</Text>
      <Text color={color.text}>
        {tokStr} <Text color={color.muted}>/ {maxStr} TOK</Text>
      </Text>
      <Text color={color.dim}>·</Text>
      <Text color={burnColor}>
        {burnRate > 0
          ? `${(burnRate / 1000).toFixed(1)}K TOK/MIN`
          : "—"}
      </Text>
      <Box flexGrow={1} />
      <Text color={syncColor[syncStatus]} bold>
        {syncStatus === "LIVE" ? "● " : ""}
        SYNC {syncStatus}
      </Text>
    </Box>
  );
}
