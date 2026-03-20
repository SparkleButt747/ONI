# ONI v2 — Rust/Ratatui + Ollama Rewrite

> **STATUS: COMPLETED.** This plan was fully executed. All paths referencing `oni/crates/` are stale — the workspace is now at repo root (`crates/`). Kept for historical reference only. See `2026-03-18-oni-complete-implementation.md` and `2026-03-19-oni-codebase-audit-fixes.md` for the follow-up work.

**Goal:** Rewrite ONI from TypeScript/Ink/Claude to Rust/Ratatui/Ollama — a local AI coding assistant with a biotech HUD aesthetic, powered by local LLMs.

**Architecture:** Elm Architecture (Model + Message + update + view) with Tokio async runtime. Ollama API calls run in spawned tasks, communicating via mpsc channels. The TUI renders at 30fps with crossterm. All LLM responses are batch-mode (`stream: false`) — show a loading indicator while waiting, render the full response at once.

**Tech Stack:** Rust 1.94, Ratatui 0.29+, crossterm, Tokio, ollama-rs, clap v4, rusqlite, serde + TOML, tui-markdown, tui-textarea, throbber-widgets-tui, tachyonfx, color-eyre, tracing.

**Reference projects:**
- Sparkles (predecessor): `/Users/brndy.747/Projects/ArchivedProjects/Sparkles/` — Ollama client patterns, LIT detection, policy engine, multi-model routing
- Oatmeal (open source): Ratatui + Ollama chat TUI — reference for architecture
- Design inspo: `/Users/brndy.747/Projects/ONI/docs/Vision/Images/` — biotech HUD aesthetic

---

## File Structure

```
oni/                              # Cargo workspace root (replaces TS packages/)
├── Cargo.toml                    # Workspace manifest
├── Cargo.lock
├── oni.toml                      # Default config (ships with binary)
├── crates/
│   ├── oni-core/                 # Shared types, config, errors
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config.rs         # TOML config loading (global + project)
│   │       ├── error.rs          # color-eyre error types
│   │       ├── types.rs          # Message, Role, ToolCall, ModelTier
│   │       └── palette.rs        # ONI colour palette constants
│   │
│   ├── oni-ollama/               # Ollama API client (batch mode)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs         # Single-model client (reqwest, stream:false)
│   │       ├── router.rs         # Multi-model router (tier selection)
│   │       ├── models.rs         # API request/response structs
│   │       └── health.rs         # Health check + model availability
│   │
│   ├── oni-agent/                # Agent loop, tools, orchestration
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── agent.rs          # Main agent loop (tool rounds)
│   │       ├── tools/
│   │       │   ├── mod.rs        # Tool registry
│   │       │   ├── read_file.rs
│   │       │   ├── write_file.rs
│   │       │   ├── bash.rs
│   │       │   └── list_dir.rs
│   │       ├── conversation.rs   # Message history management
│   │       ├── system_prompt.rs  # Prompt building with context
│   │       └── budget.rs         # Token tracking
│   │
│   ├── oni-context/              # Code indexing + RAG retrieval
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── walker.rs         # File discovery (.gitignore aware)
│   │       ├── indexer.rs        # Symbol extraction + FTS5 indexing
│   │       ├── retriever.rs      # Token-budgeted context packing
│   │       └── embeddings.rs     # nomic-embed-text via Ollama
│   │
│   ├── oni-db/                   # SQLite persistence
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── schema.rs         # Table creation + migrations
│   │       ├── conversations.rs  # CRUD for conversations/messages
│   │       └── tool_events.rs    # Tool execution logging
│   │
│   └── oni-tui/                  # Ratatui terminal UI
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── app.rs            # App state (Model), event loop, update()
│           ├── event.rs          # AppEvent enum (Key, Tick, LlmResponse, etc.)
│           ├── ui/
│           │   ├── mod.rs        # Root view() — layout composition
│           │   ├── chat.rs       # Chat pane (message list + scrolling)
│           │   ├── input.rs      # Input area (tui-textarea)
│           │   ├── status.rs     # Status bar (model, tokens, timing)
│           │   ├── sidebar.rs    # Conversation list / model selector
│           │   ├── splash.rs     # Boot/idle splash screen (dot-matrix art)
│           │   └── thinking.rs   # Loading indicator while waiting for LLM
│           ├── theme.rs          # Palette, styles, ALL_CAPS formatters
│           └── widgets/
│               ├── mod.rs
│               ├── arc.rs        # Braille canvas arc framing device
│               ├── spectrum.rs   # Bar chart visualiser
│               ├── big_text.rs   # Large numeric/text display
│               └── glitch.rs     # Glitch artefact decorative blocks
│
├── src/
│   └── main.rs                   # Binary entry point — clap CLI + dispatch
│
└── tests/
    ├── ollama_integration.rs     # Live Ollama tests (skip if not running)
    ├── agent_loop.rs             # Agent tool loop tests
    ├── context_index.rs          # Indexing + retrieval tests
    └── db_schema.rs              # SQLite schema tests
```

---

## Task 1: Cargo Workspace Scaffold

**Files:**
- Create: `oni/Cargo.toml` (workspace)
- Create: `oni/src/main.rs`
- Create: `oni/crates/oni-core/Cargo.toml`
- Create: `oni/crates/oni-core/src/lib.rs`
- Create: `oni/crates/oni-core/src/types.rs`
- Create: `oni/crates/oni-core/src/error.rs`
- Create: `oni/crates/oni-core/src/palette.rs`
- Create: `oni/crates/oni-core/src/config.rs`

This task sets up the workspace, shared types, colour palette, and config system. Everything else depends on this.

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/oni-core",
    "crates/oni-ollama",
    "crates/oni-agent",
    "crates/oni-context",
    "crates/oni-db",
    "crates/oni-tui",
]

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
color-eyre = "0.6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
rusqlite = { version = "0.33", features = ["bundled", "modern_sqlite"] }
reqwest = { version = "0.12", features = ["json"] }
clap = { version = "4", features = ["derive"] }

[package]
name = "oni"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "oni"
path = "src/main.rs"

[dependencies]
oni-core = { path = "crates/oni-core" }
oni-ollama = { path = "crates/oni-ollama" }
oni-agent = { path = "crates/oni-agent" }
oni-tui = { path = "crates/oni-tui" }
oni-db = { path = "crates/oni-db" }
oni-context = { path = "crates/oni-context" }
clap.workspace = true
tokio.workspace = true
color-eyre.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
```

- [ ] **Step 2: Create oni-core with shared types**

`oni/crates/oni-core/Cargo.toml`:
```toml
[package]
name = "oni-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde.workspace = true
serde_json.workspace = true
toml.workspace = true
color-eyre.workspace = true
tracing.workspace = true
ratatui = "0.29"
dirs = "6"
```

`oni/crates/oni-core/src/lib.rs`:
```rust
pub mod config;
pub mod error;
pub mod palette;
pub mod types;
```

`oni/crates/oni-core/src/types.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModelTier {
    Heavy,   // Qwen3.5:35b — complex reasoning, multimodal, research
    Medium,  // Qwen3-Coder:30b — agentic coding, tool use
    General, // GLM-4.7-Flash — fast general chat, quick coding
    Fast,    // Qwen3.5:9b — quick completions, shell commands
    Embed,   // nomic-embed-text — RAG embeddings
}

impl ModelTier {
    pub fn supports_tools(&self) -> bool {
        matches!(self, ModelTier::Medium | ModelTier::General | ModelTier::Fast)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub output: String,
    pub success: bool,
}
```

`oni/crates/oni-core/src/error.rs`:
```rust
use color_eyre::eyre;

pub type Result<T> = eyre::Result<T>;
pub use eyre::eyre;
pub use eyre::WrapErr;
```

`oni/crates/oni-core/src/palette.rs` — from design inspo analysis:
```rust
use ratatui::style::{Color, Modifier, Style};

// Core palette — biotech HUD aesthetic
pub const BG: Color = Color::Black;                       // True black
pub const DATA: Color = Color::Rgb(57, 255, 20);          // Neon green — live data/output
pub const SYSTEM: Color = Color::Rgb(0, 64, 255);         // Electric blue — system identity
pub const ALERT: Color = Color::Rgb(255, 32, 80);         // Red — errors/alerts
pub const STATE: Color = Color::Rgb(204, 255, 0);         // Chartreuse — state announcements
pub const DIM: Color = Color::DarkGray;                   // Background texture
pub const GHOST: Color = Color::Rgb(0, 16, 48);           // Inactive/ghost elements

// Semantic styles
pub fn data_style() -> Style {
    Style::default().fg(DATA).bg(BG)
}

pub fn system_style() -> Style {
    Style::default().fg(SYSTEM).bg(BG)
}

pub fn alert_style() -> Style {
    Style::default().fg(ALERT).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn state_style() -> Style {
    Style::default().fg(STATE).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn dim_style() -> Style {
    Style::default().fg(DIM).bg(BG).add_modifier(Modifier::DIM)
}

pub fn input_style() -> Style {
    Style::default().fg(DATA).bg(BG)
}

pub fn label_style() -> Style {
    Style::default().fg(SYSTEM).bg(BG).add_modifier(Modifier::BOLD)
}
```

Note: `palette.rs` depends on `ratatui` — add `ratatui = "0.29"` to oni-core's deps.

`oni/crates/oni-core/src/config.rs`:
```rust
use crate::error::{Result, WrapErr};
use crate::types::ModelTier;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OniConfig {
    #[serde(default)]
    pub ollama: OllamaConfig,
    #[serde(default)]
    pub models: ModelConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub agent: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_keep_alive")]
    pub keep_alive: i64,  // -1 = forever
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    #[serde(default = "default_heavy")]
    pub heavy: String,
    #[serde(default = "default_medium")]
    pub medium: String,
    #[serde(default = "default_general")]
    pub general: String,
    #[serde(default = "default_fast")]
    pub fast: String,
    #[serde(default = "default_embed")]
    pub embed: String,
    #[serde(default = "default_default_tier")]
    pub default_tier: ModelTier,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default)]
    pub show_thinking: bool,
    #[serde(default)]
    pub show_token_stats: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: usize,
    #[serde(default = "default_context_budget")]
    pub context_budget_tokens: usize,
    #[serde(default)]
    pub allow_write: bool,
    #[serde(default)]
    pub allow_exec: bool,
}

// Defaults
fn default_base_url() -> String { "http://localhost:11434".into() }
fn default_timeout() -> u64 { 300 }
fn default_keep_alive() -> i64 { -1 }
fn default_heavy() -> String { "qwen3.5:35b".into() }
fn default_medium() -> String { "qwen3-coder:30b".into() }
fn default_general() -> String { "glm-4.7-flash:q4_k_m".into() }
fn default_fast() -> String { "qwen3.5:9b".into() }
fn default_embed() -> String { "nomic-embed-text".into() }
fn default_default_tier() -> ModelTier { ModelTier::Medium }
fn default_fps() -> u32 { 30 }
fn default_max_tool_rounds() -> usize { 10 }
fn default_context_budget() -> usize { 8192 }

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            timeout_secs: default_timeout(),
            keep_alive: default_keep_alive(),
        }
    }
}
impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            heavy: default_heavy(),
            medium: default_medium(),
            general: default_general(),
            fast: default_fast(),
            embed: default_embed(),
            default_tier: default_default_tier(),
        }
    }
}
impl Default for UiConfig {
    fn default() -> Self {
        Self { fps: default_fps(), show_thinking: false, show_token_stats: false }
    }
}
impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_tool_rounds: default_max_tool_rounds(),
            context_budget_tokens: default_context_budget(),
            allow_write: false,
            allow_exec: false,
        }
    }
}
impl Default for OniConfig {
    fn default() -> Self {
        Self {
            ollama: OllamaConfig::default(),
            models: ModelConfig::default(),
            ui: UiConfig::default(),
            agent: AgentConfig::default(),
        }
    }
}

/// Load config with hierarchy: defaults → global (~/.config/oni/oni.toml) → project (.oni/oni.toml)
pub fn load_config(project_dir: Option<&Path>) -> Result<OniConfig> {
    let mut config = OniConfig::default();

    // Global config
    if let Some(config_dir) = dirs::config_dir() {
        let global_path = config_dir.join("oni").join("oni.toml");
        if global_path.exists() {
            let text = std::fs::read_to_string(&global_path)
                .wrap_err_with(|| format!("Failed to read {}", global_path.display()))?;
            let global: OniConfig = toml::from_str(&text)
                .wrap_err("Failed to parse global config")?;
            config = merge_config(config, global);
        }
    }

    // Project config
    if let Some(dir) = project_dir {
        let project_path = dir.join(".oni").join("oni.toml");
        if project_path.exists() {
            let text = std::fs::read_to_string(&project_path)
                .wrap_err_with(|| format!("Failed to read {}", project_path.display()))?;
            let project: OniConfig = toml::from_str(&text)
                .wrap_err("Failed to parse project config")?;
            config = merge_config(config, project);
        }
    }

    Ok(config)
}

fn merge_config(base: OniConfig, overlay: OniConfig) -> OniConfig {
    // Simple: overlay wins for all fields. TOML defaults mean
    // unset fields get default values, which is fine.
    overlay
}

pub fn data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("oni");
    std::fs::create_dir_all(&dir)
        .wrap_err("Failed to create data directory")?;
    Ok(dir)
}
```

- [ ] **Step 3: Create stub Cargo.tomls for all other crates**

Each crate needs a minimal `Cargo.toml` + `src/lib.rs` with `// TODO` so the workspace compiles.

Crates: `oni-ollama`, `oni-agent`, `oni-context`, `oni-db`, `oni-tui`.

- [ ] **Step 4: Create main.rs with clap CLI skeleton**

```rust
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

#[derive(Parser)]
#[command(name = "oni", about = "Onboard Native Intelligence — local AI assistant")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive chat
    Chat {
        /// Enable file write tools
        #[arg(long)]
        write: bool,
        /// Enable shell execution tools
        #[arg(long)]
        exec: bool,
        /// Model tier to use (heavy/medium/fast)
        #[arg(long, default_value = "medium")]
        tier: String,
    },
    /// Ask a one-shot question
    Ask {
        /// The question
        question: Vec<String>,
    },
    /// Check system health
    Doctor,
    /// Show/set config
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current config
    Show,
    /// Set a config value
    Set { key: String, value: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt()
        .with_env_filter("oni=info")
        .with_writer(std::fs::File::create("/tmp/oni.log").unwrap_or_else(|_| {
            std::fs::File::create("/dev/null").unwrap()
        }))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Chat { write, exec, tier }) => {
            todo!("Launch TUI")
        }
        Some(Commands::Ask { question }) => {
            todo!("One-shot query")
        }
        Some(Commands::Doctor) => {
            todo!("Health check")
        }
        Some(Commands::Config { action }) => {
            todo!("Config management")
        }
        None => {
            // Default: launch chat
            todo!("Launch TUI")
        }
    }
}
```

- [ ] **Step 5: Verify workspace compiles**

Run: `cd oni && cargo check`
Expected: Compiles with no errors (just `todo!()` warnings)

- [ ] **Step 6: Commit**

```bash
git add oni/
git commit -m "feat: scaffold Cargo workspace with 6 crates + CLI skeleton"
```

---

## Task 2: Ollama Client (oni-ollama)

**Files:**
- Create: `oni/crates/oni-ollama/src/models.rs`
- Create: `oni/crates/oni-ollama/src/client.rs`
- Create: `oni/crates/oni-ollama/src/router.rs`
- Create: `oni/crates/oni-ollama/src/health.rs`
- Modify: `oni/crates/oni-ollama/src/lib.rs`
- Create: `oni/tests/ollama_integration.rs`

Reference: `/Users/brndy.747/Projects/ArchivedProjects/Sparkles/sparkles/ollama_client.py` — port the patterns.

**CRITICAL:** All API calls use `stream: false`. No streaming.

- [ ] **Step 1: Write integration test (skip if Ollama not running)**

`oni/tests/ollama_integration.rs`:
```rust
use oni_ollama::{OllamaClient, ChatRequest, ChatMessage};

#[tokio::test]
async fn test_health_check() {
    let client = OllamaClient::default();
    match client.health_check().await {
        Ok(models) => assert!(!models.is_empty(), "Should have at least one model"),
        Err(_) => eprintln!("SKIP: Ollama not running"),
    }
}

#[tokio::test]
async fn test_batch_chat() {
    let client = OllamaClient::default();
    if client.health_check().await.is_err() {
        eprintln!("SKIP: Ollama not running");
        return;
    }
    let req = ChatRequest {
        model: "qwen3.5:9b".into(),
        messages: vec![ChatMessage {
            role: "user".into(),
            content: "Reply with exactly: HELLO".into(),
        }],
        stream: false,
        keep_alive: Some(-1),
        options: None,
    };
    let resp = client.chat(&req).await.unwrap();
    assert!(resp.message.content.contains("HELLO"));
    assert!(resp.done);
}
```

- [ ] **Step 2: Implement API models**

`oni/crates/oni-ollama/src/models.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub model: String,
    pub message: ChatMessage,
    pub done: bool,
    pub total_duration: Option<u64>,
    pub prompt_eval_count: Option<u64>,
    pub eval_count: Option<u64>,
    pub eval_duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbedRequest {
    pub model: String,
    pub input: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedResponse {
    pub model: String,
    pub embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TagsResponse {
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
}
```

- [ ] **Step 3: Implement client**

`oni/crates/oni-ollama/src/client.rs`:
```rust
use crate::models::*;
use oni_core::error::{Result, eyre, WrapErr};
use reqwest::Client;
use std::time::Duration;

pub struct OllamaClient {
    http: Client,
    base_url: String,
}

impl OllamaClient {
    pub fn new(base_url: &str, timeout_secs: u64) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to build HTTP client");
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn health_check(&self) -> Result<Vec<ModelInfo>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp: TagsResponse = self.http
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .wrap_err("Ollama not running. Start with: ollama serve")?
            .json()
            .await
            .wrap_err("Invalid response from Ollama")?;
        Ok(resp.models)
    }

    pub async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.base_url);
        let resp = self.http
            .post(&url)
            .json(request)
            .send()
            .await
            .wrap_err("Failed to connect to Ollama")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(eyre!("Ollama API error ({}): {}", status, body));
        }

        resp.json().await.wrap_err("Failed to parse Ollama response")
    }

    pub async fn embed(&self, request: &EmbedRequest) -> Result<EmbedResponse> {
        let url = format!("{}/api/embed", self.base_url);
        let resp = self.http
            .post(&url)
            .json(request)
            .send()
            .await
            .wrap_err("Failed to connect to Ollama for embeddings")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(eyre!("Ollama embed error ({}): {}", status, body));
        }

        resp.json().await.wrap_err("Failed to parse embed response")
    }

    pub async fn has_model(&self, model_name: &str) -> Result<bool> {
        let models = self.health_check().await?;
        let base = model_name.split(':').next().unwrap_or(model_name);
        Ok(models.iter().any(|m| m.name == model_name || m.name.starts_with(base)))
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new("http://localhost:11434", 300)
    }
}
```

- [ ] **Step 4: Implement multi-model router**

`oni/crates/oni-ollama/src/router.rs`:
```rust
use crate::client::OllamaClient;
use crate::models::*;
use oni_core::config::ModelConfig;
use oni_core::error::Result;
use oni_core::types::ModelTier;
use std::collections::HashMap;

pub struct ModelRouter {
    client: OllamaClient,
    models: ModelConfig,
    keep_alive: i64,
    default_tier: ModelTier,
}

impl ModelRouter {
    pub fn new(client: OllamaClient, models: ModelConfig, keep_alive: i64) -> Self {
        let default_tier = models.default_tier;
        Self { client, models, keep_alive, default_tier }
    }

    pub fn model_name(&self, tier: ModelTier) -> &str {
        match tier {
            ModelTier::Heavy => &self.models.heavy,
            ModelTier::Medium => &self.models.medium,
            ModelTier::General => &self.models.general,
            ModelTier::Fast => &self.models.fast,
            ModelTier::Embed => &self.models.embed,
        }
    }

    pub async fn chat(&self, tier: ModelTier, messages: Vec<ChatMessage>) -> Result<ChatResponse> {
        let request = ChatRequest {
            model: self.model_name(tier).to_string(),
            messages,
            stream: false,
            keep_alive: Some(self.keep_alive),
            options: self.default_options(tier),
        };
        self.client.chat(&request).await
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbedRequest {
            model: self.model_name(ModelTier::Embed).to_string(),
            input: text.to_string(),
        };
        let resp = self.client.embed(&request).await?;
        Ok(resp.embeddings.into_iter().next().unwrap_or_default())
    }

    pub async fn check_all_models(&self) -> HashMap<ModelTier, bool> {
        let mut results = HashMap::new();
        for tier in [ModelTier::Heavy, ModelTier::Medium, ModelTier::General, ModelTier::Fast, ModelTier::Embed] {
            let available = self.client.has_model(self.model_name(tier)).await.unwrap_or(false);
            results.insert(tier, available);
        }
        results
    }

    fn default_options(&self, tier: ModelTier) -> Option<HashMap<String, serde_json::Value>> {
        let mut opts = HashMap::new();
        match tier {
            ModelTier::Heavy => {
                opts.insert("temperature".into(), serde_json::json!(0.3));
                opts.insert("num_ctx".into(), serde_json::json!(8192));
            }
            ModelTier::Medium => {
                opts.insert("temperature".into(), serde_json::json!(0.2));
                opts.insert("num_ctx".into(), serde_json::json!(8192));
            }
            ModelTier::General => {
                opts.insert("temperature".into(), serde_json::json!(0.3));
                opts.insert("num_ctx".into(), serde_json::json!(8192));
            }
            ModelTier::Fast => {
                opts.insert("temperature".into(), serde_json::json!(0.1));
                opts.insert("num_ctx".into(), serde_json::json!(4096));
            }
            ModelTier::Embed => return None,
        }
        Some(opts)
    }
}
```

- [ ] **Step 5: Wire up lib.rs, run integration tests**

Run: `cd oni && cargo test --test ollama_integration`
Expected: Tests pass if Ollama running, skip gracefully if not.

- [ ] **Step 6: Commit**

```bash
git add oni/crates/oni-ollama/ oni/tests/ollama_integration.rs
git commit -m "feat: Ollama client with batch mode + multi-model router"
```

---

## Task 3: SQLite Database (oni-db)

**Files:**
- Create: `oni/crates/oni-db/src/schema.rs`
- Create: `oni/crates/oni-db/src/conversations.rs`
- Create: `oni/crates/oni-db/src/tool_events.rs`
- Modify: `oni/crates/oni-db/src/lib.rs`
- Create: `oni/tests/db_schema.rs`

Port the SQLite schema from the TS version. Same 3 tables.

- [ ] **Step 1: Write schema tests**

```rust
#[test]
fn test_schema_creation() {
    let db = oni_db::Database::open_in_memory().unwrap();
    // Tables should exist
    let count: i64 = db.conn().query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('conversations','messages','tool_events')",
        [], |r| r.get(0)
    ).unwrap();
    assert_eq!(count, 3);
}

#[test]
fn test_insert_and_query_conversation() {
    let db = oni_db::Database::open_in_memory().unwrap();
    let conv_id = db.create_conversation("/tmp/test").unwrap();
    db.add_message(&conv_id, "user", "hello").unwrap();
    db.add_message(&conv_id, "assistant", "hi there").unwrap();
    let messages = db.get_messages(&conv_id).unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].content, "hello");
}
```

- [ ] **Step 2: Implement schema + Database struct**

Use rusqlite with WAL mode, foreign keys, FTS5. Schema DDL (ported from TS `packages/db/src/migrations/0001_initial.ts`):

```rust
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

    CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(content, content_rowid='rowid');
"#;
```

```rust
pub struct Database {
    conn: rusqlite::Connection,
}

impl Database {
    pub fn open(path: &std::path::Path) -> oni_core::error::Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> oni_core::error::Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &rusqlite::Connection {
        &self.conn
    }
}
```

- [ ] **Step 3: Implement conversation + message CRUD**

```rust
impl Database {
    pub fn create_conversation(&self, project_dir: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO conversations (conv_id, project_dir) VALUES (?1, ?2)",
            rusqlite::params![id, project_dir],
        )?;
        Ok(id)
    }

    pub fn add_message(&self, conv_id: &str, role: &str, content: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let tokens = (content.len() / 4) as i64; // rough estimate
        self.conn.execute(
            "INSERT INTO messages (msg_id, conv_id, role, content, tokens) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, conv_id, role, content, tokens],
        )?;
        Ok(id)
    }

    pub fn get_messages(&self, conv_id: &str) -> Result<Vec<StoredMessage>> {
        let mut stmt = self.conn.prepare(
            "SELECT msg_id, role, content, tokens, timestamp FROM messages WHERE conv_id = ?1 ORDER BY timestamp"
        )?;
        let rows = stmt.query_map([conv_id], |row| {
            Ok(StoredMessage {
                msg_id: row.get(0)?,
                role: row.get(1)?,
                content: row.get(2)?,
                tokens: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

pub struct StoredMessage {
    pub msg_id: String,
    pub role: String,
    pub content: String,
    pub tokens: i64,
    pub timestamp: String,
}
```

Add `uuid = { version = "1", features = ["v4"] }` to oni-db's `Cargo.toml`.

- [ ] **Step 4: Implement tool_events logging**

```rust
impl Database {
    pub fn log_tool_event(
        &self,
        session_id: &str,
        tool_name: &str,
        args_json: &str,
        result_json: &str,
        latency_ms: i64,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO tool_events (session_id, tool_name, args_json, result_json, latency_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![session_id, tool_name, args_json, result_json, latency_ms],
        )?;
        Ok(())
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cd oni && cargo test --test db_schema`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add oni/crates/oni-db/ oni/tests/db_schema.rs
git commit -m "feat: SQLite database with conversations, messages, tool events"
```

---

## Task 4: Agent Core (oni-agent)

**Files:**
- Create: `oni/crates/oni-agent/src/agent.rs`
- Create: `oni/crates/oni-agent/src/conversation.rs`
- Create: `oni/crates/oni-agent/src/system_prompt.rs`
- Create: `oni/crates/oni-agent/src/budget.rs`
- Create: `oni/crates/oni-agent/src/tools/mod.rs`
- Create: `oni/crates/oni-agent/src/tools/read_file.rs`
- Create: `oni/crates/oni-agent/src/tools/write_file.rs`
- Create: `oni/crates/oni-agent/src/tools/bash.rs`
- Create: `oni/crates/oni-agent/src/tools/list_dir.rs`
- Modify: `oni/crates/oni-agent/src/lib.rs`
- Create: `oni/tests/agent_loop.rs`

The agent loop is the core of ONI. Port from `packages/agent/src/client.ts`. Key difference: Ollama uses `stream: false`, so the loop is simpler — send messages, get full response, check for tool calls, execute, repeat.

**IMPORTANT:** DeepSeek-R1 has limited tool-use. The agent should only use Medium/Fast tiers for tool-calling tasks. Heavy tier is for pure reasoning.

- [ ] **Step 1: Define tool trait and registry**

```rust
// tools/mod.rs
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn schema(&self) -> serde_json::Value;
    fn execute(&self, args: serde_json::Value) -> oni_core::error::Result<String>;
}

pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
}
```

- [ ] **Step 2: Implement the 4 tools**

Port from the TS versions. Each tool: `read_file`, `write_file` (gated by `allow_write`), `bash` (gated by `allow_exec`), `list_directory`.

- [ ] **Step 3: Implement conversation history manager**

```rust
// conversation.rs — tracks messages, provides Ollama-compatible message list
pub struct Conversation {
    messages: Vec<oni_core::types::Message>,
    system_prompt: String,
}
```

- [ ] **Step 4: Implement the agent loop**

```rust
// agent.rs
pub struct Agent {
    router: Arc<ModelRouter>,
    tools: ToolRegistry,
    conversation: Conversation,
    budget: BudgetTracker,
    max_tool_rounds: usize,
}

impl Agent {
    /// Run one turn: send user message, handle tool calls, return final response.
    /// Communicates progress via the provided sender.
    pub async fn run_turn(
        &mut self,
        user_message: &str,
        tier: ModelTier,
        progress_tx: tokio::sync::mpsc::Sender<AgentEvent>,
    ) -> Result<String> {
        // 1. Add user message to conversation
        // 2. Loop up to max_tool_rounds:
        //    a. Send conversation to Ollama (batch mode)
        //    b. Parse response for tool calls
        //    c. If no tool calls, return response text
        //    d. Execute tools, add results to conversation
        //    e. Send progress event via channel
        // 3. Return final response
    }
}
```

- [ ] **Step 5: Write agent loop tests (mock Ollama with a simple HTTP server)**

Test: agent processes a response with no tool calls.
Test: agent executes a read_file tool call and feeds result back.
Test: agent stops after max_tool_rounds.

- [ ] **Step 6: Run tests**

Run: `cd oni && cargo test --test agent_loop`

- [ ] **Step 7: Commit**

```bash
git add oni/crates/oni-agent/ oni/tests/agent_loop.rs
git commit -m "feat: agent loop with 4 tools + multi-model routing"
```

---

## Task 5: Context System (oni-context)

**Files:**
- Create: `oni/crates/oni-context/src/walker.rs`
- Create: `oni/crates/oni-context/src/indexer.rs`
- Create: `oni/crates/oni-context/src/retriever.rs`
- Create: `oni/crates/oni-context/src/embeddings.rs`
- Modify: `oni/crates/oni-context/src/lib.rs`
- Create: `oni/tests/context_index.rs`

Port from the TS `packages/context/`. Same approach: walk files, extract symbols via regex, index into SQLite FTS5, retrieve with token budget.

New addition: `embeddings.rs` uses `nomic-embed-text` via Ollama for semantic search alongside FTS5.

- [ ] **Step 1: Implement file walker** (.gitignore aware, skip binaries/large files)

- [ ] **Step 2: Implement regex symbol extraction** (functions, classes, structs for 15+ languages)

- [ ] **Step 3: Implement FTS5 indexer**

- [ ] **Step 4: Implement token-budgeted retriever**

- [ ] **Step 5: Implement embedding client** (calls `oni-ollama` embed endpoint)

- [ ] **Step 6: Write tests**

Run: `cd oni && cargo test --test context_index`

- [ ] **Step 7: Commit**

```bash
git add oni/crates/oni-context/ oni/tests/context_index.rs
git commit -m "feat: context system with file walker, FTS5 indexer, RAG embeddings"
```

---

## Task 6: TUI — Ratatui Core (oni-tui)

**Files:**
- Create: `oni/crates/oni-tui/src/app.rs`
- Create: `oni/crates/oni-tui/src/event.rs`
- Create: `oni/crates/oni-tui/src/theme.rs`
- Create: `oni/crates/oni-tui/src/ui/mod.rs`
- Create: `oni/crates/oni-tui/src/ui/chat.rs`
- Create: `oni/crates/oni-tui/src/ui/input.rs`
- Create: `oni/crates/oni-tui/src/ui/status.rs`
- Create: `oni/crates/oni-tui/src/ui/thinking.rs`
- Create: `oni/crates/oni-tui/src/ui/splash.rs`
- Modify: `oni/crates/oni-tui/src/lib.rs`

This is the biggest task. Elm Architecture: `App` holds all state, `AppEvent` enum for all inputs, `update()` mutates state, `view()` renders.

**Design reference:** `/Users/brndy.747/Projects/ONI/docs/Vision/Images/` — biotech HUD aesthetic. See palette in `oni-core/src/palette.rs`.

**Key constraints from design analysis:**
- True black background, no bordered panes
- Neon green for data, electric blue for system labels, red for alerts
- ALL_CAPS_SNAKE_CASE for all UI chrome labels
- Large typography as primary widgets
- No streaming — show a thinking indicator, then render full response
- Dot-grid backgrounds via braille characters at low intensity

- [ ] **Step 1: Define AppEvent enum**

```rust
// event.rs
pub enum AppEvent {
    Key(crossterm::event::KeyEvent),
    Tick,
    LlmResponse(String),        // Complete response from Ollama
    LlmError(String),
    LlmThinking,                // Started waiting for response
    ToolExec(String, String),   // tool_name, status
    Resize(u16, u16),
    Quit,
}
```

- [ ] **Step 2: Implement App struct (the Model)**

```rust
// app.rs
pub struct App {
    pub messages: Vec<DisplayMessage>,
    pub input: tui_textarea::TextArea<'static>,
    pub scroll_offset: u16,
    pub is_thinking: bool,
    pub current_model: ModelTier,
    pub token_count: u64,
    pub last_eval_duration_ms: u64,
    pub mode: AppMode,
    pub should_quit: bool,
}

pub enum AppMode {
    Splash,
    Chat,
    ModelSelect,
}
```

- [ ] **Step 3: Implement the event loop**

Main loop with `tokio::select!`:
- Crossterm key events
- Tick timer (30fps from config)
- mpsc receiver for LLM responses

- [ ] **Step 4: Implement theme.rs** — styles, ALL_CAPS formatters

- [ ] **Step 5: Implement chat.rs** — message list with manual scroll tracking

- [ ] **Step 6: Implement input.rs** — tui-textarea wrapper

- [ ] **Step 7: Implement status.rs** — model name, token count, timing

- [ ] **Step 8: Implement thinking.rs** — throbber spinner + "PROCESSING" label while waiting

- [ ] **Step 9: Implement splash.rs** — dot-matrix ONI logo on startup (braille canvas)

- [ ] **Step 10: Implement root layout in ui/mod.rs**

```
┌──────────────────────────────────────────────┐
│ SYSTEM_ONI                    MODEL: MEDIUM  │  ← status bar (1 line)
├──────────────────────────────────────────────┤
│                                              │
│  [chat messages / splash screen]             │  ← main area (fills remaining)
│                                              │
│                                              │
├──────────────────────────────────────────────┤
│ > [input area]                               │  ← input (3 lines)
├──────────────────────────────────────────────┤
│ TOKENS: 1,234  EVAL: 2.3s  TIER: MEDIUM     │  ← footer stats (1 line)
└──────────────────────────────────────────────┘
```

No borders between sections — just colour contrast. Status bar uses `system_style()`, main area uses `data_style()`, footer uses `dim_style()`.

- [ ] **Step 11: Wire up App::update() to handle all events**

- [ ] **Step 12: Verify TUI launches and renders splash**

Run: `cd oni && cargo run -- chat`
Expected: TUI renders with splash screen, can type and quit with Ctrl+C.

- [ ] **Step 13: Commit**

```bash
git add oni/crates/oni-tui/
git commit -m "feat: Ratatui TUI with biotech HUD aesthetic + chat layout"
```

---

## Task 7: Wire Everything Together

**Files:**
- Modify: `oni/src/main.rs`
- Modify: `oni/crates/oni-tui/src/app.rs`

Connect all crates: main.rs creates the Ollama client, agent, database, and passes them to the TUI. The TUI spawns agent turns in Tokio tasks and receives responses via channels.

- [ ] **Step 1: Wire main.rs**

```rust
// main.rs — chat command handler
async fn run_chat(write: bool, exec: bool, tier: &str) -> Result<()> {
    let config = oni_core::config::load_config(Some(&std::env::current_dir()?))?;
    let client = OllamaClient::new(&config.ollama.base_url, config.ollama.timeout_secs);
    let router = Arc::new(ModelRouter::new(client, config.models.clone(), config.ollama.keep_alive));
    let db = oni_db::Database::open(&oni_core::config::data_dir()?.join("oni.db"))?;

    let mut agent_config = config.agent.clone();
    agent_config.allow_write = write;
    agent_config.allow_exec = exec;

    oni_tui::run(router, db, agent_config, config.ui).await
}
```

- [ ] **Step 2: Wire TUI → Agent via channels**

When user submits input:
1. TUI sets `is_thinking = true`
2. Spawns `tokio::spawn` with agent.run_turn()
3. Agent sends `AgentEvent` progress updates via channel
4. On completion, sends `LlmResponse` event
5. TUI renders full response

- [ ] **Step 3: Implement `oni doctor` command**

Check: Ollama running? Each model available? Disk space? Config valid?

- [ ] **Step 4: End-to-end test — type a question, get a response**

Run: `cd oni && cargo run -- chat`
Type: "What is 2+2?"
Expected: Thinking indicator → full response appears → token stats update

- [ ] **Step 5: Commit**

```bash
git add oni/
git commit -m "feat: wire agent + TUI + Ollama for end-to-end chat"
```

---

## Task 8: Custom Widgets (oni-tui/widgets/)

**Files:**
- Create: `oni/crates/oni-tui/src/widgets/arc.rs`
- Create: `oni/crates/oni-tui/src/widgets/spectrum.rs`
- Create: `oni/crates/oni-tui/src/widgets/big_text.rs`
- Create: `oni/crates/oni-tui/src/widgets/glitch.rs`
- Create: `oni/crates/oni-tui/src/widgets/mod.rs`

These are the visual flourishes from the design inspo. Not functional — purely aesthetic. Lower priority than the core chat, but they make ONI look distinctive.

- [ ] **Step 1: Arc widget** — braille canvas, draws partial circular arcs as framing devices

- [ ] **Step 2: Spectrum widget** — dense bar chart visualiser (token rate, response timing)

- [ ] **Step 3: Big text widget** — oversized numeric display for key metrics

- [ ] **Step 4: Glitch widget** — random pixel-noise blocks as decorative motifs

- [ ] **Step 5: Integrate widgets into splash screen and status areas**

- [ ] **Step 6: Commit**

```bash
git add oni/crates/oni-tui/src/widgets/
git commit -m "feat: custom HUD widgets — arc, spectrum, big text, glitch"
```

---

## Task 9: Markdown Rendering + Code Highlighting

**Files:**
- Modify: `oni/crates/oni-tui/src/ui/chat.rs`

Use `tui-markdown` with `highlight-code` feature for syntax highlighting in responses. This is critical for a coding assistant.

- [ ] **Step 1: Add tui-markdown + syntect dependencies**

- [ ] **Step 2: Implement markdown → Ratatui Text conversion**

Parse the LLM response markdown into styled `Text` blocks. Code blocks get syntax highlighting. Headers get `STATE` colour. Inline code gets `DATA` colour.

- [ ] **Step 3: Test with a code-heavy response**

- [ ] **Step 4: Commit**

```bash
git add oni/crates/oni-tui/
git commit -m "feat: markdown rendering with syntax-highlighted code blocks"
```

---

## Task 10: Cleanup + Polish

**Files:**
- Delete: `packages/` (old TS code)
- Delete: `package.json`, `tsconfig.json`, `biome.json` etc.
- Modify: `.gitignore` (add Rust targets)
- Create: `oni.toml` (default config shipped with binary)

- [ ] **Step 1: Update .gitignore for Rust**

Add: `oni/target/`, `*.swp`, `*.swo`

- [ ] **Step 2: Create default oni.toml**

```toml
[ollama]
base_url = "http://localhost:11434"
timeout_secs = 300
keep_alive = -1

[models]
heavy = "qwen3.5:35b"
medium = "qwen3-coder:30b"
general = "glm-4.7-flash:q4_k_m"
fast = "qwen3.5:9b"
embed = "nomic-embed-text"
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
```

- [ ] **Step 3: Delete old TS packages**

Remove: `packages/`, `package.json`, `package-lock.json`, `tsconfig*.json`, `biome.json`, `vitest.config.ts`, `node_modules/`

- [ ] **Step 4: Move oni/ contents to repo root**

The workspace is currently nested under `oni/`. Move everything up so `Cargo.toml` is at the repo root.

- [ ] **Step 5: Full build + test cycle**

Run: `cargo build --release && cargo test`

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: ONI v2 — Rust/Ratatui + Ollama, delete legacy TypeScript"
```

---

## Dependency Graph

```
Task 1 (Scaffold)
  ├── Task 2 (Ollama Client)
  │     └── Task 4 (Agent) ──→ Task 7 (Wire Together)
  ├── Task 3 (Database)    ──→ Task 7
  ├── Task 5 (Context)     ──→ Task 7
  └── Task 6 (TUI)         ──→ Task 7 ──→ Task 8 (Widgets)
                                       ──→ Task 9 (Markdown)
                                       ──→ Task 10 (Cleanup)
```

**Parallelisable:** Tasks 2, 3, 5, 6 can all run in parallel after Task 1.
**Sequential:** Task 7 depends on 2+3+5+6. Tasks 8+9+10 depend on 7.

---

## Notes for Implementation

1. **No streaming** — all Ollama calls use `stream: false`. Show throbber while waiting.
2. **Qwen3.5 thinking blocks** — if thinking mode is enabled, strip `<think>...</think>` from display or show separately if `show_thinking` is enabled.
3. **Tool use** — Medium (Qwen3-Coder), General (GLM-4.7-Flash), and Fast (Qwen3.5:9b) tiers support tool calling. Heavy (Qwen3.5:35b) supports it too but reserve for complex reasoning.
4. **LIT detection** — port from Sparkles (`/Users/brndy.747/Projects/ArchivedProjects/Sparkles/sparkles/thinking_guard.py`) in a future iteration. Not needed for batch mode (timeout handles it), but useful if we ever add streaming.
5. **Keep models resident** — `keep_alive: -1` prevents cold starts. With 128GB RAM, all 5 models (~69GB) fit simultaneously with 59GB headroom.
6. **Logging** — `tracing` outputs to `/tmp/oni.log` only. Never stdout (conflicts with TUI).
7. **Config merge** — currently simple overlay. If users need field-level merging, refactor later. YAGNI for now.
