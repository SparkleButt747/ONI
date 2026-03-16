import type Database from "better-sqlite3";
import { createPrefsSchema } from "./schema.js";

export type Outcome = "accepted" | "rejected" | "modified" | "auto" | "always";

export interface ToolEvent {
  sessionId: string;
  toolName: string;
  intentKey: string;
  outcome: Outcome;
  latencyMs?: number;
}

export interface LearnedRule {
  id: number;
  condition: { intent: string; tool?: string };
  action: string;
  confidence: number;
  nObs: number;
  active: boolean;
}

const DECAY_RATE = 0.97; // Per day

export class PreferenceEngine {
  private db: Database.Database;

  constructor(db: Database.Database) {
    this.db = db;
    createPrefsSchema(db);
  }

  /** Record a tool event and update preference weight */
  record(event: ToolEvent): void {
    // Insert event
    this.db
      .prepare(
        "INSERT INTO tool_events (session_id, tool_name, intent_key, outcome, latency_ms, ts) VALUES (?, ?, ?, ?, ?, ?)",
      )
      .run(
        event.sessionId,
        event.toolName,
        event.intentKey,
        event.outcome,
        event.latencyMs ?? null,
        Date.now(),
      );

    // Update or insert preference weight
    const weightDelta = this.outcomeToDelta(event.outcome);
    const existing = this.db
      .prepare("SELECT weight, n_obs FROM preferences WHERE tool_name = ? AND intent_key = ?")
      .get(event.toolName, event.intentKey) as
      | { weight: number; n_obs: number }
      | undefined;

    if (existing) {
      const newWeight = Math.max(0, Math.min(1, existing.weight + weightDelta / (existing.n_obs + 1)));
      const finalWeight = event.outcome === "always" ? 1.0 : newWeight;
      this.db
        .prepare(
          "UPDATE preferences SET weight = ?, n_obs = n_obs + 1, last_updated = ? WHERE tool_name = ? AND intent_key = ?",
        )
        .run(finalWeight, Date.now(), event.toolName, event.intentKey);
    } else {
      const initialWeight = event.outcome === "always" ? 1.0 : 0.5 + weightDelta;
      this.db
        .prepare(
          "INSERT INTO preferences (tool_name, intent_key, weight, n_obs, last_updated) VALUES (?, ?, ?, 1, ?)",
        )
        .run(event.toolName, event.intentKey, initialWeight, Date.now());
    }
  }

  /** Get the current score for a tool+intent, applying time decay */
  score(toolName: string, intentKey: string): number {
    const row = this.db
      .prepare("SELECT weight, last_updated FROM preferences WHERE tool_name = ? AND intent_key = ?")
      .get(toolName, intentKey) as
      | { weight: number; last_updated: number }
      | undefined;

    if (!row) return 0.5; // Default — propose range

    const daysSince = (Date.now() - row.last_updated) / (1000 * 60 * 60 * 24);
    const decayed = row.weight * Math.pow(DECAY_RATE, daysSince);
    return Math.max(0, Math.min(1, decayed));
  }

  /** Should this tool be auto-used (>= 0.85), proposed (0.5-0.85), or omitted (< 0.5)? */
  decision(toolName: string, intentKey: string): "auto" | "propose" | "omit" {
    const s = this.score(toolName, intentKey);
    if (s >= 0.85) return "auto";
    if (s >= 0.5) return "propose";
    return "omit";
  }

  /** Crystallise rules: tools with high confidence and enough observations */
  crystallise(): LearnedRule[] {
    const rows = this.db
      .prepare(
        "SELECT tool_name, intent_key, weight, n_obs FROM preferences WHERE weight > 0.85 AND n_obs >= 10",
      )
      .all() as Array<{ tool_name: string; intent_key: string; weight: number; n_obs: number }>;

    const rules: LearnedRule[] = [];

    for (const row of rows) {
      // Check if rule already exists
      const existing = this.db
        .prepare(
          "SELECT id FROM learned_rules WHERE condition_json LIKE ? AND active = 1",
        )
        .get(`%"tool":"${row.tool_name}"%`) as { id: number } | undefined;

      if (existing) continue;

      const condition = { intent: row.intent_key, tool: row.tool_name };
      const action = `Auto-use ${row.tool_name} for ${row.intent_key} tasks (${row.n_obs} observations, ${Math.round(row.weight * 100)}% confidence)`;

      this.db
        .prepare(
          "INSERT INTO learned_rules (condition_json, action, confidence, n_obs, active, created_at) VALUES (?, ?, ?, ?, 1, ?)",
        )
        .run(JSON.stringify(condition), action, row.weight, row.n_obs, Date.now());

      rules.push({
        id: 0,
        condition,
        action,
        confidence: row.weight,
        nObs: row.n_obs,
        active: true,
      });
    }

    return rules;
  }

  /** Get all active learned rules for system prompt injection */
  activeRules(): LearnedRule[] {
    const rows = this.db
      .prepare("SELECT * FROM learned_rules WHERE active = 1")
      .all() as Array<{
      id: number;
      condition_json: string;
      action: string;
      confidence: number;
      n_obs: number;
      active: number;
    }>;

    return rows.map((r) => ({
      id: r.id,
      condition: JSON.parse(r.condition_json),
      action: r.action,
      confidence: r.confidence,
      nObs: r.n_obs,
      active: r.active === 1,
    }));
  }

  /** Reset preferences for a specific tool or all */
  reset(toolName?: string): void {
    if (toolName) {
      this.db.prepare("DELETE FROM preferences WHERE tool_name = ?").run(toolName);
      this.db
        .prepare("DELETE FROM learned_rules WHERE condition_json LIKE ?")
        .run(`%"tool":"${toolName}"%`);
    } else {
      this.db.prepare("DELETE FROM preferences").run();
      this.db.prepare("DELETE FROM learned_rules").run();
    }
  }

  /** Get summary stats */
  stats(): { totalPrefs: number; activeRules: number; totalEvents: number } {
    const prefs = this.db.prepare("SELECT COUNT(*) as c FROM preferences").get() as { c: number };
    const rules = this.db
      .prepare("SELECT COUNT(*) as c FROM learned_rules WHERE active = 1")
      .get() as { c: number };
    const events = this.db.prepare("SELECT COUNT(*) as c FROM tool_events").get() as { c: number };
    return { totalPrefs: prefs.c, activeRules: rules.c, totalEvents: events.c };
  }

  private outcomeToDelta(outcome: Outcome): number {
    switch (outcome) {
      case "accepted":
      case "auto":
        return 1.0;
      case "always":
        return 1.0;
      case "rejected":
        return -1.0;
      case "modified":
        return 0.5;
      default:
        return 0;
    }
  }
}
