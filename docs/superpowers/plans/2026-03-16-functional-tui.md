# ONI Functional TUI — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform the static TUI demo into a functional, interactive terminal UI matching the oni-terminal-ui.html reference — boot sequence with ASCII logo, working REPL with input handling, interactive Mission Control dashboard, and all UI states (tool proposals, critic veto, blocked, error).

**Architecture:** Ink v6 + React 19 component tree. Root app manages view state (boot → repl → mission-control). Each view is a self-contained component that receives shared state via React context. Simulated agent events drive the UI to demonstrate all states without needing a real backend.

**Tech Stack:** TypeScript strict, ink v6, React 19, chalk v5, tsx for dev runner

---

## File Structure

```
packages/tui/
├── src/
│   ├── theme.ts                    # (exists) colour tokens — UPDATE with refined values from HTML ref
│   ├── context/
│   │   └── oni-context.tsx          # React context: shared app state (conv, tokens, tasks, agent states)
│   ├── hooks/
│   │   ├── use-simulated-events.ts  # Fake event stream to drive demo UI through states
│   │   └── use-terminal-size.ts     # Terminal dimensions hook
│   ├── components/
│   │   ├── BootLogo.tsx             # ASCII "ONI" logo with amber accent
│   │   ├── BootSequence.tsx         # Init checklist with scan-in animation
│   │   ├── StatusTag.tsx            # (exists) — minor update for dot indicator variant
│   │   ├── HazardDivider.tsx        # (exists) — no changes
│   │   ├── ProgressBar.tsx          # (exists) — update layout to match HTML (label/track/value row)
│   │   ├── SectionHeader.tsx        # (exists) — no changes
│   │   ├── ToolCallLine.tsx         # (exists) — update with [tool]/[plugin]/[fail] badge variants
│   │   ├── SubAgentLine.tsx         # (exists) — no changes
│   │   ├── StatBar.tsx              # (exists) — REWRITE to large-number stat cards for MC
│   │   ├── DiffView.tsx             # (exists) — update with file header + accept/reject controls
│   │   ├── TaskQueue.tsx            # (exists) — update with dot indicator + elapsed time
│   │   ├── AgentStatus.tsx          # (exists) — update with bordered per-agent rows
│   │   ├── SyncPanel.tsx            # (exists) — update with pulsing dot
│   │   ├── ToolProposal.tsx         # (exists) — update with bordered box + input handling
│   │   ├── CriticVeto.tsx           # NEW — coral rejection box with replan counter
│   │   ├── BlockedState.tsx         # NEW — warning bordered blocker panel
│   │   ├── ErrorState.tsx           # NEW — tool failure display
│   │   ├── SessionHeader.tsx        # NEW — REPL top bar (conv/model/tokens/burn/sync)
│   │   ├── InputPrompt.tsx          # NEW — "you ›" with text input + cursor
│   │   ├── MessageBubble.tsx        # NEW — "oni ›" response with agent prefix
│   │   └── index.ts                 # barrel export — update
│   ├── views/
│   │   ├── BootView.tsx             # NEW — logo + init sequence → auto-transition to REPL
│   │   ├── REPLView.tsx             # (exists) — REWRITE to functional with input, message history, scrolling
│   │   └── MissionControl.tsx       # (exists) — REWRITE to grid layout matching HTML
│   ├── demo.tsx                     # (exists) — REWRITE: boot → repl → mc flow with keybindings
│   └── app.tsx                      # NEW — root app with context provider + view router
```

---

## Chunk 1: Foundation — Context, Hooks, Theme Update

### Task 1: Update theme with HTML reference values

**Files:**
- Modify: `packages/tui/src/theme.ts`

- [ ] **Step 1: Update colour tokens to match HTML reference**

The HTML ref uses slightly different values for some tokens. Align:

```typescript
// ONI Design System — Graphic Realism
// Colour tokens from oni-terminal-ui.html reference

export const color = {
  // Base palette
  black: "#080807",
  off: "#111110",
  panel: "#191917",
  border: "#252523",
  dim: "#323230",
  muted: "#5a5855",
  text: "#b8b5ac",
  white: "#f0ede6",

  // Accent palette
  amber: "#f5a623",
  cyan: "#00d4c8",
  coral: "#ff4d2e",
  lime: "#b4e033",
  violet: "#7b5ea7",
  warning: "#e8c547",
} as const;
```

- [ ] **Step 2: Verify typecheck passes**

Run: `cd packages/tui && npx tsc --noEmit`
Expected: clean

- [ ] **Step 3: Commit**

```bash
git add packages/tui/src/theme.ts
git commit -m "style: align theme tokens with HTML reference"
```

### Task 2: Create React context for shared app state

**Files:**
- Create: `packages/tui/src/context/oni-context.tsx`

- [ ] **Step 1: Define the ONI app state and context**

```typescript
import React, { createContext, useContext, useState, type ReactNode } from "react";
import type { TaskStatus, AgentRole, SyncStatus } from "../theme.js";

interface ToolCall {
  timestamp: string;
  tool: string;
  args: string;
  latency: string;
  plugin?: string;
  status?: "ok" | "fail";
}

interface Task {
  id: string;
  mission: string;
  status: TaskStatus;
  elapsed?: string;
  blocker?: string;
}

interface Message {
  id: string;
  role: "user" | "oni";
  content: string;
  agent?: AgentRole;
  toolCalls?: ToolCall[];
  diff?: DiffData | null;
}

interface DiffData {
  file: string;
  additions: number;
  deletions: number;
  lines: Array<{
    type: "add" | "remove" | "context";
    content: string;
    lineNum?: number;
  }>;
}

type AgentState = "active" | "idle" | "reviewing";

type ViewState = "boot" | "repl" | "mc";

interface ONIState {
  view: ViewState;
  setView: (v: ViewState) => void;
  convId: string;
  model: string;
  version: string;
  tokens: number;
  maxTokens: number;
  burnRate: number;
  syncStatus: SyncStatus;
  agentStates: Record<AgentRole, AgentState>;
  setAgentStates: (s: Record<AgentRole, AgentState>) => void;
  tasks: Task[];
  setTasks: (t: Task[]) => void;
  toolLog: ToolCall[];
  addToolCall: (t: ToolCall) => void;
  messages: Message[];
  addMessage: (m: Message) => void;
  activeDiff: DiffData | null;
  setActiveDiff: (d: DiffData | null) => void;
  setTokens: (n: number) => void;
  setBurnRate: (n: number) => void;
  setSyncStatus: (s: SyncStatus) => void;
}

const ONIContext = createContext<ONIState | null>(null);

export function useONI(): ONIState {
  const ctx = useContext(ONIContext);
  if (!ctx) throw new Error("useONI must be used within ONIProvider");
  return ctx;
}

export function ONIProvider({ children }: { children: ReactNode }) {
  const [view, setView] = useState<ViewState>("boot");
  const [tokens, setTokens] = useState(0);
  const [burnRate, setBurnRate] = useState(0);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>("LOCAL");
  const [agentStates, setAgentStates] = useState<Record<AgentRole, AgentState>>({
    planner: "idle",
    executor: "idle",
    critic: "idle",
  });
  const [tasks, setTasks] = useState<Task[]>([]);
  const [toolLog, setToolLog] = useState<ToolCall[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [activeDiff, setActiveDiff] = useState<DiffData | null>(null);

  const addToolCall = (t: ToolCall) => setToolLog((prev) => [...prev, t]);
  const addMessage = (m: Message) => setMessages((prev) => [...prev, m]);

  return (
    <ONIContext.Provider
      value={{
        view, setView,
        convId: "8fk2a9",
        model: "claude-sonnet-4-6",
        version: "0.1.0",
        tokens, maxTokens: 200000,
        burnRate,
        syncStatus,
        agentStates, setAgentStates,
        tasks, setTasks,
        toolLog, addToolCall,
        messages, addMessage,
        activeDiff, setActiveDiff,
        setTokens, setBurnRate, setSyncStatus,
      }}
    >
      {children}
    </ONIContext.Provider>
  );
}

export type { ToolCall, Task, Message, DiffData, AgentState, ViewState };
```

- [ ] **Step 2: Verify typecheck**

Run: `npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add packages/tui/src/context/
git commit -m "feat: add ONI shared state context"
```

### Task 3: Terminal size hook

**Files:**
- Create: `packages/tui/src/hooks/use-terminal-size.ts`

- [ ] **Step 1: Write the hook**

```typescript
import { useState, useEffect } from "react";

export function useTerminalSize() {
  const [size, setSize] = useState({
    columns: process.stdout.columns || 80,
    rows: process.stdout.rows || 24,
  });

  useEffect(() => {
    const onResize = () => {
      setSize({
        columns: process.stdout.columns || 80,
        rows: process.stdout.rows || 24,
      });
    };
    process.stdout.on("resize", onResize);
    return () => { process.stdout.off("resize", onResize); };
  }, []);

  return size;
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/tui/src/hooks/
git commit -m "feat: add terminal size hook"
```

### Task 4: Simulated event stream hook

**Files:**
- Create: `packages/tui/src/hooks/use-simulated-events.ts`

- [ ] **Step 1: Write the simulation hook**

This hook drives the demo by emitting timed events that update the shared state — simulating what the real agent core would do. It runs the boot sequence init checks, then populates the REPL with a demo conversation.

```typescript
import { useEffect, useRef } from "react";
import { useONI } from "../context/oni-context.js";
import type { Task, ToolCall, Message, DiffData } from "../context/oni-context.js";

export function useSimulatedEvents() {
  const oni = useONI();
  const hasRun = useRef(false);

  useEffect(() => {
    if (hasRun.current) return;
    hasRun.current = true;

    const timers: ReturnType<typeof setTimeout>[] = [];
    const at = (ms: number, fn: () => void) => {
      timers.push(setTimeout(fn, ms));
    };

    // Boot → REPL transition
    at(3000, () => {
      oni.setView("repl");
      oni.setSyncStatus("LIVE");
      oni.setTokens(0);
    });

    // Simulate a conversation after boot
    at(3500, () => {
      oni.addMessage({
        id: "m1",
        role: "user",
        content: "the order total is wrong for discount codes. look at the pricing engine and fix it",
      });
      oni.setAgentStates({ planner: "active", executor: "idle", critic: "idle" });
    });

    at(4200, () => {
      oni.setAgentStates({ planner: "idle", executor: "active", critic: "idle" });
      oni.addToolCall({ timestamp: "14:22:01", tool: "read_file", args: "src/services/PricingEngine.ts", latency: "9ms" });
      oni.setTokens(8400);
    });

    at(4800, () => {
      oni.addToolCall({ timestamp: "14:22:01", tool: "read_file", args: "src/services/OrderService.ts:processTotal", latency: "7ms" });
      oni.setTokens(14200);
    });

    at(5400, () => {
      oni.addToolCall({ timestamp: "14:22:04", tool: "bash", args: "npx jest PricingEngine --no-coverage 2>&1 | tail -20", latency: "1.4s" });
      oni.setTokens(22800);
      oni.setBurnRate(1842);
    });

    at(6200, () => {
      const diff: DiffData = {
        file: "src/services/OrderService.ts",
        additions: 2,
        deletions: 2,
        lines: [
          { type: "context", content: "  calculateSubtotal(items)", lineNum: 44 },
          { type: "remove", content: "  applyDiscount(calculateTax(subtotal), code)", lineNum: 45 },
          { type: "add", content: "  const discounted = applyDiscount(subtotal, code)", lineNum: 45 },
          { type: "add", content: "  calculateTax(discounted)", lineNum: 46 },
          { type: "context", content: "  return total", lineNum: 47 },
        ],
      };

      oni.addMessage({
        id: "m2",
        role: "oni",
        agent: "executor",
        content: "applyDiscount() runs after calculateTax() — discounts reduce post-tax total instead of pre-tax subtotal. Off-by-tax-rate on every discounted order. Swapping call order in processTotal().",
        diff,
      });
      oni.setActiveDiff(diff);
      oni.setTokens(35400);
    });

    at(7500, () => {
      oni.setAgentStates({ planner: "idle", executor: "idle", critic: "active" });
    });

    at(8200, () => {
      oni.addMessage({
        id: "m3",
        role: "oni",
        agent: "critic",
        content: "output accepted · no regressions · tests cover this path · clean.",
      });
      oni.setAgentStates({ planner: "idle", executor: "idle", critic: "idle" });
      oni.setTokens(47200);
    });

    // Populate tasks for MC view
    at(3200, () => {
      oni.setTasks([
        { id: "a3f82e", mission: "Refactor auth middleware", status: "RUNNING", elapsed: "2m 14s" },
        { id: "b7c1d4", mission: "Write unit tests — UserService", status: "RUNNING", elapsed: "0m 47s" },
        { id: "c9d0e1", mission: "Generate OpenAPI schema", status: "RUNNING", elapsed: "0m 08s" },
        { id: "d4e5f6", mission: "Deploy staging — awaiting CI", status: "BLOCKED", blocker: "requires approval" },
        { id: "e2f3a4", mission: "Lint fix — tsconfig paths", status: "ERROR" },
        { id: "f1a2b3", mission: "Scaffold Express router", status: "DONE", elapsed: "4m 02s" },
      ]);
    });

    return () => timers.forEach(clearTimeout);
  }, []);
}
```

- [ ] **Step 2: Verify typecheck**

Run: `npx tsc --noEmit`

- [ ] **Step 3: Commit**

```bash
git add packages/tui/src/hooks/
git commit -m "feat: add simulated event stream for demo"
```

---

## Chunk 2: New Components — Boot, Input, Messages, States

### Task 5: Boot logo + boot sequence

**Files:**
- Create: `packages/tui/src/components/BootLogo.tsx`
- Create: `packages/tui/src/components/BootSequence.tsx`

- [ ] **Step 1: Write BootLogo**

ASCII art rendering of "ONI" with amber "I", matching the HTML ref (Barlow Condensed 42px → large block text in terminal).

```tsx
import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

export function BootLogo() {
  return (
    <Box flexDirection="row" gap={2}>
      <Box flexDirection="column">
        <Text color={color.white} bold>
          {"  ██████╗  ███╗   ██╗"}
        </Text>
        <Text color={color.white} bold>
          {" ██╔═══██╗ ████╗  ██║"}
        </Text>
        <Text color={color.white} bold>
          {" ██║   ██║ ██╔██╗ ██║"}
        </Text>
        <Text color={color.white} bold>
          {" ██║   ██║ ██║╚██╗██║"}
        </Text>
        <Text color={color.white} bold>
          {" ╚██████╔╝ ██║ ╚████║"}
        </Text>
        <Text color={color.white} bold>
          {"  ╚═════╝  ╚═╝  ╚═══╝"}
        </Text>
      </Box>
      <Box flexDirection="column">
        <Text color={color.amber} bold>
          {" ██╗"}
        </Text>
        <Text color={color.amber} bold>
          {" ██║"}
        </Text>
        <Text color={color.amber} bold>
          {" ██║"}
        </Text>
        <Text color={color.amber} bold>
          {" ██║"}
        </Text>
        <Text color={color.amber} bold>
          {" ██║"}
        </Text>
        <Text color={color.amber} bold>
          {" ╚═╝"}
        </Text>
      </Box>
    </Box>
  );
}
```

- [ ] **Step 2: Write BootSequence**

Init checklist with staggered appearance (uses useState + setTimeout for scan-in effect).

```tsx
import React, { useState, useEffect } from "react";
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

  useEffect(() => {
    const timers: ReturnType<typeof setTimeout>[] = [];
    INIT_STEPS.forEach((_, i) => {
      timers.push(setTimeout(() => setVisibleSteps(i + 1), 400 + i * 300));
    });
    timers.push(setTimeout(() => setShowHints(true), 400 + INIT_STEPS.length * 300 + 200));
    timers.push(setTimeout(onComplete, 400 + INIT_STEPS.length * 300 + 1200));
    return () => timers.forEach(clearTimeout);
  }, [onComplete]);

  return (
    <Box flexDirection="column" width={width}>
      <Box flexDirection="row" gap={2} marginBottom={1}>
        <BootLogo />
        <Box flexDirection="column" justifyContent="flex-end" borderLeft borderColor={color.border} paddingLeft={2}>
          <Text color={color.muted}>ONBOARD NEURAL INTELLIGENCE</Text>
          <Text color={color.text}>
            v0.1.0 · <Text color={color.cyan}>claude-sonnet-4-6</Text> · non-commercial
          </Text>
        </Box>
      </Box>

      <HazardDivider width={width} />

      <Box flexDirection="column" marginTop={1}>
        {INIT_STEPS.slice(0, visibleSteps).map((step, i) => (
          <Text key={i}>
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
```

- [ ] **Step 3: Verify typecheck + test render**

Run: `npx tsc --noEmit`

- [ ] **Step 4: Commit**

```bash
git add packages/tui/src/components/BootLogo.tsx packages/tui/src/components/BootSequence.tsx
git commit -m "feat: add boot logo and sequence components"
```

### Task 6: Input prompt with text handling

**Files:**
- Create: `packages/tui/src/components/InputPrompt.tsx`

- [ ] **Step 1: Write InputPrompt with useInput**

```tsx
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
      <Text color={color.amber} bold>
        {"you › "}
      </Text>
      <Text color={color.white}>{input}</Text>
      <Text color={color.amber}>{"█"}</Text>
    </Box>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/tui/src/components/InputPrompt.tsx
git commit -m "feat: add input prompt with text handling"
```

### Task 7: Session header bar

**Files:**
- Create: `packages/tui/src/components/SessionHeader.tsx`

- [ ] **Step 1: Write SessionHeader matching HTML ref**

```tsx
import React from "react";
import { Box, Text } from "ink";
import { color, syncColor, type SyncStatus } from "../theme.js";

interface SessionHeaderProps {
  convId: string;
  model: string;
  tokens: number;
  maxTokens: number;
  burnRate: number;
  syncStatus: SyncStatus;
}

export function SessionHeader({
  convId,
  model,
  tokens,
  maxTokens,
  burnRate,
  syncStatus,
}: SessionHeaderProps) {
  const tokStr = tokens >= 1000 ? `${(tokens / 1000).toFixed(1)}k` : `${tokens}`;
  const maxStr = `${(maxTokens / 1000).toFixed(0)}k`;
  let burnColor = color.muted;
  if (burnRate > 5000) burnColor = color.coral;
  else if (burnRate > 2000) burnColor = color.warning;

  return (
    <Box borderBottom borderColor={color.border} paddingBottom={1} gap={1}>
      <Text color={color.muted}>{convId}</Text>
      <Text color={color.dim}>·</Text>
      <Text color={color.cyan}>{model}</Text>
      <Text color={color.dim}>·</Text>
      <Text color={color.text}>
        {tokStr} <Text color={color.muted}>/ {maxStr} tok</Text>
      </Text>
      <Text color={color.dim}>·</Text>
      <Text color={burnColor}>
        {burnRate > 0 ? `${(burnRate / 1000).toFixed(1)}k tok/min` : "—"}
      </Text>
      <Box flexGrow={1} />
      <Text color={syncColor[syncStatus]}>
        {syncStatus === "LIVE" ? "● " : ""}
        {syncStatus}
      </Text>
    </Box>
  );
}
```

- [ ] **Step 2: Commit**

```bash
git add packages/tui/src/components/SessionHeader.tsx
git commit -m "feat: add REPL session header bar"
```

### Task 8: Message bubble + Critic veto + Blocked + Error components

**Files:**
- Create: `packages/tui/src/components/MessageBubble.tsx`
- Create: `packages/tui/src/components/CriticVeto.tsx`
- Create: `packages/tui/src/components/BlockedState.tsx`
- Create: `packages/tui/src/components/ErrorState.tsx`

- [ ] **Step 1: Write MessageBubble**

```tsx
import React from "react";
import { Box, Text } from "ink";
import { color, subAgent, type AgentRole } from "../theme.js";

interface MessageBubbleProps {
  role: "user" | "oni";
  content: string;
  agent?: AgentRole;
}

export function MessageBubble({ role, content, agent }: MessageBubbleProps) {
  if (role === "user") {
    return (
      <Box>
        <Text color={color.amber} bold>{"you › "}</Text>
        <Text color={color.white}>{content}</Text>
      </Box>
    );
  }

  const prefix = agent ? subAgent[agent] : null;

  return (
    <Box flexDirection="column">
      <Box>
        <Text color={color.amber}>{"oni › "}</Text>
        {prefix && (
          <Text color={prefix.color} bold>{prefix.prefix} </Text>
        )}
      </Box>
      <Text color={color.text}>{content}</Text>
    </Box>
  );
}
```

- [ ] **Step 2: Write CriticVeto**

```tsx
import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface CriticVetoProps {
  reason: string;
  replanNum: number;
  maxReplans: number;
}

export function CriticVeto({ reason, replanNum, maxReplans }: CriticVetoProps) {
  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={color.coral}
      paddingX={1}
      paddingY={1}
    >
      <Box gap={1} marginBottom={1}>
        <Text color={color.coral} bold>{"[⊘]"}</Text>
        <Text color={color.coral} bold>REJECTED</Text>
      </Box>
      <Text color={color.text}>{reason}</Text>
      <Box marginTop={1}>
        <Text color={color.coral}>
          Replan? <Text color={color.amber}>[y]</Text>
          <Text color={color.muted}> / n · replan {replanNum} of {maxReplans}</Text>
        </Text>
      </Box>
    </Box>
  );
}
```

- [ ] **Step 3: Write BlockedState**

```tsx
import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface BlockedStateProps {
  reason: string;
  detail?: string;
  link?: string;
}

export function BlockedState({ reason, detail, link }: BlockedStateProps) {
  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={color.warning}
      paddingX={1}
      paddingY={1}
    >
      <Box gap={1} marginBottom={1}>
        <Text color={color.warning} bold>BLOCKED</Text>
        <Text color={color.warning}>{reason}</Text>
      </Box>
      {detail && <Text color={color.text}>{detail}</Text>}
      {link && (
        <Box marginTop={1}>
          <Text color={color.muted}>
            {"→ "}<Text color={color.cyan}>{link}</Text>
          </Text>
        </Box>
      )}
    </Box>
  );
}
```

- [ ] **Step 4: Write ErrorState**

```tsx
import React from "react";
import { Box, Text } from "ink";
import { color } from "../theme.js";

interface ErrorStateProps {
  tool: string;
  args: string;
  error: string;
  suggestion?: string;
}

export function ErrorState({ tool, args, error, suggestion }: ErrorStateProps) {
  return (
    <Box flexDirection="column">
      <Box gap={1}>
        <Text color={color.coral}>{" FAIL "}</Text>
        <Text color={color.coral}>{tool}</Text>
        <Text color={color.muted}>{args}</Text>
        <Box flexGrow={1} />
        <Text color={color.coral}>ERR</Text>
      </Box>
      <Box
        borderLeft
        borderColor={color.coral}
        paddingLeft={1}
        marginTop={1}
      >
        <Text color={color.coral}>{error}</Text>
      </Box>
      {suggestion && (
        <Box marginTop={1}>
          <Text color={color.muted}>
            Run <Text color={color.amber}>{suggestion}</Text> then retry.
          </Text>
        </Box>
      )}
    </Box>
  );
}
```

- [ ] **Step 5: Verify typecheck**

Run: `npx tsc --noEmit`

- [ ] **Step 6: Commit**

```bash
git add packages/tui/src/components/MessageBubble.tsx packages/tui/src/components/CriticVeto.tsx packages/tui/src/components/BlockedState.tsx packages/tui/src/components/ErrorState.tsx
git commit -m "feat: add message, critic veto, blocked, and error state components"
```

---

## Chunk 3: Update Existing Components + Barrel Export

### Task 9: Update existing components to match HTML reference

**Files:**
- Modify: `packages/tui/src/components/ToolCallLine.tsx` — add badge variant (tool/plugin/fail)
- Modify: `packages/tui/src/components/ProgressBar.tsx` — horizontal label/track/value layout
- Modify: `packages/tui/src/components/TaskQueue.tsx` — dot indicator + elapsed time
- Modify: `packages/tui/src/components/AgentStatus.tsx` — bordered row per agent
- Modify: `packages/tui/src/components/DiffView.tsx` — file header with +N −N, accept/reject hint
- Modify: `packages/tui/src/components/index.ts` — add all new exports

- [ ] **Step 1: Update ToolCallLine with badge variants**

Add `status` prop: `"ok" | "fail" | "plugin"` — defaults to `"ok"`. Plugin badge in violet, fail badge in coral.

- [ ] **Step 2: Update ProgressBar to horizontal layout**

Match HTML: `LABEL ███████████░░░░░░░ 24%` — all on one row.

- [ ] **Step 3: Update TaskQueue with dot + elapsed**

Match HTML: coloured dot (6px equivalent `●`), mission text, status tag, elapsed time.

- [ ] **Step 4: Update AgentStatus with bordered rows**

Each agent in its own bordered box with colour tint, matching HTML ref.

- [ ] **Step 5: Update DiffView with file header + counts**

Show `src/file.ts · +3 −2` header, accept/reject hint at bottom.

- [ ] **Step 6: Update barrel export**

Add: `BootLogo`, `BootSequence`, `InputPrompt`, `SessionHeader`, `MessageBubble`, `CriticVeto`, `BlockedState`, `ErrorState`.

- [ ] **Step 7: Verify typecheck**

Run: `npx tsc --noEmit`

- [ ] **Step 8: Commit**

```bash
git add packages/tui/src/components/
git commit -m "refactor: update components to match HTML reference"
```

---

## Chunk 4: Views — Boot, REPL, Mission Control

### Task 10: Boot view

**Files:**
- Create: `packages/tui/src/views/BootView.tsx`

- [ ] **Step 1: Write BootView**

Wraps BootSequence, on complete transitions to REPL via context.

- [ ] **Step 2: Commit**

### Task 11: Rewrite REPL view — functional with message history

**Files:**
- Modify: `packages/tui/src/views/REPLView.tsx` — full rewrite

- [ ] **Step 1: Write functional REPL**

SessionHeader at top. Scrollable message history from context. Each message renders with MessageBubble + inline tool calls + inline diffs. InputPrompt at bottom. `:q` exits, `:mc` switches to MC. User messages added to context, which triggers simulated responses.

- [ ] **Step 2: Commit**

### Task 12: Rewrite Mission Control — grid layout

**Files:**
- Modify: `packages/tui/src/views/MissionControl.tsx` — full rewrite

- [ ] **Step 1: Write MC with grid layout matching HTML**

Top: 4 stat cards in a row (running tasks / tokens / burn rate / tool calls) — large numbers.
Middle: 2-column layout — left = task queue, right = sync + context bars + sub-agents.
Bottom: tool log + active diff with accept/reject.
All data from context. `:q` exits, `ESC` returns to REPL.

- [ ] **Step 2: Commit**

---

## Chunk 5: Root App + Demo Entry Point

### Task 13: Root app with context + view router

**Files:**
- Create: `packages/tui/src/app.tsx`
- Modify: `packages/tui/src/demo.tsx` — simplify to render App

- [ ] **Step 1: Write app.tsx**

ONIProvider wraps everything. View router switches on `view` state. Global keybindings: `q`/`Ctrl+C` = exit, `Tab` = toggle REPL/MC. Simulated events hook runs on mount.

- [ ] **Step 2: Rewrite demo.tsx**

Just `render(<App />, { exitOnCtrlC: true })`.

- [ ] **Step 3: Verify full demo runs**

Run: `cd packages/tui && npm run demo`
Expected: Boot logo appears → init checklist scans in → transitions to REPL → conversation plays out with tool calls, diff, critic verdict. Tab switches to Mission Control with live data.

- [ ] **Step 4: Commit**

```bash
git add packages/tui/src/
git commit -m "feat: functional TUI with boot, REPL, and Mission Control"
```

---

## Execution Notes

- Each chunk is independently testable — typecheck after every task
- The simulated events hook replaces what the real agent core will provide later
- All components are designed for real data — the context interface matches `API_CONTRACTS.md`
- The boot sequence timings can be tuned after visual review
- Components use `borderStyle="single"` for ink borders — zero border-radius by design (matching Graphic Realism)
