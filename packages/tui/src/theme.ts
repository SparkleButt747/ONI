// ONI Design System — Graphic Realism v3
// Deep navy + neon lime + electric blue — Marathon-inspired

export const color = {
  // Base palette (deep navy, not warm black)
  void: "#05050a",
  ink: "#080810",
  panel: "#0c0c18",
  lift: "#121224",
  edge: "#1a1a38",
  wire: "#222244",
  stone: "#3a3a6a",
  ash: "#6060aa",
  fog: "#8888cc",
  bone: "#ccccff",

  // Accent palette
  lime: "#8fff00",
  limeD: "#5aaa00",
  blue: "#1a4dff",
  blueB: "#0033cc",
  coral: "#ff3060",
  amber: "#f5a623",
  cyan: "#00d4c8",
  violet: "#7b5ea7",
  gold: "#e8ea00",

  // Aliases for backward compat (used by components)
  black: "#05050a",
  off: "#080810",
  border: "#1a1a38",
  dim: "#3a3a6a",     // was #222244 — too dark for text, bumped to stone
  muted: "#6060aa",
  text: "#8888cc",
  white: "#ccccff",
  warning: "#f5a623",
} as const;

export const subAgent = {
  planner: { prefix: "[Σ PLANNER]", color: color.violet, label: "PLANNER" },
  executor: { prefix: "[⚡ EXECUTOR]", color: color.cyan, label: "EXECUTOR" },
  critic: { prefix: "[⊘ CRITIC]", color: color.coral, label: "CRITIC" },
} as const;

export type AgentRole = keyof typeof subAgent;

export type TaskStatus = "RUNNING" | "BLOCKED" | "ERROR" | "DONE" | "IDLE";

export const statusColor: Record<TaskStatus, string> = {
  RUNNING: color.lime,
  BLOCKED: color.amber,
  ERROR: color.coral,
  DONE: color.stone,
  IDLE: color.wire,
};

export type SyncStatus = "LIVE" | "STALE" | "ERROR" | "LOCAL";

export const syncColor: Record<SyncStatus, string> = {
  LIVE: color.lime,
  STALE: color.amber,
  ERROR: color.coral,
  LOCAL: color.stone,
};

// Chroma stripe segments (left to right)
export const CHROMA = [
  color.coral,
  color.amber,
  color.gold,
  color.lime,
  color.cyan,
  color.blue,
  color.coral,
] as const;
