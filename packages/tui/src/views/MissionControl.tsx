import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";
import {
  HazardDivider,
  SectionHeader,
  TaskQueue,
  ToolCallLine,
  DiffView,
  AgentStatus,
  SyncPanel,
  ProgressBar,
} from "../components/index.js";
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
  const burnStr =
    oni.burnRate >= 1000
      ? `${(oni.burnRate / 1000).toFixed(1)}k`
      : `${oni.burnRate}`;
  const runningCount = oni.tasks.filter(
    (t) => t.status === "RUNNING",
  ).length;

  // Quarter width for stat cards — leave room for borders
  const cardW = Math.floor((width - 8) / 4);

  return (
    <Box flexDirection="column" width={width}>
      {/* STAT CARDS ROW */}
      <Box gap={1}>
        <Box
          flexDirection="column"
          borderStyle="single"
          borderColor={color.border}
          paddingX={1}
          width={cardW}
        >
          <Text color={color.lime} bold>
            {String(runningCount)}
          </Text>
          <Text color={color.muted}>RUNNING TASKS</Text>
        </Box>
        <Box
          flexDirection="column"
          borderStyle="single"
          borderColor={color.border}
          paddingX={1}
          width={cardW}
        >
          <Text color={color.white} bold>
            {tokStr}
          </Text>
          <Text color={color.muted}>TOKENS USED</Text>
        </Box>
        <Box
          flexDirection="column"
          borderStyle="single"
          borderColor={color.border}
          paddingX={1}
          width={cardW}
        >
          <Text
            color={
              oni.burnRate > 5000
                ? color.coral
                : oni.burnRate > 2000
                  ? color.warning
                  : color.white
            }
            bold
          >
            {burnStr}
          </Text>
          <Text color={color.muted}>TOK / MIN</Text>
        </Box>
        <Box
          flexDirection="column"
          borderStyle="single"
          borderColor={color.border}
          paddingX={1}
          width={cardW}
        >
          <Text color={color.white} bold>
            {String(oni.toolLog.length)}
          </Text>
          <Text color={color.muted}>TOOL CALLS</Text>
        </Box>
      </Box>

      <HazardDivider width={width} />

      {/* TASK QUEUE — full width */}
      <Box marginTop={1} flexDirection="column">
        <SectionHeader title="Task queue" accentColor={color.amber} />
        <Box marginTop={1}>
          <TaskQueue tasks={oni.tasks} />
        </Box>
      </Box>

      <Box marginTop={1}>
        <Text color={color.border}>{"─".repeat(width)}</Text>
      </Box>

      {/* SYNC + CONTEXT + AGENTS — stacked */}
      <Box marginTop={1} flexDirection="column">
        <SectionHeader title="Claude.ai sync" accentColor={color.lime} />
        <Box marginTop={1}>
          <SyncPanel
            status={oni.syncStatus}
            convId={oni.convId}
            lastSync="3s ago"
          />
        </Box>
      </Box>

      <Box marginTop={1} flexDirection="column">
        <SectionHeader title="Context window" accentColor={color.warning} />
        <Box marginTop={1} flexDirection="column">
          <ProgressBar
            label={`${tokStr} / 200k tokens`}
            value={oni.tokens / oni.maxTokens}
            width={Math.min(30, width - 30)}
            warnAt={0.6}
            critAt={0.8}
            valueLabel={`${Math.round((oni.tokens / oni.maxTokens) * 100)}%`}
          />
          <ProgressBar
            label="burn rate"
            value={Math.min(oni.burnRate / 5000, 1)}
            width={Math.min(30, width - 30)}
            warnAt={0.5}
            critAt={0.75}
            valueLabel={`${burnStr}/m`}
          />
        </Box>
      </Box>

      <Box marginTop={1} flexDirection="column">
        <SectionHeader title="Sub-agents" accentColor={color.violet} />
        <Box marginTop={1}>
          <AgentStatus states={oni.agentStates} />
        </Box>
      </Box>

      <HazardDivider width={width} />

      {/* TOOL LOG — full width */}
      <Box marginTop={1} flexDirection="column">
        <SectionHeader title="Tool call log" accentColor={color.cyan} />
        <Box marginTop={1} flexDirection="column">
          {oni.toolLog.length === 0 ? (
            <Text color={color.dim}>No tool calls yet.</Text>
          ) : (
            oni.toolLog.slice(-8).map((tc, i) => (
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

      {/* ACTIVE DIFF — full width */}
      {oni.activeDiff && (
        <Box marginTop={1} flexDirection="column">
          <SectionHeader
            title={`Active diff — ${oni.activeDiff.file}`}
            accentColor={color.lime}
          />
          <Box marginTop={1}>
            <DiffView
              file={oni.activeDiff.file}
              additions={oni.activeDiff.additions}
              deletions={oni.activeDiff.deletions}
              lines={oni.activeDiff.lines}
              showActions
            />
          </Box>
        </Box>
      )}
    </Box>
  );
}
