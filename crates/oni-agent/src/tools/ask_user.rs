//! Ask-user tool — pauses agent execution and prompts the user for a free-text response.
//! Uses a synchronous channel pair: the tool sends the question and blocks until
//! the TUI sends back the user's answer.

use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;
use std::sync::{Arc, Mutex};

/// A pending question from the agent to the user.
pub struct AskUserRequest {
    pub question: String,
    /// The tool sends a response back through this sender.
    pub respond: std::sync::mpsc::SyncSender<String>,
}

/// Shared state for the ask-user channel. Cloneable so both Agent and tool hold a reference.
#[derive(Clone)]
pub struct AskUserChannel {
    inner: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<AskUserRequest>>>>,
}

impl AskUserChannel {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    pub fn set_sender(&self, tx: tokio::sync::mpsc::UnboundedSender<AskUserRequest>) {
        *self.inner.lock().unwrap() = Some(tx);
    }

    /// Ask the user a question. Blocks until a response is received (or channel is gone).
    pub fn ask(&self, question: &str) -> Option<String> {
        let tx = self.inner.lock().unwrap().clone()?;
        let (resp_tx, resp_rx) = std::sync::mpsc::sync_channel(1);
        let req = AskUserRequest {
            question: question.to_string(),
            respond: resp_tx,
        };
        tx.send(req).ok()?;
        // Block this thread (tool execute is sync) until TUI responds.
        // block_in_place so Tokio can park this worker without deadlocking.
        tokio::task::block_in_place(|| resp_rx.recv()).ok()
    }
}

impl Default for AskUserChannel {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AskUserTool {
    channel: AskUserChannel,
}

impl AskUserTool {
    pub fn new(channel: AskUserChannel) -> Self {
        Self { channel }
    }
}

impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::UserInteraction]
    }

    fn description(&self) -> &str {
        "Ask the user a clarifying question and wait for their response before continuing. \
         Use this when you need information that cannot be inferred from the codebase or context."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "ask_user",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "question": {
                            "type": "string",
                            "description": "The question to ask the user"
                        }
                    },
                    "required": ["question"]
                }
            }
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let question = args
            .get("question")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'question' argument"))?;

        match self.channel.ask(question) {
            Some(response) => Ok(format!("User responded: {}", response)),
            None => Ok("No response received (channel unavailable).".to_string()),
        }
    }
}
