import React, { useState } from "react";
import { render, Box, Text, useInput, useApp } from "ink";
import { color } from "./theme.js";
import { HazardDivider } from "./components/index.js";
import { MissionControl } from "./views/MissionControl.js";
import { REPLView } from "./views/REPLView.js";

type View = "repl" | "mc";

function App() {
  const [view, setView] = useState<View>("repl");
  const { exit } = useApp();
  const width = process.stdout.columns || 80;

  useInput((input, key) => {
    if (input === "q" || (key.ctrl && input === "c")) {
      exit();
    }
    if (key.tab) {
      setView((v) => (v === "repl" ? "mc" : "repl"));
    }
  });

  return (
    <Box flexDirection="column" width={width}>
      {/* Top bar */}
      <Box>
        <Text color={color.amber} bold>
          {"ONI "}
        </Text>
        <Text color={color.muted}>ONBOARD NEURAL INTELLIGENCE</Text>
        <Box flexGrow={1} />
        <Box gap={2}>
          <Text
            color={view === "repl" ? color.amber : color.dim}
            bold={view === "repl"}
          >
            REPL
          </Text>
          <Text
            color={view === "mc" ? color.amber : color.dim}
            bold={view === "mc"}
          >
            MISSION CONTROL
          </Text>
        </Box>
      </Box>

      {/* Gradient line */}
      <Box>
        <Text color={color.coral}>{"██"}</Text>
        <Text color={color.amber}>{"███"}</Text>
        <Text color={color.cyan}>{"███"}</Text>
        <Text color={color.lime}>{"██"}</Text>
        <Text color={color.dim}>
          {"─".repeat(Math.max(0, width - 10))}
        </Text>
      </Box>

      {/* Main content */}
      <Box flexDirection="column" marginTop={1}>
        {view === "repl" ? (
          <REPLView width={width} />
        ) : (
          <MissionControl width={width} />
        )}
      </Box>

      {/* Footer */}
      <HazardDivider width={width} />
      <Box>
        <Text color={color.dim}>
          <Text color={color.muted}>TAB</Text> switch view  {" "}
          <Text color={color.muted}>Q</Text> quit
        </Text>
      </Box>
    </Box>
  );
}

render(<App />);
