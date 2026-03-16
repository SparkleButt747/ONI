import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";

const BUDGET_FILE = join(homedir(), ".local", "share", "oni", "budget.json");

export interface BudgetConfig {
  sessionLimit: number; // max tokens per session (0 = unlimited)
  monthlyLimit: number; // max tokens per calendar month (0 = unlimited)
}

export interface BudgetState {
  sessionTokens: number;
  monthTokens: number;
  monthKey: string; // "YYYY-MM" for tracking monthly usage
}

const DEFAULT_CONFIG: BudgetConfig = {
  sessionLimit: 0,
  monthlyLimit: 0,
};

function currentMonthKey(): string {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}`;
}

function loadBudgetState(): BudgetState {
  try {
    if (existsSync(BUDGET_FILE)) {
      const data = JSON.parse(readFileSync(BUDGET_FILE, "utf-8"));
      // Reset if month changed
      if (data.monthKey !== currentMonthKey()) {
        return { sessionTokens: 0, monthTokens: 0, monthKey: currentMonthKey() };
      }
      return data;
    }
  } catch {
    // fresh state
  }
  return { sessionTokens: 0, monthTokens: 0, monthKey: currentMonthKey() };
}

function saveBudgetState(state: BudgetState): void {
  const dir = join(homedir(), ".local", "share", "oni");
  mkdirSync(dir, { recursive: true });
  writeFileSync(BUDGET_FILE, JSON.stringify(state, null, 2) + "\n");
}

export class BudgetTracker {
  private config: BudgetConfig;
  private state: BudgetState;

  constructor(config?: Partial<BudgetConfig>) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.state = loadBudgetState();
    // Reset session tokens on new tracker (new session)
    this.state.sessionTokens = 0;
  }

  /** Record tokens used. Returns false if budget exceeded. */
  record(tokens: number): boolean {
    this.state.sessionTokens += tokens;
    this.state.monthTokens += tokens;
    saveBudgetState(this.state);

    if (this.config.sessionLimit > 0 && this.state.sessionTokens > this.config.sessionLimit) {
      return false;
    }
    if (this.config.monthlyLimit > 0 && this.state.monthTokens > this.config.monthlyLimit) {
      return false;
    }
    return true;
  }

  /** Check if a request of given size would exceed budget */
  canSpend(estimatedTokens: number): boolean {
    if (
      this.config.sessionLimit > 0 &&
      this.state.sessionTokens + estimatedTokens > this.config.sessionLimit
    ) {
      return false;
    }
    if (
      this.config.monthlyLimit > 0 &&
      this.state.monthTokens + estimatedTokens > this.config.monthlyLimit
    ) {
      return false;
    }
    return true;
  }

  get sessionUsed(): number {
    return this.state.sessionTokens;
  }

  get monthUsed(): number {
    return this.state.monthTokens;
  }

  get sessionRemaining(): number | null {
    if (this.config.sessionLimit === 0) return null;
    return Math.max(0, this.config.sessionLimit - this.state.sessionTokens);
  }

  get monthRemaining(): number | null {
    if (this.config.monthlyLimit === 0) return null;
    return Math.max(0, this.config.monthlyLimit - this.state.monthTokens);
  }

  summary(): string {
    const parts: string[] = [];
    parts.push(`Session: ${this.state.sessionTokens} tok`);
    if (this.config.sessionLimit > 0) {
      parts[0] += ` / ${this.config.sessionLimit} limit`;
    }
    parts.push(`Month (${this.state.monthKey}): ${this.state.monthTokens} tok`);
    if (this.config.monthlyLimit > 0) {
      parts[1] += ` / ${this.config.monthlyLimit} limit`;
    }
    return parts.join("\n");
  }
}
