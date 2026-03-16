import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface HazardDividerProps {
  width?: number;
}

export function HazardDivider({ width = 80 }: HazardDividerProps) {
  // Lime dashed hazard pattern — v3
  const segment = "━ ";
  const repeatCount = Math.ceil(width / segment.length);
  const bar = segment.repeat(repeatCount).slice(0, width);

  return (
    <Box>
      <Text color={color.lime} dimColor>
        {bar}
      </Text>
    </Box>
  );
}
