import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface PhaseGateProps {
  phase: number;
  feature: string;
}

/**
 * Placeholder for features not yet implemented.
 * Shows which phase the feature is planned for.
 */
export function PhaseGate({ phase, feature }: PhaseGateProps) {
  return (
    <Box
      borderStyle="single"
      borderColor={color.lime}
      paddingX={1}
      paddingY={0}
      marginTop={1}
    >
      <Text color={color.lime} dimColor>
        {"━ "}
        <Text color={color.muted}>{feature.toUpperCase()}</Text>
        {" — "}
        <Text color={color.amber}>PHASE {phase}</Text>
      </Text>
    </Box>
  );
}
