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

// Vertical "SYSTEM" label — one char per line
const SYSTEM_VERT = ["S", "Y", "S", "T", "E", "M"];

export function BootLogo() {
  return (
    <Box flexDirection="row" gap={2}>
      {/* Vertical SYSTEM label on the left */}
      <Box flexDirection="column" justifyContent="center">
        {SYSTEM_VERT.map((ch, i) => (
          <Text key={`sv-${i}`} color={color.lime} dimColor bold>
            {ch}
          </Text>
        ))}
      </Box>

      {/* Logo block */}
      <Box flexDirection="column">
        {ON_LINES.map((line, i) => (
          <Box key={`on-${i}`} gap={0}>
            <Text color={color.white} bold>{line}</Text>
            <Text color={color.lime} bold>{I_LINES[i]}</Text>
          </Box>
        ))}
      </Box>

      {/* Data cluster on the right */}
      <Box
        borderLeft
        borderColor={color.border}
        paddingLeft={2}
        flexDirection="column"
        justifyContent="flex-end"
      >
        <Text color={color.lime} bold>SYSTEM_</Text>
        <Text color={color.muted}>ONBOARD NEURAL INTELLIGENCE</Text>
        <Text color={color.text}>
          V0.1.0 · <Text color={color.cyan}>MODEL</Text>{" "}
          <Text color={color.cyan}>CLAUDE-SONNET-4-6</Text>
        </Text>
        <Text color={color.text}>
          <Text color={color.lime}>AUTH</Text>{" "}
          <Text color={color.muted}>VALIDATED</Text> ·{" "}
          <Text color={color.violet}>PLUGINS</Text>{" "}
          <Text color={color.muted}>0 LOADED</Text>
        </Text>
      </Box>
    </Box>
  );
}
