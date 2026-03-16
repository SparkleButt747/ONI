import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";
import {
  AgentStatus,
  HazardDivider,
  SectionHeader,
  TaskQueue,
  ToolCallLine,
} from "../components/index.js";
import { PhaseGate } from "../components/PhaseGate.js";
import { useONI } from "../context/oni-context.js";

interface MissionControlProps {
  width: number;
}

// Vertical "MISSION CTRL" label — one char per line
const MC_VERT = ["M", "I", "S", "S", "I", "O", "N", " ", "C", "T", "R", "L"];

export function MissionControl({ width }: MissionControlProps) {
  const oni = useONI();

  const tokStr =
    oni.tokens >= 1000
      ? `${(oni.tokens / 1000).toFixed(1)}k`
      : `${oni.tokens}`;

  return (
    <Box flexDirection="row" width={width}>
      {/* Vertical MISSION CTRL label on the left */}
      <Box flexDirection="column" marginRight={1}>
        {MC_VERT.map((ch, i) => (
          <Text key={`mc-v-${i}`} color={color.lime} dimColor bold>
            {ch}
          </Text>
        ))}
      </Box>

      {/* Main content */}
      <Box flexDirection="column" flexGrow={1}>
        {/* 1. Session info + LIVE badge */}
        <Box>
          <Text color={color.muted}>
            SESSION <Text color={color.text}>{oni.convId}</Text> ·{" "}
            <Text color={color.text}>{oni.model.toUpperCase()}</Text> ·{" "}
            <Text color={color.text}>{tokStr} TOK</Text>
          </Text>
          <Box flexGrow={1} />
          <Text color={color.lime} bold>● LIVE</Text>
        </Box>

        {/* 2. Hazard divider */}
        <HazardDivider width={width - 4} />

        {/* 3. Sub-agent status — UNGATED (Phase 3) */}
        <Box marginTop={1} flexDirection="column">
          <SectionHeader title="SUB-AGENTS" accentColor={color.violet} />
          <Box marginTop={1}>
            <AgentStatus states={oni.agentStates} />
          </Box>
        </Box>

        {/* 4. Task queue — UNGATED (Phase 3) */}
        <Box marginTop={1} flexDirection="column">
          <SectionHeader title="TASK QUEUE" accentColor={color.cyan} />
          <Box marginTop={1}>
            <TaskQueue tasks={oni.tasks} />
          </Box>
        </Box>

        {/* 5. Tool call log */}
        <Box marginTop={1} flexDirection="column">
          <SectionHeader title="TOOL CALL LOG" accentColor={color.cyan} />
          <Box marginTop={1} flexDirection="column">
            {oni.toolLog.length === 0 ? (
              <Text color={color.dim}>NO TOOL CALLS THIS SESSION.</Text>
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

        {/* 6. Hazard divider */}
        <HazardDivider width={width - 4} />

        {/* Phase-gated features (Phase 4+) */}
        <PhaseGate phase={4} feature="Preference learning — adaptive tool proposals" />
        <PhaseGate phase={4} feature="MCP plugins — third-party tool ecosystem" />
      </Box>
    </Box>
  );
}
