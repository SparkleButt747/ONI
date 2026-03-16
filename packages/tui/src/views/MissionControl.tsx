import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";
import {
  HazardDivider,
  SectionHeader,
  ToolCallLine,
} from "../components/index.js";
import { PhaseGate } from "../components/PhaseGate.js";
import { useONI } from "../context/oni-context.js";

interface MissionControlProps {
  width: number;
}

export function MissionControl({ width }: MissionControlProps) {
  const oni = useONI();

  const tokStr =
    oni.tokens >= 1000
      ? `${(oni.tokens / 1000).toFixed(1)}k`
      : `${oni.tokens}`;

  return (
    <Box flexDirection="column" width={width}>
      <Box>
        <Text color={color.muted}>
          Session <Text color={color.text}>{oni.convId}</Text> ·{" "}
          <Text color={color.text}>{oni.model}</Text> ·{" "}
          <Text color={color.text}>{tokStr} tok</Text>
        </Text>
      </Box>

      <HazardDivider width={width} />

      {/* TOOL LOG — Phase 1 (active) */}
      <Box marginTop={1} flexDirection="column">
        <SectionHeader title="Tool call log" accentColor={color.cyan} />
        <Box marginTop={1} flexDirection="column">
          {oni.toolLog.length === 0 ? (
            <Text color={color.dim}>No tool calls this session.</Text>
          ) : (
            oni.toolLog.slice(-10).map((tc, i) => (
              <ToolCallLine
                key={`mc-tl-${i}`}
                timestamp={tc.timestamp}
                tool={tc.tool}
                args={tc.args}
                latency={tc.latency}
                plugin={tc.plugin}
                status={tc.status}
              />
            ))
          )}
        </Box>
      </Box>

      <HazardDivider width={width} />

      {/* Phase-gated features */}
      <PhaseGate phase={2} feature="Context engine — project indexing + retrieval" />
      <PhaseGate phase={3} feature="Sub-agent status — Planner / Executor / Critic" />
      <PhaseGate phase={3} feature="Task queue — background agents" />
      <PhaseGate phase={3} feature="Claude.ai sync — conversation mirroring" />
      <PhaseGate phase={3} feature="Active diff — inline file change review" />
      <PhaseGate phase={4} feature="Preference learning — adaptive tool proposals" />
      <PhaseGate phase={4} feature="MCP plugins — third-party tool ecosystem" />
    </Box>
  );
}
