import React, { useState, useEffect, useCallback } from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";
import { HazardDivider } from "./HazardDivider.js";
import { BootLogo } from "./BootLogo.js";

interface InitStep {
  label: string;
  detail: string;
}

const INIT_STEPS: InitStep[] = [
  { label: "init", detail: "OAuth token valid — expires in 47d" },
  { label: "init", detail: "Project index loaded — 1,247 files · 18,432 symbols" },
  { label: "init", detail: "Sync daemon running — conv_8fk2a9 linked" },
  { label: "init", detail: "3 plugins loaded — github · npm · docker" },
  { label: "init", detail: "7 learned rules active" },
];

interface BootSequenceProps {
  width: number;
  onComplete: () => void;
}

export function BootSequence({ width, onComplete }: BootSequenceProps) {
  const [visibleSteps, setVisibleSteps] = useState(0);
  const [showHints, setShowHints] = useState(false);

  const stableOnComplete = useCallback(onComplete, []);

  useEffect(() => {
    const timers: ReturnType<typeof setTimeout>[] = [];
    INIT_STEPS.forEach((_, i) => {
      timers.push(setTimeout(() => setVisibleSteps(i + 1), 400 + i * 280));
    });
    const allDone = 400 + INIT_STEPS.length * 280;
    timers.push(setTimeout(() => setShowHints(true), allDone + 200));
    timers.push(setTimeout(stableOnComplete, allDone + 1000));
    return () => timers.forEach(clearTimeout);
  }, [stableOnComplete]);

  return (
    <Box flexDirection="column" width={width}>
      <Box flexDirection="row" gap={2} marginBottom={1}>
        <BootLogo />
        <Box
          flexDirection="column"
          justifyContent="flex-end"
          borderLeft
          borderColor={color.border}
          paddingLeft={2}
        >
          <Text color={color.muted}>ONBOARD NEURAL INTELLIGENCE</Text>
          <Text color={color.text}>
            v0.1.0 · <Text color={color.cyan}>claude-sonnet-4-6</Text> ·
            non-commercial
          </Text>
        </Box>
      </Box>

      <HazardDivider width={width} />

      <Box flexDirection="column" marginTop={1}>
        {INIT_STEPS.slice(0, visibleSteps).map((step, i) => (
          <Text key={`init-${i}`}>
            <Text color={color.muted}>{step.label.padEnd(6)}</Text>
            <Text color={color.lime}>{"✓ "}</Text>
            <Text color={color.text}>{step.detail}</Text>
          </Text>
        ))}
      </Box>

      {visibleSteps === INIT_STEPS.length && (
        <Box marginTop={1}>
          <HazardDivider width={width} />
        </Box>
      )}

      {showHints && (
        <Box marginTop={1}>
          <Text color={color.muted}>
            Type a mission. <Text color={color.amber}>:q</Text> exit ·{" "}
            <Text color={color.amber}>:mc</Text> mission control ·{" "}
            <Text color={color.amber}>:diff</Text> review changes ·{" "}
            <Text color={color.amber}>:tools</Text> plugins
          </Text>
        </Box>
      )}
    </Box>
  );
}
