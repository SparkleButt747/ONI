import React from "react";
import { Box, Text, useInput, useApp } from "ink";
import { color } from "./theme.js";
import { HazardDivider } from "./components/index.js";
import { ONIProvider, useONI } from "./context/oni-context.js";
import { useTerminalSize } from "./hooks/use-terminal-size.js";
import { useSimulatedEvents } from "./hooks/use-simulated-events.js";
import { BootView } from "./views/BootView.js";
import { REPLView } from "./views/REPLView.js";
import { MissionControl } from "./views/MissionControl.js";

function AppInner() {
  const oni = useONI();
  const { exit } = useApp();
  const { columns } = useTerminalSize();

  // Start simulated events
  useSimulatedEvents();

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

  return (
    <Box flexDirection="column" width={columns}>
      {oni.view !== "boot" && (
        <>
          {/* Top bar */}
          <Box>
            <Text color={color.amber} bold>
              {"ONI "}
            </Text>
            <Text color={color.muted}>ONBOARD NEURAL INTELLIGENCE</Text>
            <Box flexGrow={1} />
            <Box gap={2}>
              <Text
                color={oni.view === "repl" ? color.amber : color.dim}
                bold={oni.view === "repl"}
              >
                REPL
              </Text>
              <Text
                color={oni.view === "mc" ? color.amber : color.dim}
                bold={oni.view === "mc"}
              >
                MISSION CONTROL
              </Text>
            </Box>
          </Box>
          {/* Gradient accent line */}
          <Box>
            <Text color={color.coral}>{"██"}</Text>
            <Text color={color.amber}>{"███"}</Text>
            <Text color={color.cyan}>{"███"}</Text>
            <Text color={color.lime}>{"██"}</Text>
            <Text color={color.dim}>
              {"─".repeat(Math.max(0, columns - 10))}
            </Text>
          </Box>
        </>
      )}

      {/* Main content */}
      <Box flexDirection="column" marginTop={oni.view === "boot" ? 0 : 1}>
        {oni.view === "boot" && <BootView width={columns} />}
        {oni.view === "repl" && <REPLView width={columns} />}
        {oni.view === "mc" && <MissionControl width={columns} />}
      </Box>

      {/* Footer */}
      {oni.view !== "boot" && (
        <>
          <HazardDivider width={columns} />
          <Box>
            <Text color={color.dim}>
              <Text color={color.muted}>TAB</Text> switch view {"  "}
              <Text color={color.muted}>ESC</Text> back to REPL {"  "}
              <Text color={color.muted}>:q</Text> quit
            </Text>
          </Box>
        </>
      )}
    </Box>
  );
}

export function App() {
  return (
    <ONIProvider>
      <AppInner />
    </ONIProvider>
  );
}
