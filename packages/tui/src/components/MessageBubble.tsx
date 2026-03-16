import React from "react";
import { Box, Text } from "ink";
import { color, subAgent, type AgentRole } from "../theme.js";

interface MessageBubbleProps {
  role: "user" | "oni";
  content: string;
  agent?: AgentRole;
}

export function MessageBubble({ role, content, agent }: MessageBubbleProps) {
  if (role === "user") {
    return (
      <Box marginTop={1}>
        <Text color={color.amber} bold>
          {"you › "}
        </Text>
        <Text color={color.white}>{content}</Text>
      </Box>
    );
  }

  const prefix = agent ? subAgent[agent] : null;

  return (
    <Box flexDirection="column" marginTop={1}>
      <Box>
        <Text color={color.amber}>{"oni › "}</Text>
        {prefix && (
          <Text color={prefix.color} bold>
            {prefix.prefix}{" "}
          </Text>
        )}
      </Box>
      <Text color={color.text}>{content}</Text>
    </Box>
  );
}
