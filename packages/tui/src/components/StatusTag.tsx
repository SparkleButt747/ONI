import React from "react";
import { Text } from "ink";
import { type TaskStatus, statusColor, color } from "../theme.js";

interface StatusTagProps {
  status: TaskStatus;
}

export function StatusTag({ status }: StatusTagProps) {
  const c = statusColor[status];

  // v3: all statuses get coloured text + dim bg simulation via dimColor
  const isDone = status === "DONE";

  return (
    <Text color={c} dimColor={!isDone} bold>
      {`[ ${status.toUpperCase()} ]`}
    </Text>
  );
}
