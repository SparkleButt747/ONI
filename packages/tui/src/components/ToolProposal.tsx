import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface ProposedTool {
  index: number;
  tool: string;
  args: string;
  optional?: boolean;
}

interface ToolProposalProps {
  tools: ProposedTool[];
}

export function ToolProposal({ tools }: ToolProposalProps) {
  return (
    <Box flexDirection="column">
      <Text color={color.text}>Proposing:</Text>
      {tools.map((t) => (
        <Box key={t.index} gap={1}>
          <Text color={color.muted}> [{t.index}]</Text>
          <Text color={color.cyan}>{t.tool.padEnd(14)}</Text>
          <Text color={color.muted}>{t.args}</Text>
          {t.optional && (
            <Text color={color.dim}> — optional</Text>
          )}
        </Box>
      ))}
      <Box marginTop={1}>
        <Text color={color.muted}>
          Use all? <Text color={color.amber}>[enter]</Text> · pick{" "}
          <Text color={color.amber}>[1/2/3]</Text> · skip{" "}
          <Text color={color.amber}>[s]</Text> · always auto{" "}
          <Text color={color.amber}>[a]</Text>
        </Text>
      </Box>
    </Box>
  );
}
