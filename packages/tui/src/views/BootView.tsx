import React, { useCallback } from "react";
import { Box } from "ink";
import { BootSequence } from "../components/index.js";
import { useONI } from "../context/oni-context.js";

interface BootViewProps {
  width: number;
}

export function BootView({ width }: BootViewProps) {
  const { setView } = useONI();

  const handleComplete = useCallback(() => {
    setView("repl");
  }, [setView]);

  return (
    <Box flexDirection="column">
      <BootSequence width={width} onComplete={handleComplete} />
    </Box>
  );
}
