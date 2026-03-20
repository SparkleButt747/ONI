//! Execution Trace — records all agent actions as a queryable graph.
//! Every tool call, decision, and file change is tracked.
//! Enables "what did the agent do and why" analysis.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub timestamp: u64,
    pub agent: String,
    pub event_type: TraceEventType,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraceEventType {
    ToolCall { tool: String, args_summary: String },
    ToolResult { tool: String, success: bool },
    Decision { description: String },
    FileChange { path: String, action: String }, // "create", "modify", "delete"
    PlanStep { step: usize, total: usize },
    CriticVerdict { accepted: bool },
    AgentSpawn { child_id: String },
    Error { message: String },
}

/// In-memory trace for the current session.
#[derive(Debug, Clone)]
pub struct ExecutionTrace {
    events: VecDeque<TraceEvent>,
    max_events: usize,
}

impl ExecutionTrace {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: VecDeque::new(),
            max_events,
        }
    }

    pub fn record(&mut self, agent: &str, event_type: TraceEventType, details: &str) {
        let event = TraceEvent {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            agent: agent.to_string(),
            event_type,
            details: details.to_string(),
        };
        self.events.push_back(event);
        if self.events.len() > self.max_events {
            self.events.pop_front();
        }
    }

    /// Get all events.
    pub fn events(&self) -> &VecDeque<TraceEvent> {
        &self.events
    }

    /// Get events for a specific agent.
    pub fn events_for_agent(&self, agent: &str) -> Vec<&TraceEvent> {
        self.events.iter().filter(|e| e.agent == agent).collect()
    }

    /// Get the last N events.
    pub fn recent(&self, n: usize) -> Vec<&TraceEvent> {
        self.events.iter().rev().take(n).collect()
    }

    /// Summary for display.
    pub fn summary(&self) -> String {
        let tool_calls = self.events.iter().filter(|e| matches!(e.event_type, TraceEventType::ToolCall { .. })).count();
        let file_changes = self.events.iter().filter(|e| matches!(e.event_type, TraceEventType::FileChange { .. })).count();
        let errors = self.events.iter().filter(|e| matches!(e.event_type, TraceEventType::Error { .. })).count();
        format!(
            "Trace: {} events ({} tool calls, {} file changes, {} errors)",
            self.events.len(), tool_calls, file_changes, errors
        )
    }

    /// Export as readable text for journal entry.
    pub fn to_journal_entry(&self) -> String {
        let mut lines = Vec::new();
        for event in &self.events {
            let type_str = match &event.event_type {
                TraceEventType::ToolCall { tool, .. } => format!("TOOL:{}", tool),
                TraceEventType::ToolResult { tool, success } => {
                    format!("RESULT:{}:{}", tool, if *success { "OK" } else { "ERR" })
                }
                TraceEventType::Decision { description } => format!("DECIDE:{}", description),
                TraceEventType::FileChange { path, action } => format!("FILE:{}:{}", action, path),
                TraceEventType::PlanStep { step, total } => format!("STEP:{}/{}", step, total),
                TraceEventType::CriticVerdict { accepted } => {
                    format!("VERDICT:{}", if *accepted { "ACCEPT" } else { "REJECT" })
                }
                TraceEventType::AgentSpawn { child_id } => format!("SPAWN:{}", child_id),
                TraceEventType::Error { message } => format!("ERROR:{}", message),
            };
            lines.push(format!("  {} {}", type_str, event.details));
        }
        lines.join("\n")
    }
}

impl Default for ExecutionTrace {
    fn default() -> Self {
        Self::new(500)
    }
}
