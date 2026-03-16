import { useState, useEffect, useCallback } from "react";

export function useTerminalSize() {
  const getSize = useCallback(() => ({
    columns: process.stdout.columns || 80,
    rows: process.stdout.rows || 24,
  }), []);

  const [size, setSize] = useState(getSize);

  useEffect(() => {
    const onResize = () => {
      setSize(getSize());
    };

    // Listen on both stdout and SIGWINCH for maximum compatibility
    process.stdout.on("resize", onResize);
    process.on("SIGWINCH", onResize);

    return () => {
      process.stdout.off("resize", onResize);
      process.off("SIGWINCH", onResize);
    };
  }, [getSize]);

  return size;
}
