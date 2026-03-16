import React, { useCallback } from "react";
import { Box } from "ink";
import { BootSequence, type InitStep } from "../components/index.js";
import { useONI } from "../context/oni-context.js";

interface BootViewProps {
  width: number;
  bootSteps?: InitStep[];
}

export function BootView({ width, bootSteps }: BootViewProps) {
  const { setView } = useONI();

  const handleComplete = useCallback(() => {
    setView("repl");
  }, [setView]);

  return (
    <Box flexDirection="column">
      <BootSequence width={width} onComplete={handleComplete} steps={bootSteps} />
    </Box>
  );
}
