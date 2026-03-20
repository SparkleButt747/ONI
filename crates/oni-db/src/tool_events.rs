use crate::schema::Database;
use oni_core::error::{Result, WrapErr};

impl Database {
    pub fn log_tool_event(
        &self,
        session_id: &str,
        tool_name: &str,
        args_json: &str,
        result_json: &str,
        latency_ms: i64,
    ) -> Result<()> {
        self.conn()
            .execute(
                "INSERT INTO tool_events (session_id, tool_name, args_json, result_json, latency_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![session_id, tool_name, args_json, result_json, latency_ms],
            )
            .wrap_err("Failed to log tool event")?;
        Ok(())
    }
}
