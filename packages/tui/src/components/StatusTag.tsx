import React from "react";
import { Text } from "ink";
import { type TaskStatus, statusColor, color } from "../theme.js";

interface StatusTagProps {
  status: TaskStatus;
}

export function StatusTag({ status }: StatusTagProps) {
  const c = statusColor[status];
  const isError = status === "ERROR";

  return (
    <Text
      color={isError ? color.black : c}
      backgroundColor={isError ? c : undefined}
    >
      {` ${status} `}
    </Text>
  );
}
