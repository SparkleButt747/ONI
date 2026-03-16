import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";
import {
  SubAgentLine,
  ToolProposal,
  HazardDivider,
} from "../components/index.js";

interface REPLViewProps {
  width: number;
}

export function REPLView({ width }: REPLViewProps) {
  return (
    <Box flexDirection="column" width={width}>
      {/* Header */}
      <Box>
        <Text color={color.muted}>
          ONI <Text color={color.amber}>v0.1.0</Text> ·{" "}
          <Text color={color.text}>conv_8fk2a</Text> ·{" "}
          <Text color={color.text}>47.2k</Text> tok ·{" "}
          <Text color={color.lime}>3 RUNNING</Text>
        </Text>
      </Box>
      <Box>
        <Text color={color.dim}>
          {"─".repeat(Math.min(width, 80))}
        </Text>
      </Box>

      {/* User message */}
      <Box marginTop={1}>
        <Text>
          <Text color={color.amber}>you › </Text>
          <Text color={color.white}>
            the order total is wrong for discount codes
          </Text>
        </Text>
      </Box>

      {/* Planner */}
      <Box marginTop={1} flexDirection="column">
        <SubAgentLine
          agent="planner"
          content="decomposing: 3 subtasks · budget: 6 calls · no ambiguity"
        />
      </Box>

      {/* Executor tool calls */}
      <Box marginTop={1} flexDirection="column">
        <Box gap={1}>
          <Text color={color.cyan} bold>
            {"[⚡]"}
          </Text>
          <Text color={color.dim}>[tool]</Text>
          <Text color={color.cyan}>read_file</Text>
          <Text color={color.muted}>src/services/PricingEngine.ts</Text>
          <Text color={color.dim}>· 4ms</Text>
        </Box>
        <Box gap={1}>
          <Text color={color.cyan} bold>
            {"[⚡]"}
          </Text>
          <Text color={color.dim}>[tool]</Text>
          <Text color={color.cyan}>read_file</Text>
          <Text color={color.muted}>
            src/services/OrderService.ts:processTotal
          </Text>
          <Text color={color.dim}>· 3ms</Text>
        </Box>
      </Box>

      {/* ONI response */}
      <Box marginTop={1} flexDirection="column">
        <Text>
          <Text color={color.amber}>oni › </Text>
          <Text color={color.white}>
            applyDiscount() runs post-tax. Discounts must reduce pre-tax
            subtotal. Swapping call order in processTotal().
          </Text>
        </Text>
      </Box>

      {/* Inline diff */}
      <Box marginTop={1} flexDirection="column">
        <Text>
          <Text color={color.lime}>{"+ "}</Text>
          <Text color={color.muted}>
            calculateTax(subtotal - discount)
          </Text>
        </Text>
        <Text>
          <Text color={color.coral}>{"- "}</Text>
          <Text color={color.muted}>
            applyDiscount(calculateTax(subtotal))
          </Text>
        </Text>
      </Box>

      {/* Write confirmation */}
      <Box marginTop={1}>
        <Text color={color.muted}>
          Write to{" "}
          <Text color={color.text}>src/services/PricingEngine.ts</Text>?{" "}
          <Text color={color.amber}>[y/n/diff]</Text>
        </Text>
      </Box>

      {/* Critic verdict */}
      <Box marginTop={1}>
        <SubAgentLine
          agent="critic"
          content="output accepted · no regressions · tests cover this path · clean."
        />
      </Box>

      <HazardDivider width={width} />

      {/* Tool proposal example */}
      <Box marginTop={1} flexDirection="column">
        <Text>
          <Text color={color.amber}>you › </Text>
          <Text color={color.white}>fix the failing CI build</Text>
        </Text>
      </Box>

      <Box marginTop={1} flexDirection="column">
        <SubAgentLine
          agent="planner"
          content="CI failure analysis · 2 subtasks · budget: 4 calls"
        />
      </Box>

      <Box marginTop={1}>
        <Text>
          <Text color={color.amber}>oni › </Text>
          <Text color={color.text}>
            I can use a few tools here.
          </Text>
        </Text>
      </Box>

      <Box marginTop={1}>
        <ToolProposal
          tools={[
            { index: 1, tool: "gh:get_run_logs", args: "run_id=9241" },
            {
              index: 2,
              tool: "read_file",
              args: "Dockerfile, docker-compose.yml",
            },
            {
              index: 3,
              tool: "bash",
              args: 'docker build . --no-cache 2>&1 | tail -40',
            },
          ]}
        />
      </Box>

      {/* Input prompt */}
      <Box marginTop={1}>
        <Text color={color.amber}>{"you › "}</Text>
        <Text color={color.amber}>{"█"}</Text>
      </Box>
    </Box>
  );
}
