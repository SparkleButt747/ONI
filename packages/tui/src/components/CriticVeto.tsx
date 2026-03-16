import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface CriticVetoProps {
  reason: string;
  replanNum: number;
  maxReplans: number;
}

export function CriticVeto({
  reason,
  replanNum,
  maxReplans,
}: CriticVetoProps) {
  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={color.coral}
      paddingX={1}
      paddingY={1}
      marginTop={1}
    >
      <Box gap={1} marginBottom={1}>
        <Text color={color.coral} bold>
          {"[⊘]"}
        </Text>
        <Text color={color.coral} bold>
          REJECTED
        </Text>
      </Box>
      <Text color={color.text}>{reason}</Text>
      <Box marginTop={1}>
        <Text color={color.coral}>
          Replan? <Text color={color.amber}>[y]</Text>
          <Text color={color.muted}>
            {" "}
            / n · replan {replanNum} of {maxReplans}
          </Text>
        </Text>
      </Box>
    </Box>
  );
}
