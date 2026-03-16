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
  lines: DiffLine[];
}

export function DiffView({ file, lines }: DiffViewProps) {
  return (
    <Box flexDirection="column">
      <Text color={color.muted}>
        {"─── "}
        <Text color={color.text}>{file}</Text>
      </Text>
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

        const num = line.lineNum?.toString().padStart(3, " ") ?? "   ";

        return (
          <Text key={`${line.type}-${line.lineNum ?? i}-${i}`}>
            <Text color={color.dim}>{num} </Text>
            <Text color={lineColor}>
              {prefix} {line.content}
            </Text>
          </Text>
        );
      })}
    </Box>
  );
}
