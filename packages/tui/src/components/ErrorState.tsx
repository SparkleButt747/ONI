import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface ErrorStateProps {
  tool: string;
  args: string;
  error: string;
  suggestion?: string;
}

export function ErrorState({
  tool,
  args,
  error,
  suggestion,
}: ErrorStateProps) {
  return (
    <Box flexDirection="column" marginTop={1}>
      <Box gap={1}>
        <Text color={color.coral} backgroundColor={color.coral}>
          {" FAIL "}
        </Text>
        <Text color={color.coral}>{tool}</Text>
        <Text color={color.muted}>{args}</Text>
        <Box flexGrow={1} />
        <Text color={color.coral}>ERR</Text>
      </Box>
      <Box borderLeft borderColor={color.coral} paddingLeft={1} marginTop={1}>
        <Text color={color.coral}>{error}</Text>
      </Box>
      {suggestion && (
        <Box marginTop={1}>
          <Text color={color.muted}>
            Run <Text color={color.amber}>{suggestion}</Text> then retry.
          </Text>
        </Box>
      )}
    </Box>
  );
}
