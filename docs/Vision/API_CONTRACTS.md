# ONI — API Contracts

Internal interfaces, message formats, and SQLite schemas.
All types are Rust.

---

## Ollama Client (`oni-ollama`)

`OllamaClient` in `crates/oni-ollama/src/client.rs` wraps reqwest.
Default base URL: `http://localhost:11434`. Default timeout: 300 s.

### `OllamaClient::chat(request: &ChatRequest) -> Result<ChatResponse>`

Posts to `/api/chat`. Always non-streaming (`stream: false`).
Returns `ChatResponse` with the model's message and token counts.

### `OllamaClient::embed(request: &EmbedRequest) -> Result<EmbedResponse>`

Posts to `/api/embed`. Returns embedding vectors for the input string.
Used by `oni-context` for semantic retrieval.

### `OllamaClient::health_check() -> Result<Vec<ModelInfo>>`

Gets `/api/tags` with a 5-second timeout. Returns a list of locally available models.
Used by `oni doctor` and `ModelRouter` to verify model availability.

### Message Types (`crates/oni-ollama/src/models.rs`)

```rust
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,          // always false
    pub keep_alive: Option<i64>,
    pub options: Option<HashMap<String, serde_json::Value>>,
    pub tools: Option<Vec<serde_json::Value>>,  // native tool calling schemas
}

pub struct ChatMessage {
    pub role: String,          // "system" | "user" | "assistant" | "tool"
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

pub struct ChatResponse {
    pub model: String,
    pub message: ResponseMessage,
    pub done: bool,
    pub total_duration: Option<u64>,     // nanoseconds
    pub prompt_eval_count: Option<u64>,  // input tokens
    pub eval_count: Option<u64>,         // output tokens
    pub eval_duration: Option<u64>,      // nanoseconds
}

pub struct EmbedRequest {
    pub model: String,
    pub input: String,
}

pub struct EmbedResponse {
    pub model: String,
    pub embeddings: Vec<Vec<f32>>,
}
```

`ChatMessage` constructors: `::system()`, `::user()`, `::assistant()`, `::tool()`,
`::assistant_with_tool_calls()`.

---

## Tool Trait (`oni-agent`)

All tools implement `Tool` from `crates/oni-agent/src/tools/mod.rs`:

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;  // OpenAI-style function schema
    fn execute(&self, args: serde_json::Value) -> Result<String>;
}
```

`execute` is synchronous. Async operations (e.g. `get_url` HTTP fetch) use
`tokio::task::block_in_place` + `Handle::current().block_on(...)`.

---

## ToolRegistry (`oni-agent`)

`crates/oni-agent/src/tools/mod.rs`

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
    _allow_write: bool,
    _allow_exec: bool,
    pub undo_history: UndoHistory,
}
```

### Construction

```rust
ToolRegistry::new(allow_write: bool, allow_exec: bool) -> Self
ToolRegistry::new_with_channels(allow_write, allow_exec, ask_user_channel: Option<AskUserChannel>) -> Self
```

Tools registered at construction time based on flags:
- Always: `read_file`, `list_dir`, `search_files`, `get_url`, `undo`
- `allow_write`: `write_file`, `edit_file`
- `allow_exec`: `bash`, `forge_tool`
- Optional: `ask_user` (when channel provided)

### Methods

```rust
// Dispatch tool call by name. Snapshots file state before write_file / edit_file.
pub fn execute(&self, name: &str, args: serde_json::Value) -> Result<String>

// Returns OpenAI-style schemas for all registered tools (passed to Ollama).
pub fn tool_schemas(&self) -> Vec<serde_json::Value>

pub fn tool_names(&self) -> Vec<&str>
```

---

## AgentEvent Enum (`oni-agent`)

`crates/oni-agent/src/agent.rs`. Events flow from the agent to the TUI via
`mpsc::UnboundedSender<AgentEvent>`.

```rust
pub enum AgentEvent {
    /// LLM is generating — show spinner.
    Thinking,

    /// Planner produced a step list (orchestrated mode only).
    PlanGenerated { steps: Vec<String> },

    /// Executor is working on one step of the plan.
    ExecutorStep { step: usize, total: usize, description: String },

    /// A tool was called. Status progresses: "PENDING" -> "EXECUTING" -> "DONE" | "SKIPPED".
    /// args is raw JSON so the TUI can render rich previews (diff view, command block).
    ToolExec {
        name: String,
        status: String,
        args: serde_json::Value,
        result: Option<String>,
    },

    /// Critic gave a verdict on an executor step.
    CriticVerdict { accepted: bool, reason: String },

    /// Orchestrator is replanning after a critic rejection.
    Replanning { cycle: usize, reason: String },

    /// Final text response from the model.
    Response(String),

    /// Agent-level error (not a tool error).
    Error(String),

    /// Turn complete.
    Done { tokens: u64, duration_ms: u64 },

    /// Session or monthly token budget exhausted.
    BudgetExhausted { limit_type: String, used: u64, limit: u64 },
}
```

### Tool Confirmation Flow

`ToolProposal` is sent via a separate `mpsc::UnboundedSender<ToolProposal>` channel when
user confirmation is required (controlled by `AutonomyLevel`). The TUI responds with a
`oneshot::Sender<ConfirmResponse>`.

```rust
pub enum ConfirmResponse { Yes, No, Diff, Always }
```

---

## Config (`oni-core`)

TOML format. `crates/oni-core/src/config.rs`.

Load order: defaults → `~/.config/oni/oni.toml` (global) → `.oni/oni.toml` (project).
Each layer is deep-merged (table keys merged recursively, scalars overwritten).

```toml
[ollama]
base_url = "http://localhost:11434"  # default
timeout_secs = 300
keep_alive = -1                      # keep models loaded indefinitely

[models]
heavy   = "qwen3.5:35b"             # Planner tier
medium  = "qwen3-coder:30b"         # Executor tier (default)
general = "glm-4.7-flash:q4_k_m"   # General chat tier
fast    = "qwen3.5:9b"              # Critic / fast tier
embed   = "nomic-embed-text"        # Embeddings
default_tier = "Medium"

[ui]
fps = 30
show_thinking = false
show_token_stats = true

[agent]
max_tool_rounds = 10
context_budget_tokens = 8192
allow_write = false
allow_exec = false
autonomy = "Medium"              # Low | Medium | High
session_budget = 0               # 0 = unlimited
monthly_limit = 0                # 0 = unlimited

[agent.compaction]
token_threshold = 19660          # trigger at ~60% of 32K context
message_threshold = 40
retention_window = 4             # messages to keep after compaction
summary_max_tokens = 500

[agent.reasoning]
enabled = false
effort = "medium"                # low | medium | high
show_thinking = false
```

---

## Database Schema (`oni-db`)

SQLite. `crates/oni-db/src/schema.rs`. WAL mode, foreign keys ON.
Default path: `~/.local/share/oni/oni.db`.

### `conversations`

```sql
CREATE TABLE conversations (
    conv_id     TEXT PRIMARY KEY,
    source      TEXT NOT NULL DEFAULT 'cli',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    last_active TEXT NOT NULL DEFAULT (datetime('now')),
    project_dir TEXT
);
```

### `messages`

```sql
CREATE TABLE messages (
    msg_id    TEXT PRIMARY KEY,
    conv_id   TEXT NOT NULL REFERENCES conversations(conv_id) ON DELETE CASCADE,
    role      TEXT NOT NULL CHECK(role IN ('system','user','assistant','tool')),
    content   TEXT NOT NULL,
    origin    TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    tokens    INTEGER DEFAULT 0
);
CREATE INDEX idx_messages_conv ON messages(conv_id, timestamp);
```

### `tool_events`

```sql
CREATE TABLE tool_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT,
    tool_name   TEXT NOT NULL,
    args_json   TEXT,
    result_json TEXT,
    latency_ms  INTEGER,
    timestamp   TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_tool_events_session ON tool_events(session_id);
```

### `preference_signals`

```sql
CREATE TABLE preference_signals (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT,
    tool_name   TEXT NOT NULL,
    signal_type TEXT NOT NULL CHECK(signal_type IN ('accept','reject','edit','rerun')),
    context     TEXT,
    weight      REAL DEFAULT 1.0,
    timestamp   TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_pref_signals_tool ON preference_signals(tool_name);
```

### `learned_rules`

```sql
CREATE TABLE learned_rules (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    description  TEXT NOT NULL,
    context      TEXT NOT NULL,       -- e.g. "TOOL=bash"
    confidence   REAL NOT NULL DEFAULT 0.5,
    observations INTEGER NOT NULL DEFAULT 0,
    last_updated TEXT NOT NULL DEFAULT (datetime('now')),
    active       INTEGER NOT NULL DEFAULT 0  -- 1 when confidence >= 0.8
);
```

There are exactly 5 tables. There are no `tasks`, `installed_plugins`, `plugin_tools`,
`plugin_auth`, `sync_log`, or index tables in this schema.

---

## Preference Signals and Rule Crystallisation (`oni-agent`)

`crates/oni-agent/src/preferences.rs`

### Signal Types

```rust
pub enum SignalType { Accept, Reject, Edit, Rerun }
```

`Accept` is recorded automatically on every successful tool execution.
`Reject` / `Edit` / `Rerun` are recorded when the user declines, modifies, or
re-runs a tool proposal.

### Confidence Formula

```
confidence = (Σ accept_weight × decay + Σ rerun_weight × 0.5 × decay) / Σ total_weight × decay
decay = 0.5 if signal older than 7 days, else 1.0
```

### Crystallisation Threshold

A new `learned_rule` row is created when a tool accumulates **≥ 10 signals** AND
weighted confidence **≥ 0.7**. Rules become `active = 1` when confidence **≥ 0.8**.

Active rules are injected into the system prompt at the start of each turn:
```
## LEARNED PREFERENCES
- Use bash tool (inferred from usage patterns) (confidence: 85%)
```

### `PreferenceEngine` Methods

```rust
fn record_signal(&self, tool_name: &str, signal: SignalType, context: &str, session_id: Option<&str>)
fn get_active_rules(&self) -> Vec<LearnedRule>   // confidence >= 0.8
fn get_all_rules(&self) -> Vec<LearnedRule>       // all rules, for TUI preferences view
fn update_rules(&self)                            // recompute confidence for all rules
fn crystallise_rules(&self)                       // create new rules from signal patterns
```

---

## Model Router (`oni-ollama`)

`crates/oni-ollama/src/router.rs`. Routes requests to the correct Ollama model
based on `ModelTier`.

```rust
pub enum ModelTier { Heavy, Medium, General, Fast, Embed }
```

`ModelRouter::chat(tier, messages)` and `::chat_with_tools(tier, messages, schemas)`
select the model string from `ModelConfig` and dispatch to `OllamaClient`.

`tier.supports_tools()` — returns true for tiers whose configured model supports
Ollama's native tool calling protocol. Used by the agent to decide whether to pass
tool schemas or rely on text-based tool call extraction.

---

## Context Retrieval (`oni-context`)

`crates/oni-context/src/retriever.rs`. Queries the project's `.oni/index.db`
(a separate SQLite database, not the main `oni.db`).

`retrieve(conn, query, budget_tokens) -> Result<Vec<Chunk>>`

Returns code chunks ranked by keyword match. Budget limits total characters returned.
Called from `build_system_prompt_with_context_opts()` in `oni-agent/src/system_prompt.rs`.
