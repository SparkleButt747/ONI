//! Undo tool — reverts the last file write/edit operation.
//! Maintains a stack of file snapshots taken before each write.

use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Shared undo history — thread-safe snapshot stack.
#[derive(Clone)]
pub struct UndoHistory {
    inner: Arc<Mutex<VecDeque<FileSnapshot>>>,
    max_entries: usize,
}

#[derive(Debug, Clone)]
struct FileSnapshot {
    path: String,
    content: Option<String>, // None = file didn't exist before
}

impl UndoHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::new())),
            max_entries,
        }
    }

    /// Snapshot the current state of a file before modifying it.
    pub fn snapshot(&self, path: &str) {
        let content = std::fs::read_to_string(path).ok();
        let mut stack = self.inner.lock().unwrap();
        stack.push_back(FileSnapshot {
            path: path.to_string(),
            content,
        });
        if stack.len() > self.max_entries {
            stack.pop_front();
        }
    }

    /// Undo the last change. Returns (path, result_message).
    pub fn undo(&self) -> Option<(String, String)> {
        let mut stack = self.inner.lock().unwrap();
        let snapshot = stack.pop_back()?;
        match &snapshot.content {
            Some(content) => {
                if let Err(e) = std::fs::write(&snapshot.path, content) {
                    return Some((
                        snapshot.path.clone(),
                        format!("Error reverting {}: {}", snapshot.path, e),
                    ));
                }
                Some((
                    snapshot.path.clone(),
                    format!("Reverted {}", snapshot.path),
                ))
            }
            None => {
                // File didn't exist before — delete it
                if let Err(e) = std::fs::remove_file(&snapshot.path) {
                    return Some((
                        snapshot.path.clone(),
                        format!("Error reverting {}: {}", snapshot.path, e),
                    ));
                }
                Some((
                    snapshot.path.clone(),
                    format!("Removed {} (didn't exist before)", snapshot.path),
                ))
            }
        }
    }

    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct UndoTool {
    history: UndoHistory,
}

impl UndoTool {
    pub fn new(history: UndoHistory) -> Self {
        Self { history }
    }
}

impl Tool for UndoTool {
    fn name(&self) -> &str {
        "undo"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::WriteFs]
    }

    fn description(&self) -> &str {
        "Undo the last file change. Restores the previous state of the most recently written or edited file."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "undo",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }
        })
    }

    fn execute(&self, _args: serde_json::Value) -> Result<String> {
        match self.history.undo() {
            Some((_path, msg)) => Ok(msg),
            None => Ok("No changes to undo.".to_string()),
        }
    }
}
