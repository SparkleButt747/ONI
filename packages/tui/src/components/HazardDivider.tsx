import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface HazardDividerProps {
  width?: number;
}

export function HazardDivider({ width = 80 }: HazardDividerProps) {
  // Build amber/black repeating stripe pattern using block chars
  const segment = "████░";
  const repeatCount = Math.ceil(width / segment.length);
  const bar = segment.repeat(repeatCount).slice(0, width);

  return (
    <Box>
      <Text color={color.amber} dimColor>
        {bar}
      </Text>
    </Box>
  );
}
