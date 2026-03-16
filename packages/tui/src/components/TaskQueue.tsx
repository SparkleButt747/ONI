import React from "react";
import { Box, Text } from "ink";
import { color, type TaskStatus, statusColor } from "../theme.js";
import { StatusTag } from "./StatusTag.js";

interface Task {
  id: string;
  mission: string;
  status: TaskStatus;
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
      {tasks.map((task) => (
        <Box key={task.id} gap={1}>
          <StatusTag status={task.status} />
          <Text color={color.muted}>{task.id.slice(0, 6)}</Text>
          <Text color={color.text} wrap="truncate">
            {task.mission}
          </Text>
          {task.blocker && (
            <Text color={color.coral} dimColor>
              {" "}
              {task.blocker}
            </Text>
          )}
        </Box>
      ))}
    </Box>
  );
}
