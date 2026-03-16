import React from "react";
import { Box, Text } from "ink";
import { readFileSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";
import { color } from "../theme.js";

// Resolve path to ONI_LOGO.txt relative to project root
function loadLogo(): string[] {
  const paths = [
    resolve(process.cwd(), "docs/Vision/ONI_LOGO.txt"),
    resolve(process.cwd(), "../../docs/Vision/ONI_LOGO.txt"),
  ];

  for (const p of paths) {
    try {
      return readFileSync(p, "utf-8").split("\n").filter((l) => l.length > 0);
    } catch {
      // try next
    }
  }

  // Fallback: simple text logo if file not found
  return [
    "  ██████╗  ███╗   ██╗ ██╗",
    " ██╔═══██╗ ████╗  ██║ ██║",
    " ██║   ██║ ██╔██╗ ██║ ██║",
    " ██║   ██║ ██║╚██╗██║ ██║",
    " ╚██████╔╝ ██║ ╚████║ ██║",
    "  ╚═════╝  ╚═╝  ╚═══╝ ╚═╝",
  ];
}

export function BootLogo() {
  const lines = loadLogo();

  return (
    <Box flexDirection="column">
      {lines.map((line, i) => (
        <Text key={`logo-${i}`} color={color.amber}>
          {line}
        </Text>
      ))}
    </Box>
  );
}
