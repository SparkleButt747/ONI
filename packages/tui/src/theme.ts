// ONI Design System — Graphic Realism
// Colour tokens, spacing, and semantic constants

export const color = {
  // Base palette (near-black, not true black)
  black: "#0a0a09",
  panel: "#1a1a18",
  border: "#2a2a27",
  dim: "#3a3a37",
  muted: "#6b6860",
  text: "#c8c5bb",
  white: "#ffffff",

  // Accent palette (semantic)
  amber: "#f5a623",
  cyan: "#00d4c8",
  coral: "#ff4d2e",
  lime: "#b4e033",
  violet: "#7b5ea7",
  warning: "#e8c547",
} as const;

export const subAgent = {
  planner: { prefix: "[Σ]", color: color.violet, label: "PLANNER" },
  executor: { prefix: "[⚡]", color: color.cyan, label: "EXECUTOR" },
  critic: { prefix: "[⊘]", color: color.coral, label: "CRITIC" },
} as const;

export type AgentRole = keyof typeof subAgent;

export type TaskStatus = "RUNNING" | "BLOCKED" | "ERROR" | "DONE" | "IDLE";

export const statusColor: Record<TaskStatus, string> = {
  RUNNING: color.amber,
  BLOCKED: color.coral,
  ERROR: color.coral,
  DONE: color.lime,
  IDLE: color.muted,
};

export type SyncStatus = "LIVE" | "STALE" | "ERROR" | "LOCAL";

export const syncColor: Record<SyncStatus, string> = {
  LIVE: color.lime,
  STALE: color.amber,
  ERROR: color.coral,
  LOCAL: color.muted,
};

export const space = {
  xs: 1,
  sm: 1,
  md: 2,
  lg: 3,
} as const;
