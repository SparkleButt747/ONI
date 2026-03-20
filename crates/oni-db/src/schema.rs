use oni_core::error::{Result, WrapErr};
use std::path::Path;

const SCHEMA: &str = r#"
    PRAGMA journal_mode = WAL;
    PRAGMA foreign_keys = ON;

    CREATE TABLE IF NOT EXISTS conversations (
        conv_id TEXT PRIMARY KEY,
        source TEXT NOT NULL DEFAULT 'cli',
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        last_active TEXT NOT NULL DEFAULT (datetime('now')),
        project_dir TEXT
    );

    CREATE TABLE IF NOT EXISTS messages (
        msg_id TEXT PRIMARY KEY,
        conv_id TEXT NOT NULL REFERENCES conversations(conv_id) ON DELETE CASCADE,
        role TEXT NOT NULL CHECK(role IN ('system','user','assistant','tool')),
        content TEXT NOT NULL,
        origin TEXT,
        timestamp TEXT NOT NULL DEFAULT (datetime('now')),
        tokens INTEGER DEFAULT 0
    );
    CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conv_id, timestamp);

    CREATE TABLE IF NOT EXISTS tool_events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id TEXT,
        tool_name TEXT NOT NULL,
        args_json TEXT,
        result_json TEXT,
        latency_ms INTEGER,
        timestamp TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_tool_events_session ON tool_events(session_id);

    CREATE TABLE IF NOT EXISTS preference_signals (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id TEXT,
        tool_name TEXT NOT NULL,
        signal_type TEXT NOT NULL CHECK(signal_type IN ('accept','reject','edit','rerun')),
        context TEXT,
        weight REAL DEFAULT 1.0,
        timestamp TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE INDEX IF NOT EXISTS idx_pref_signals_tool ON preference_signals(tool_name);

    CREATE TABLE IF NOT EXISTS learned_rules (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        description TEXT NOT NULL,
        context TEXT NOT NULL,
        confidence REAL NOT NULL DEFAULT 0.5,
        observations INTEGER NOT NULL DEFAULT 0,
        last_updated TEXT NOT NULL DEFAULT (datetime('now')),
        active INTEGER NOT NULL DEFAULT 0
    );
"#;

pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).wrap_err("Failed to create database directory")?;
        }
        let conn = rusqlite::Connection::open(path)
            .wrap_err_with(|| format!("Failed to open database at {}", path.display()))?;
        conn.execute_batch(SCHEMA)
            .wrap_err("Failed to initialize database schema")?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn =
            rusqlite::Connection::open_in_memory().wrap_err("Failed to open in-memory database")?;
        conn.execute_batch(SCHEMA)
            .wrap_err("Failed to initialize database schema")?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &rusqlite::Connection {
        &self.conn
    }

    /// Clean up old data to control database size.
    /// - Deletes conversations older than `max_age_days`
    /// - Deletes tool events older than `max_age_days`
    /// - VACUUMs the database to reclaim space
    pub fn cleanup(&self, max_age_days: u32) -> Result<(usize, usize)> {
        let age_clause = format!("datetime('now', '-{} days')", max_age_days);

        let convos_deleted: usize = self
            .conn
            .execute(
                &format!(
                    "DELETE FROM conversations WHERE last_active < {}",
                    age_clause
                ),
                [],
            )
            .wrap_err("Failed to delete old conversations")?;

        // Messages cascade-delete via foreign key

        let events_deleted: usize = self
            .conn
            .execute(
                &format!(
                    "DELETE FROM tool_events WHERE timestamp < {}",
                    age_clause
                ),
                [],
            )
            .wrap_err("Failed to delete old tool events")?;

        // Reclaim space
        let _ = self.conn.execute("VACUUM", []);

        Ok((convos_deleted, events_deleted))
    }

    /// Get database file size in bytes (returns 0 for in-memory databases).
    pub fn file_size(&self) -> u64 {
        let page_count: i64 = self
            .conn
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .unwrap_or(0);
        let page_size: i64 = self
            .conn
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .unwrap_or(0);
        (page_count * page_size) as u64
    }
}
