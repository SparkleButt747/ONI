import React from "react";
import { Box, Text } from "ink";
import { color, type TaskStatus, statusColor } from "../theme.js";
import { StatusTag } from "./StatusTag.js";

interface Task {
  id: string;
  mission: string;
  status: TaskStatus;
  elapsed?: string;
  blocker?: string;
}

interface TaskQueueProps {
  tasks: Task[];
}

export function TaskQueue({ tasks }: TaskQueueProps) {
  if (tasks.length === 0) {
    return <Text color={color.dim}>No active tasks.</Text>;
  }

  return (
    <Box flexDirection="column">
      {tasks.map((task) => {
        const dotColor = statusColor[task.status];
        const textColor =
          task.status === "DONE" || task.status === "ERROR"
            ? color.dim
            : color.text;

        return (
          <Box
            key={task.id}
            gap={1}
            paddingY={0}
            borderBottom
            borderColor={color.border}
          >
            <Text color={dotColor}>{"●"}</Text>
            <Text color={textColor} wrap="truncate">
              {task.mission}
            </Text>
            <Box flexGrow={1} />
            <StatusTag status={task.status} />
            <Text color={color.dim}>
              {task.elapsed ?? "—"}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}
