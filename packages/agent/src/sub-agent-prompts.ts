import { buildSystemPrompt } from "./system-prompt.js";

export type SubAgentRole = "planner" | "executor" | "critic";

const PLANNER_ADDENDUM = `
You are the PLANNER sub-agent (Σ). Your job is to decompose the user's mission into concrete subtasks.
Output a numbered plan. Set a tool budget. Flag any ambiguity. Do NOT execute — only plan.
Prefix your output with [Σ PLANNER].`;

const EXECUTOR_ADDENDUM = `
You are the EXECUTOR sub-agent (⚡). Execute the plan provided. Use tools. Write code. Report completed actions.
Do NOT narrate intentions — only report what you did. Prefix output with [⚡ EXECUTOR].`;

const CRITIC_ADDENDUM = `
You are the CRITIC sub-agent (⊘). Review the executor's output against the original mission.
If the output is correct and complete: respond with "ACCEPTED" and a brief summary.
If there are issues: respond with "REJECTED" followed by the specific problems.
Prefix output with [⊘ CRITIC].`;

const ADDENDA: Record<SubAgentRole, string> = {
  planner: PLANNER_ADDENDUM,
  executor: EXECUTOR_ADDENDUM,
  critic: CRITIC_ADDENDUM,
};

export function buildSubAgentPrompt(
  role: SubAgentRole,
  projectDir: string,
): string {
  return buildSystemPrompt(projectDir) + "\n" + ADDENDA[role];
}

export function buildPlannerUserMessage(mission: string): string {
  return `Mission: ${mission}\n\nDecompose this into a numbered plan with a tool budget.`;
}

export function buildExecutorUserMessage(mission: string, plan: string): string {
  return `Mission: ${mission}\n\nPlan to execute:\n${plan}`;
}

export function buildCriticUserMessage(
  mission: string,
  executorOutput: string,
): string {
  return `Original mission: ${mission}\n\nExecutor output:\n${executorOutput}\n\nEvaluate whether the output satisfies the mission. Respond with ACCEPTED or REJECTED.`;
}
