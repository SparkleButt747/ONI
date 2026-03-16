import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface DiffLine {
  type: "add" | "remove" | "context";
  content: string;
  lineNum?: number;
}

interface DiffViewProps {
  file: string;
  additions?: number;
  deletions?: number;
  lines: DiffLine[];
  showActions?: boolean;
}

export function DiffView({
  file,
  additions,
  deletions,
  lines,
  showActions = false,
}: DiffViewProps) {
  const header = [file];
  if (additions !== undefined || deletions !== undefined) {
    const parts: string[] = [];
    if (additions) parts.push(`+${additions}`);
    if (deletions) parts.push(`-${deletions}`);
    header.push(` · ${parts.join(" ")}`);
  }

  return (
    <Box flexDirection="column" borderLeft borderColor={color.lime} paddingLeft={1}>
      <Text color={color.muted}>{header.join("").toUpperCase()}</Text>
      {lines.map((line, i) => {
        let prefix: string;
        let lineColor: string;

        switch (line.type) {
          case "add":
            prefix = "+";
            lineColor = color.lime;
            break;
          case "remove":
            prefix = "-";
            lineColor = color.coral;
            break;
          default:
            prefix = " ";
            lineColor = color.dim;
        }

        return (
          <Text key={`${line.type}-${line.lineNum ?? i}-${i}`}>
            <Text color={lineColor}>
              {prefix} {line.content}
            </Text>
          </Text>
        );
      })}
      {showActions && (
        <Box marginTop={1} gap={1}>
          <Text color={color.lime}>[ACCEPT]</Text>
          <Text color={color.coral}>[REJECT]</Text>
          <Text color={color.muted}>[ACCEPT FILE]</Text>
        </Box>
      )}
    </Box>
  );
}
