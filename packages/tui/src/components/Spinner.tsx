import React, { useState, useEffect } from "react";
import { Text } from "ink";
import { color } from "../theme.js";

const FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

interface SpinnerProps {
  label?: string;
  accentColor?: string;
}

export function Spinner({ label, accentColor = color.lime }: SpinnerProps) {
  const [frame, setFrame] = useState(0);

  useEffect(() => {
    const timer = setInterval(() => {
      setFrame((f) => (f + 1) % FRAMES.length);
    }, 80);
    return () => clearInterval(timer);
  }, []);

  return (
    <Text>
      <Text color={accentColor}>{FRAMES[frame]}</Text>
      {label && <Text color={color.muted}>{` ${label}`}</Text>}
    </Text>
  );
}
