//! Generic pub/sub message bus — thread-safe, lock-based.
//! Used for AgentEvent delivery (agent -> TUI/headless) and for inter-agent
//! communication (BusMessage).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Thread-safe pub/sub message bus. Generic over any `Clone + Send` payload.
///
/// Cloning the bus gives a handle to the **same** underlying queue (via `Arc`),
/// so publishers and consumers share one buffer.
#[derive(Debug)]
pub struct MessageBus<T: Clone + Send> {
    messages: Arc<Mutex<VecDeque<T>>>,
    max_history: usize,
}

// Manual Clone — derive would add a `T: Clone` bound on the impl which is
// already satisfied by the trait bound, but we also need it without requiring
// `T: Debug`.
impl<T: Clone + Send> Clone for MessageBus<T> {
    fn clone(&self) -> Self {
        Self {
            messages: self.messages.clone(),
            max_history: self.max_history,
        }
    }
}

impl<T: Clone + Send> MessageBus<T> {
    pub fn new(max_history: usize) -> Self {
        Self {
            messages: Arc::new(Mutex::new(VecDeque::new())),
            max_history,
        }
    }

    /// Publish a message to the bus.
    pub fn publish(&self, msg: T) {
        let mut msgs = self.messages.lock().unwrap();
        msgs.push_back(msg);
        if msgs.len() > self.max_history {
            msgs.pop_front();
        }
    }

    /// Drain all messages since the last drain (destructive read).
    pub fn drain(&self) -> Vec<T> {
        let mut msgs = self.messages.lock().unwrap();
        msgs.drain(..).collect()
    }

    /// Get all messages (non-destructive).
    pub fn peek_all(&self) -> Vec<T> {
        self.messages.lock().unwrap().iter().cloned().collect()
    }

    /// Get recent N messages (non-destructive, newest first).
    pub fn recent(&self, n: usize) -> Vec<T> {
        let msgs = self.messages.lock().unwrap();
        msgs.iter().rev().take(n).cloned().collect()
    }

    /// Count of buffered messages.
    pub fn len(&self) -> usize {
        self.messages.lock().unwrap().len()
    }

    /// Whether the bus is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T: Clone + Send> Default for MessageBus<T> {
    fn default() -> Self {
        Self::new(100)
    }
}

// ─── Legacy BusMessage type (inter-agent communication) ─────────────────────

#[derive(Debug, Clone)]
pub enum BusMessage {
    /// An agent discovered something noteworthy.
    Discovery { agent: String, content: String },
    /// Warning: something might be wrong.
    Warning { agent: String, content: String },
    /// A task or subtask completed.
    TaskComplete { agent: String, task: String, result: String },
    /// A task failed.
    TaskFailed { agent: String, task: String, error: String },
    /// A file was modified.
    FileChanged { agent: String, path: String },
}

impl BusMessage {
    pub fn agent(&self) -> &str {
        match self {
            Self::Discovery { agent, .. } => agent,
            Self::Warning { agent, .. } => agent,
            Self::TaskComplete { agent, .. } => agent,
            Self::TaskFailed { agent, .. } => agent,
            Self::FileChanged { agent, .. } => agent,
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::Discovery { agent, content } => format!("[{}] DISCOVERY: {}", agent, content),
            Self::Warning { agent, content } => format!("[{}] WARNING: {}", agent, content),
            Self::TaskComplete { agent, task, .. } => format!("[{}] DONE: {}", agent, task),
            Self::TaskFailed { agent, task, error } => {
                format!("[{}] FAILED: {} — {}", agent, task, error)
            }
            Self::FileChanged { agent, path } => format!("[{}] MODIFIED: {}", agent, path),
        }
    }
}
