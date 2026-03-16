import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

// Compact 3-line block-letter "ONI" matching the HTML ref's bold header feel
const ON_LINES = [
  " ██████  ██   ██",
  "██    ██ ███  ██",
  "██    ██ ████ ██",
  "██    ██ ██ ████",
  " ██████  ██  ███",
];

const I_LINES = [
  "██",
  "██",
  "██",
  "██",
  "██",
];

export function BootLogo() {
  return (
    <Box flexDirection="row" gap={2}>
      <Box flexDirection="column">
        {ON_LINES.map((line, i) => (
          <Box key={`on-${i}`} gap={0}>
            <Text color={color.white} bold>{line}</Text>
            <Text color={color.amber} bold>{I_LINES[i]}</Text>
          </Box>
        ))}
      </Box>
      <Box
        borderLeft
        borderColor={color.border}
        paddingLeft={2}
        flexDirection="column"
        justifyContent="flex-end"
      >
        <Text color={color.muted}>ONBOARD NEURAL INTELLIGENCE</Text>
        <Text color={color.text}>
          v0.1.0 · <Text color={color.cyan}>claude-sonnet-4-6</Text> ·
          non-commercial
        </Text>
      </Box>
    </Box>
  );
}
