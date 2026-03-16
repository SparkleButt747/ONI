import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

export function BootLogo() {
  return (
    <Box flexDirection="row" gap={0}>
      <Box flexDirection="column">
        <Text color={color.white} bold>{"  ██████╗  ███╗   ██╗"}</Text>
        <Text color={color.white} bold>{" ██╔═══██╗ ████╗  ██║"}</Text>
        <Text color={color.white} bold>{" ██║   ██║ ██╔██╗ ██║"}</Text>
        <Text color={color.white} bold>{" ██║   ██║ ██║╚██╗██║"}</Text>
        <Text color={color.white} bold>{" ╚██████╔╝ ██║ ╚████║"}</Text>
        <Text color={color.white} bold>{"  ╚═════╝  ╚═╝  ╚═══╝"}</Text>
      </Box>
      <Box flexDirection="column">
        <Text color={color.amber} bold>{" ██╗"}</Text>
        <Text color={color.amber} bold>{" ██║"}</Text>
        <Text color={color.amber} bold>{" ██║"}</Text>
        <Text color={color.amber} bold>{" ██║"}</Text>
        <Text color={color.amber} bold>{" ██║"}</Text>
        <Text color={color.amber} bold>{" ╚═╝"}</Text>
      </Box>
    </Box>
  );
}
