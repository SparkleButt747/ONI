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
  BLOCKED: color.warning,
  ERROR: color.coral,
  DONE: color.lime,
  IDLE: color.muted,
};

export type SyncStatus = "LIVE" | "STALE" | "ERROR" | "LOCAL";

export const syncColor: Record<SyncStatus, string> = {
  LIVE: color.lime,
  STALE: color.warning,
  ERROR: color.coral,
  LOCAL: color.muted,
};
