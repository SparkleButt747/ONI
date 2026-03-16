import React from "react";
import { Box, Text } from "ink";
import { color, type SyncStatus, syncColor } from "../theme.js";

interface StatBarProps {
  version: string;
  convId: string;
  tokens: string;
  runningTasks: number;
  toolCalls: number;
  burnRate: number;
  syncStatus: SyncStatus;
  model: string;
}

export function StatBar({
  version,
  convId,
  tokens,
  runningTasks,
  toolCalls,
  burnRate,
  syncStatus,
  model,
}: StatBarProps) {
  const taskColor = runningTasks > 0 ? color.lime : color.muted;
  let burnColor: string = color.muted;
  if (burnRate > 5000) burnColor = color.coral;
  else if (burnRate > 2000) burnColor = color.amber;

  return (
    <Box flexDirection="column">
      <Box gap={2}>
        <Text color={color.muted}>
          ONI <Text color={color.amber}>v{version}</Text>
        </Text>
        <Text color={color.muted}>
          MODEL <Text color={color.text}>{model}</Text>
        </Text>
        <Text color={color.muted}>
          CONV <Text color={color.text}>{convId}</Text>
        </Text>
        <Text color={color.muted}>
          SYNC{" "}
          <Text color={syncColor[syncStatus]} bold>
            {syncStatus}
          </Text>
        </Text>
      </Box>
      <Box gap={2}>
        <Text color={color.muted}>
          TOKENS <Text color={color.text}>{tokens}</Text>
        </Text>
        <Text color={color.muted}>
          TASKS{" "}
          <Text color={taskColor} bold>
            {runningTasks}
          </Text>
        </Text>
        <Text color={color.muted}>
          TOOLS <Text color={color.text}>{toolCalls}</Text>
        </Text>
        <Text color={color.muted}>
          BURN{" "}
          <Text color={burnColor}>
            {burnRate} <Text color={color.muted}>tok/min</Text>
          </Text>
        </Text>
      </Box>
    </Box>
  );
}
