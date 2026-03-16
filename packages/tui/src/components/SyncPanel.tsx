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
    <Box gap={2}>
      <Text color={color.muted}>
        SYNC{" "}
        <Text color={syncColor[status]} bold>
          {status === "LIVE" ? "● " : ""}
          {status}
        </Text>
      </Text>
      <Text color={color.muted}>
        CONV <Text color={color.text}>{convId}</Text>
      </Text>
      <Text color={color.muted}>
        LAST <Text color={color.text}>{lastSync}</Text>
      </Text>
    </Box>
  );
}
