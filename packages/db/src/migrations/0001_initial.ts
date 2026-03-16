import type Database from "better-sqlite3";

export function runMigrations(db: Database.Database): void {
  db.exec(`
    CREATE TABLE IF NOT EXISTS conversations (
      conv_id       TEXT PRIMARY KEY,
      source        TEXT NOT NULL DEFAULT 'local',
      created_at    INTEGER NOT NULL,
      last_active   INTEGER NOT NULL,
      project_dir   TEXT
    );

    CREATE TABLE IF NOT EXISTS messages (
      msg_id        TEXT PRIMARY KEY,
      conv_id       TEXT NOT NULL REFERENCES conversations(conv_id),
      role          TEXT NOT NULL,
      content       TEXT NOT NULL,
      origin        TEXT NOT NULL DEFAULT 'terminal',
      ts            INTEGER NOT NULL,
      tokens        INTEGER
    );
    CREATE INDEX IF NOT EXISTS messages_conv ON messages(conv_id, ts);

    CREATE TABLE IF NOT EXISTS tool_events (
      id            INTEGER PRIMARY KEY AUTOINCREMENT,
      session_id    TEXT NOT NULL,
      tool_name     TEXT NOT NULL,
      args_json     TEXT,
      result_json   TEXT,
      latency_ms    INTEGER,
      ts            INTEGER NOT NULL
    );
  `);
}
