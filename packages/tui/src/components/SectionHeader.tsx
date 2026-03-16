import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface SectionHeaderProps {
  title: string;
  accentColor?: string;
  number?: string; // e.g. "01"
}

export function SectionHeader({
  title,
  accentColor = color.cyan,
  number,
}: SectionHeaderProps) {
  return (
    <Box>
      <Text color={accentColor}>{"│ "}</Text>
      {number && (
        <Text color={color.muted}>
          {number}
          {"  "}
        </Text>
      )}
      <Text color={color.white} bold>
        {title.toUpperCase()}
      </Text>
    </Box>
  );
}
