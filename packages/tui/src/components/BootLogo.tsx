import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

export function BootLogo() {
  return (
    <Box flexDirection="row" gap={2}>
      <Box>
        <Text color={color.white} bold>ON</Text>
        <Text color={color.amber} bold>I</Text>
      </Box>
      <Box borderLeft borderColor={color.border} paddingLeft={2} flexDirection="column">
        <Text color={color.muted}>ONBOARD NEURAL INTELLIGENCE</Text>
        <Text color={color.text}>
          v0.1.0 · <Text color={color.cyan}>claude-sonnet-4-6</Text> · non-commercial
        </Text>
      </Box>
    </Box>
  );
}
