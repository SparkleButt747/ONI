import React, { useState, useEffect, useCallback } from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";
import { HazardDivider } from "./HazardDivider.js";
import { BootLogo } from "./BootLogo.js";

export interface InitStep {
  label: string;
  detail: string;
  ok: boolean;
}

// Default steps for Phase 1
const DEFAULT_STEPS: InitStep[] = [
  { label: "auth", detail: "API key validated", ok: true },
  { label: "db", detail: "SQLite database ready", ok: true },
  { label: "tools", detail: "4 built-in tools loaded", ok: true },
];

interface BootSequenceProps {
  width: number;
  onComplete: () => void;
  steps?: InitStep[];
}

export function BootSequence({
  width,
  onComplete,
  steps = DEFAULT_STEPS,
}: BootSequenceProps) {
  const [visibleSteps, setVisibleSteps] = useState(0);
  const [showHints, setShowHints] = useState(false);

  const stableOnComplete = useCallback(onComplete, []);

  useEffect(() => {
    const timers: ReturnType<typeof setTimeout>[] = [];
    steps.forEach((_, i) => {
      timers.push(setTimeout(() => setVisibleSteps(i + 1), 300 + i * 250));
    });
    const allDone = 300 + steps.length * 250;
    timers.push(setTimeout(() => setShowHints(true), allDone + 150));
    timers.push(setTimeout(stableOnComplete, allDone + 800));
    return () => timers.forEach(clearTimeout);
  }, [stableOnComplete, steps]);

  return (
    <Box flexDirection="column" width={width}>
      <Box marginBottom={1}>
        <BootLogo />
      </Box>

      <HazardDivider width={width} />

      <Box flexDirection="column" marginTop={1}>
        {steps.slice(0, visibleSteps).map((step, i) => (
          <Text key={`init-${i}`}>
            <Text color={color.muted}>{step.label.padEnd(6)}</Text>
            <Text color={step.ok ? color.lime : color.coral}>
              {step.ok ? "✓ " : "✗ "}
            </Text>
            <Text color={color.text}>{step.detail}</Text>
          </Text>
        ))}
      </Box>

      {visibleSteps === steps.length && (
        <Box marginTop={1}>
          <HazardDivider width={width} />
        </Box>
      )}

      {showHints && (
        <Box marginTop={1}>
          <Text color={color.muted}>
            Type a mission. <Text color={color.amber}>:q</Text> exit ·{" "}
            <Text color={color.amber}>:mc</Text> mission control
          </Text>
        </Box>
      )}
    </Box>
  );
}
