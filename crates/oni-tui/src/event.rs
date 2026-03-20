use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    LlmResponse(String),
    LlmThinking,
    LlmError(String),
    LlmDone { tokens: u64, duration_ms: u64 },
    ToolExec { name: String, status: String },
    Resize(u16, u16),
    Quit,
}
