import type Database from "better-sqlite3";

export function createPrefsSchema(db: Database.Database): void {
  db.exec(`
    CREATE TABLE IF NOT EXISTS preferences (
      id            INTEGER PRIMARY KEY AUTOINCREMENT,
      tool_name     TEXT NOT NULL,
      intent_key    TEXT NOT NULL,
      weight        REAL NOT NULL DEFAULT 0.5,
      n_obs         INTEGER NOT NULL DEFAULT 0,
      last_updated  INTEGER NOT NULL,
      UNIQUE(tool_name, intent_key)
    );

    CREATE TABLE IF NOT EXISTS learned_rules (
      id             INTEGER PRIMARY KEY AUTOINCREMENT,
      condition_json TEXT NOT NULL,
      action         TEXT NOT NULL,
      confidence     REAL NOT NULL,
      n_obs          INTEGER NOT NULL DEFAULT 0,
      active         INTEGER NOT NULL DEFAULT 1,
      created_at     INTEGER NOT NULL,
      last_fired     INTEGER
    );

    CREATE TABLE IF NOT EXISTS tool_events (
      id            INTEGER PRIMARY KEY AUTOINCREMENT,
      session_id    TEXT NOT NULL,
      tool_name     TEXT NOT NULL,
      intent_key    TEXT NOT NULL,
      outcome       TEXT NOT NULL,
      latency_ms    INTEGER,
      ts            INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS tool_events_tool ON tool_events(tool_name, intent_key);
  `);
}
