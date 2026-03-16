import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface BlockedStateProps {
  reason: string;
  detail?: string;
  link?: string;
}

export function BlockedState({ reason, detail, link }: BlockedStateProps) {
  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={color.warning}
      paddingX={1}
      paddingY={1}
      marginTop={1}
    >
      <Box gap={1} marginBottom={1}>
        <Text color={color.warning} bold>
          BLOCKED
        </Text>
        <Text color={color.warning}>{reason}</Text>
      </Box>
      {detail && <Text color={color.text}>{detail}</Text>}
      {link && (
        <Box marginTop={1}>
          <Text color={color.muted}>
            {"→ "}
            <Text color={color.cyan}>{link}</Text>
          </Text>
        </Box>
      )}
    </Box>
  );
}
