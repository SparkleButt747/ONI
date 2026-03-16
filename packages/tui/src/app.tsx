import React from "react";
import { Box, Text, useInput, useApp } from "ink";
import { color, CHROMA } from "./theme.js";
import { HazardDivider } from "./components/index.js";
import { ONIProvider, useONI } from "./context/oni-context.js";
import { useTerminalSize } from "./hooks/use-terminal-size.js";
import { useSimulatedEvents } from "./hooks/use-simulated-events.js";
import { useAgent, type CreateDispatchFn } from "./hooks/use-agent.js";
import { BootView } from "./views/BootView.js";
import { REPLView } from "./views/REPLView.js";
import { MissionControl } from "./views/MissionControl.js";
import type { InitStep } from "./components/BootSequence.js";

interface AppInnerProps {
  createDispatch: CreateDispatchFn | null;
  bootSteps?: InitStep[];
}

function AppInner({ createDispatch, bootSteps }: AppInnerProps) {
  const oni = useONI();
  const { exit } = useApp();
  const { columns } = useTerminalSize();

  // Wire agent dispatch if provided, otherwise run demo simulation
  useAgent(createDispatch);
  if (!createDispatch) {
    useSimulatedEvents();
  }

  useInput((input, key) => {
    if (input === "q" && oni.view !== "repl") {
      exit();
      return;
    }
    if (key.escape) {
      if (oni.view === "mc") {
        oni.setView("repl");
      }
      return;
    }
    if (key.tab && oni.view !== "boot") {
      oni.setView(oni.view === "repl" ? "mc" : "repl");
    }
  });

  // Build chroma stripe — distribute 7 segments across available width
  const segWidth = Math.max(2, Math.floor(columns / CHROMA.length));
  const chromaUsed = segWidth * CHROMA.length;

  return (
    <Box flexDirection="column" width={columns}>
      {oni.view !== "boot" && (
        <>
          {/* Top bar */}
          <Box>
            <Text color={color.lime} bold>
              {"ONI "}
            </Text>
            {oni.view === "mc" ? (
              <Text color={color.muted}>
                MISSION CONTROL · <Text color={color.dim}>{oni.convId}</Text>
              </Text>
            ) : (
              <Text color={color.muted}>ONBOARD NEURAL INTELLIGENCE</Text>
            )}
            <Box flexGrow={1} />
            <Box gap={2}>
              <Text
                color={oni.view === "repl" ? color.lime : color.dim}
                bold={oni.view === "repl"}
              >
                REPL
              </Text>
              <Text
                color={oni.view === "mc" ? color.lime : color.dim}
                bold={oni.view === "mc"}
              >
                MISSION CONTROL
              </Text>
            </Box>
          </Box>
          {/* 7-segment chroma gradient accent line */}
          <Box>
            {CHROMA.map((c, i) => (
              <Text key={`chroma-${i}`} color={c}>
                {"█".repeat(segWidth)}
              </Text>
            ))}
            <Text color={color.dim}>
              {"─".repeat(Math.max(0, columns - chromaUsed))}
            </Text>
          </Box>
        </>
      )}

      {/* Main content */}
      <Box flexDirection="column" marginTop={oni.view === "boot" ? 0 : 1}>
        {oni.view === "boot" && <BootView width={columns} bootSteps={bootSteps} />}
        {oni.view === "repl" && <REPLView width={columns} />}
        {oni.view === "mc" && <MissionControl width={columns} />}
      </Box>

      {/* Footer */}
      {oni.view !== "boot" && (
        <>
          <HazardDivider width={columns} />
          <Box>
            <Text color={color.dim}>
              <Text color={color.muted}>TAB</Text> SWITCH VIEW {"  "}
              <Text color={color.muted}>ESC</Text> BACK TO REPL {"  "}
              <Text color={color.muted}>:Q</Text> QUIT
            </Text>
          </Box>
        </>
      )}
    </Box>
  );
}

export interface AppProps {
  createDispatch?: CreateDispatchFn;
  convId?: string;
  model?: string;
  bootSteps?: InitStep[];
}

export function App({ createDispatch, convId, model, bootSteps }: AppProps) {
  return (
    <ONIProvider convId={convId} model={model}>
      <AppInner
        createDispatch={createDispatch ?? null}
        bootSteps={bootSteps}
      />
    </ONIProvider>
  );
}
