use crate::schema::Database;
use oni_core::error::{Result, WrapErr};

pub struct StoredMessage {
    pub msg_id: String,
    pub role: String,
    pub content: String,
    pub tokens: i64,
    pub timestamp: String,
}

pub struct ConversationInfo {
    pub conv_id: String,
    pub source: String,
    pub created_at: String,
    pub last_active: String,
    pub project_dir: Option<String>,
}

impl Database {
    pub fn create_conversation(&self, project_dir: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn()
            .execute(
                "INSERT INTO conversations (conv_id, project_dir) VALUES (?1, ?2)",
                rusqlite::params![id, project_dir],
            )
            .wrap_err("Failed to create conversation")?;
        Ok(id)
    }

    pub fn add_message(&self, conv_id: &str, role: &str, content: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let tokens = (content.len() / 4) as i64;
        self.conn()
            .execute(
                "INSERT INTO messages (msg_id, conv_id, role, content, tokens) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, conv_id, role, content, tokens],
            )
            .wrap_err("Failed to add message")?;

        // Update last_active
        self.conn()
            .execute(
                "UPDATE conversations SET last_active = datetime('now') WHERE conv_id = ?1",
                [conv_id],
            )
            .wrap_err("Failed to update conversation")?;

        Ok(id)
    }

    pub fn get_messages(&self, conv_id: &str) -> Result<Vec<StoredMessage>> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT msg_id, role, content, tokens, timestamp FROM messages WHERE conv_id = ?1 ORDER BY timestamp",
            )
            .wrap_err("Failed to prepare message query")?;
        let rows = stmt
            .query_map([conv_id], |row| {
                Ok(StoredMessage {
                    msg_id: row.get(0)?,
                    role: row.get(1)?,
                    content: row.get(2)?,
                    tokens: row.get(3)?,
                    timestamp: row.get(4)?,
                })
            })
            .wrap_err("Failed to query messages")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn list_conversations(&self) -> Result<Vec<ConversationInfo>> {
        let mut stmt = self
            .conn()
            .prepare(
                "SELECT conv_id, source, created_at, last_active, project_dir FROM conversations ORDER BY last_active DESC",
            )
            .wrap_err("Failed to prepare conversation query")?;
        let rows = stmt
            .query_map([], |row| {
                Ok(ConversationInfo {
                    conv_id: row.get(0)?,
                    source: row.get(1)?,
                    created_at: row.get(2)?,
                    last_active: row.get(3)?,
                    project_dir: row.get(4)?,
                })
            })
            .wrap_err("Failed to query conversations")?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}
