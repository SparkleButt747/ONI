import React from "react";
import { Box, Text } from "ink";
import { color, type SyncStatus, syncColor } from "../theme.js";

interface SyncPanelProps {
  status: SyncStatus;
  convId: string;
  lastSync: string;
}

export function SyncPanel({ status, convId, lastSync }: SyncPanelProps) {
  return (
    <Box
      borderStyle="single"
      borderColor={color.border}
      paddingX={1}
      gap={1}
    >
      <Text color={syncColor[status]} bold>
        {status === "LIVE" ? "● " : ""}
        {status}
      </Text>
      <Text color={color.white}>Session linked</Text>
      <Box flexGrow={1} />
      <Text color={color.muted}>
        Last sync {lastSync} · {convId}
      </Text>
    </Box>
  );
}
