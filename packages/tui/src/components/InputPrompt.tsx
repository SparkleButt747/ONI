import React, { useState } from "react";
import { Box, Text, useInput } from "ink";
import { color } from "../theme.js";

interface InputPromptProps {
  onSubmit: (text: string) => void;
  isActive?: boolean;
}

export function InputPrompt({ onSubmit, isActive = true }: InputPromptProps) {
  const [input, setInput] = useState("");

  useInput(
    (ch, key) => {
      if (!isActive) return;
      if (key.return) {
        if (input.trim()) {
          onSubmit(input.trim());
          setInput("");
        }
        return;
      }
      if (key.backspace || key.delete) {
        setInput((prev) => prev.slice(0, -1));
        return;
      }
      if (ch && !key.ctrl && !key.meta) {
        setInput((prev) => prev + ch);
      }
    },
    { isActive },
  );

  return (
    <Box>
      <Text color={color.lime} bold>
        {"YOU › "}
      </Text>
      <Text color={color.white}>{input}</Text>
      <Text color={color.lime}>{"█"}</Text>
    </Box>
  );
}
