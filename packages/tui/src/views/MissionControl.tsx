import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";
import {
  StatBar,
  HazardDivider,
  SectionHeader,
  TaskQueue,
  ToolCallLine,
  DiffView,
  AgentStatus,
  SyncPanel,
  ProgressBar,
} from "../components/index.js";

interface MissionControlProps {
  width: number;
}

export function MissionControl({ width }: MissionControlProps) {
  return (
    <Box flexDirection="column" width={width}>
      {/* STAT BAR */}
      <StatBar
        version="0.1.0"
        convId="8fk2a"
        tokens="47.2k"
        runningTasks={3}
        toolCalls={14}
        burnRate={1842}
        syncStatus="LIVE"
        model="sonnet-4.6"
      />

      <HazardDivider width={width} />

      {/* SUB-AGENT STATUS */}
      <Box flexDirection="column" paddingY={1}>
        <SectionHeader title="Sub-agent status" accentColor={color.violet} />
        <Box marginTop={1}>
          <AgentStatus
            states={{
              planner: "idle",
              executor: "active",
              critic: "idle",
            }}
          />
        </Box>
      </Box>

      {/* TASK QUEUE */}
      <Box flexDirection="column" paddingY={1}>
        <SectionHeader title="Task queue" accentColor={color.amber} />
        <Box marginTop={1} flexDirection="column">
          <TaskQueue
            tasks={[
              {
                id: "a3f82e",
                mission: "refactor auth middleware to async/await",
                status: "RUNNING",
              },
              {
                id: "b7c1d4",
                mission: "add JSDoc to exported functions in services/",
                status: "RUNNING",
              },
              {
                id: "e9f0a2",
                mission: "fix race condition in PricingEngine",
                status: "BLOCKED",
                blocker: "CI requires approval",
              },
              {
                id: "d4e5f6",
                mission: "update deps to latest semver",
                status: "DONE",
              },
              {
                id: "c2d3e4",
                mission: "migrate fetch() calls to ky",
                status: "ERROR",
              },
            ]}
          />
        </Box>
      </Box>

      {/* TOOL CALL LOG */}
      <Box flexDirection="column" paddingY={1}>
        <SectionHeader title="Tool call log" accentColor={color.cyan} />
        <Box marginTop={1} flexDirection="column">
          <ToolCallLine
            timestamp="14:22:01"
            tool="read_file"
            args="src/services/PricingEngine.ts"
            latency="4ms"
          />
          <ToolCallLine
            timestamp="14:22:01"
            tool="read_file"
            args="src/services/OrderService.ts:processTotal"
            latency="3ms"
          />
          <ToolCallLine
            timestamp="14:22:04"
            tool="bash"
            args="npx jest PricingEngine --watch=false"
            latency="1.2s"
          />
          <ToolCallLine
            timestamp="14:22:06"
            tool="write_file"
            args="src/services/PricingEngine.ts"
            latency="2ms"
          />
          <ToolCallLine
            timestamp="14:22:08"
            tool="bash"
            args="npx jest PricingEngine --watch=false"
            latency="1.1s"
          />
          <ToolCallLine
            timestamp="14:22:10"
            tool="create_pr"
            args="--title 'fix: discount applied pre-tax'"
            latency="842ms"
            plugin="github"
          />
        </Box>
      </Box>

      {/* ACTIVE DIFF */}
      <Box flexDirection="column" paddingY={1}>
        <SectionHeader title="Active diff" accentColor={color.lime} />
        <Box marginTop={1} flexDirection="column">
          <DiffView
            file="src/services/PricingEngine.ts"
            lines={[
              {
                type: "context",
                content: "export function processTotal(items: LineItem[], discount: Discount) {",
                lineNum: 44,
              },
              {
                type: "context",
                content: "  const subtotal = items.reduce((sum, i) => sum + i.price * i.qty, 0);",
                lineNum: 45,
              },
              {
                type: "remove",
                content: "  const taxed = calculateTax(subtotal);",
                lineNum: 46,
              },
              {
                type: "remove",
                content: "  return applyDiscount(taxed, discount);",
                lineNum: 47,
              },
              {
                type: "add",
                content: "  const discounted = applyDiscount(subtotal, discount);",
                lineNum: 46,
              },
              {
                type: "add",
                content: "  return calculateTax(discounted);",
                lineNum: 47,
              },
            ]}
          />
        </Box>
      </Box>

      {/* CONTEXT WINDOW + SYNC */}
      <Box flexDirection="column" paddingY={1}>
        <SectionHeader title="Context window" accentColor={color.warning} />
        <Box marginTop={1} flexDirection="column" gap={1}>
          <ProgressBar
            label="tokens"
            value={0.47}
            width={Math.min(40, width - 4)}
          />
          <ProgressBar
            label="burn rate"
            value={0.37}
            width={Math.min(40, width - 4)}
            warnAt={0.5}
            critAt={0.75}
          />
        </Box>
      </Box>

      <Box paddingY={1}>
        <SyncPanel status="LIVE" convId="conv_8fk2a" lastSync="14:22:10" />
      </Box>
    </Box>
  );
}
