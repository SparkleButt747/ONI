use crate::error::{Result, WrapErr};
use crate::types::{AutonomyLevel, ModelTier};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OniConfig {
    #[serde(default, alias = "ollama")]
    pub server: ServerConfig,
    #[serde(default)]
    pub models: ModelConfig,
    #[serde(default)]
    pub ui: UiConfig,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(default)]
    pub neo4j: Neo4jConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jConfig {
    #[serde(default = "default_neo4j_uri")]
    pub uri: String,
    #[serde(default)]
    pub enabled: bool,
}

fn default_neo4j_uri() -> String {
    "bolt://localhost:7687".into()
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            uri: default_neo4j_uri(),
            enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_tier_urls")]
    pub tier_urls: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default = "default_models_dir")]
    pub models_dir: String,
    #[serde(default)]
    pub tiers: std::collections::HashMap<String, TierServerConfig>,
    /// Memory headroom to keep free (bytes). Default: 4GB.
    #[serde(default = "default_memory_headroom")]
    pub memory_headroom: u64,
    /// GGUF file size → runtime memory multiplier. Default: 1.3.
    #[serde(default = "default_memory_multiplier")]
    pub memory_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierServerConfig {
    /// GGUF filename, relative to models_dir.
    pub gguf: String,
    #[serde(default = "default_ctx_size")]
    pub ctx_size: u32,
    #[serde(default)]
    pub cache_type_k: Option<String>,
    #[serde(default)]
    pub cache_type_v: Option<String>,
    #[serde(default = "default_true")]
    pub flash_attn: bool,
    #[serde(default = "default_threads")]
    pub threads: u32,
    #[serde(default = "default_threads_batch")]
    pub threads_batch: u32,
    #[serde(default = "default_parallel")]
    pub parallel: u32,
    #[serde(default = "default_gpu_layers")]
    pub gpu_layers: u32,
    #[serde(default)]
    pub extra_args: Vec<String>,
}

fn default_models_dir() -> String {
    "~/.cache/llama.cpp/models".into()
}
fn default_ctx_size() -> u32 {
    32768
}
fn default_true() -> bool {
    true
}
fn default_threads() -> u32 {
    8
}
fn default_threads_batch() -> u32 {
    16
}
fn default_parallel() -> u32 {
    1
}
fn default_gpu_layers() -> u32 {
    99
}
fn default_memory_headroom() -> u64 {
    4 * 1024 * 1024 * 1024 // 4 GB
}
fn default_memory_multiplier() -> f64 {
    1.3
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
    /// Per-tier reasoning / sampling overrides.
    #[serde(default)]
    pub heavy_reasoning: TierReasoningConfig,
    #[serde(default)]
    pub medium_reasoning: TierReasoningConfig,
    #[serde(default)]
    pub general_reasoning: TierReasoningConfig,
    #[serde(default)]
    pub fast_reasoning: TierReasoningConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    #[serde(default = "default_fps")]
    pub fps: u32,
    #[serde(default)]
    pub show_thinking: bool,
    #[serde(default = "default_show_token_stats")]
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
    #[serde(default)]
    pub autonomy: AutonomyLevel,
    /// Per-session token budget (0 = unlimited).
    #[serde(default)]
    pub session_budget: u64,
    /// Monthly token limit (0 = unlimited). Persisted to budget.json.
    #[serde(default)]
    pub monthly_limit: u64,
    #[serde(default)]
    pub compaction: CompactionConfig,
    #[serde(default)]
    pub reasoning: ReasoningConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Trigger compaction when estimated tokens exceed this.
    #[serde(default = "default_token_threshold")]
    pub token_threshold: u64,
    /// Trigger when message count exceeds this.
    #[serde(default = "default_message_threshold")]
    pub message_threshold: usize,
    /// Number of recent messages to keep during compaction.
    #[serde(default = "default_retention_window")]
    pub retention_window: usize,
    /// Max tokens for the compaction summary.
    #[serde(default = "default_summary_max_tokens")]
    pub summary_max_tokens: usize,
}

fn default_token_threshold() -> u64 {
    19660
} // 60% of 32K
fn default_message_threshold() -> usize {
    40
}
fn default_retention_window() -> usize {
    4
}
fn default_summary_max_tokens() -> usize {
    500
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            token_threshold: default_token_threshold(),
            message_threshold: default_message_threshold(),
            retention_window: default_retention_window(),
            summary_max_tokens: default_summary_max_tokens(),
        }
    }
}

/// Per-tier sampling / inference overrides.
/// All fields are optional; absent fields fall back to router defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierReasoningConfig {
    pub temperature: Option<f32>,
    pub num_ctx: Option<u32>,
    /// Max tokens to generate. -1 means unlimited.
    pub num_predict: Option<i32>,
}

impl Default for TierReasoningConfig {
    fn default() -> Self {
        Self {
            temperature: None,
            num_ctx: None,
            num_predict: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// Enable extended thinking (models emit <think> blocks).
    #[serde(default)]
    pub enabled: bool,
    /// Effort level: low, medium, high.
    #[serde(default = "default_effort")]
    pub effort: String,
    /// Don't strip <think> blocks from display (show reasoning to user).
    #[serde(default)]
    pub show_thinking: bool,
    /// Global temperature override (applied to all tiers unless per-tier overrides present).
    pub temperature: Option<f32>,
    /// Global context window override.
    pub num_ctx: Option<u32>,
    /// Global max tokens to generate (-1 = unlimited).
    pub num_predict: Option<i32>,
}

fn default_effort() -> String {
    "medium".into()
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            effort: default_effort(),
            show_thinking: false,
            temperature: None,
            num_ctx: None,
            num_predict: None,
        }
    }
}

// Defaults
fn default_base_url() -> String {
    "http://localhost:8082".into()
}
fn default_timeout() -> u64 {
    300
}
fn default_tier_urls() -> std::collections::HashMap<String, String> {
    let mut m = std::collections::HashMap::new();
    m.insert("heavy".into(), "http://localhost:8081".into());
    m.insert("medium".into(), "http://localhost:8082".into());
    m.insert("general".into(), "http://localhost:8083".into());
    m.insert("fast".into(), "http://localhost:8084".into());
    m.insert("embed".into(), "http://localhost:8085".into());
    m
}
fn default_heavy() -> String {
    "qwen3.5:35b".into()
}
fn default_medium() -> String {
    "qwen3-coder:30b".into()
}
fn default_general() -> String {
    "glm-4.7-flash:q4_k_m".into()
}
fn default_fast() -> String {
    "qwen3.5:9b".into()
}
fn default_embed() -> String {
    "nomic-embed-text".into()
}
fn default_default_tier() -> ModelTier {
    ModelTier::Medium
}
fn default_fps() -> u32 {
    30
}
fn default_show_token_stats() -> bool {
    true
}
fn default_max_tool_rounds() -> usize {
    10
}
fn default_context_budget() -> usize {
    8192
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            timeout_secs: default_timeout(),
            tier_urls: default_tier_urls(),
            auto_start: false,
            models_dir: default_models_dir(),
            tiers: std::collections::HashMap::new(),
            memory_headroom: default_memory_headroom(),
            memory_multiplier: default_memory_multiplier(),
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
            heavy_reasoning: TierReasoningConfig::default(),
            medium_reasoning: TierReasoningConfig::default(),
            general_reasoning: TierReasoningConfig::default(),
            fast_reasoning: TierReasoningConfig::default(),
        }
    }
}


impl Default for UiConfig {
    fn default() -> Self {
        Self {
            fps: default_fps(),
            show_thinking: false,
            show_token_stats: default_show_token_stats(),
        }
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_tool_rounds: default_max_tool_rounds(),
            context_budget_tokens: default_context_budget(),
            allow_write: false,
            allow_exec: false,
            autonomy: AutonomyLevel::default(),
            session_budget: 0,
            monthly_limit: 0,
            compaction: CompactionConfig::default(),
            reasoning: ReasoningConfig::default(),
        }
    }
}

impl Default for OniConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            models: ModelConfig::default(),
            ui: UiConfig::default(),
            agent: AgentConfig::default(),
            neo4j: Neo4jConfig::default(),
        }
    }
}

fn merge_toml(base: &mut toml::Value, overlay: &toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, value) in overlay_table {
                if let Some(base_value) = base_table.get_mut(key) {
                    merge_toml(base_value, value);
                } else {
                    base_table.insert(key.clone(), value.clone());
                }
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

/// Load config with hierarchy: defaults -> global (~/.config/oni/oni.toml) -> project (.oni/oni.toml)
pub fn load_config(project_dir: Option<&Path>) -> Result<OniConfig> {
    // Start with defaults serialised to a TOML value so we can merge into it.
    let mut merged: toml::Value =
        toml::Value::try_from(OniConfig::default()).wrap_err("Failed to serialise default config")?;

    // Global config
    if let Some(config_dir) = dirs::config_dir() {
        let global_path = config_dir.join("oni").join("oni.toml");
        if global_path.exists() {
            let text = std::fs::read_to_string(&global_path)
                .wrap_err_with(|| format!("Failed to read {}", global_path.display()))?;
            let global_val: toml::Value =
                toml::from_str(&text).wrap_err("Failed to parse global config")?;
            merge_toml(&mut merged, &global_val);
        }
    }

    // Project config — merged over global, not replacing it
    // Check .oni/oni.toml first, then fall back to ./oni.toml at project root
    if let Some(dir) = project_dir {
        let dot_oni_path = dir.join(".oni").join("oni.toml");
        let root_path = dir.join("oni.toml");
        let project_path = if dot_oni_path.exists() {
            Some(dot_oni_path)
        } else if root_path.exists() {
            Some(root_path)
        } else {
            None
        };

        if let Some(path) = project_path {
            let text = std::fs::read_to_string(&path)
                .wrap_err_with(|| format!("Failed to read {}", path.display()))?;
            let project_val: toml::Value =
                toml::from_str(&text).wrap_err("Failed to parse project config")?;
            merge_toml(&mut merged, &project_val);
        }
    }

    // Backward compat: if the TOML has [ollama] but not [server], warn the user.
    if let Some(table) = merged.as_table() {
        if table.contains_key("ollama") && !table.contains_key("server") {
            warn!("[ollama] config section is deprecated — rename to [server] in your oni.toml");
        }
    }

    let config: OniConfig = merged.try_into().wrap_err("Failed to deserialise merged config")?;
    Ok(config)
}

pub fn data_dir() -> Result<PathBuf> {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("oni");
    std::fs::create_dir_all(&dir).wrap_err("Failed to create data directory")?;
    Ok(dir)
}

impl ModelConfig {
    pub fn model_for_tier(&self, tier: ModelTier) -> &str {
        match tier {
            ModelTier::Heavy => &self.heavy,
            ModelTier::Medium => &self.medium,
            ModelTier::General => &self.general,
            ModelTier::Fast => &self.fast,
            ModelTier::Embed => &self.embed,
        }
    }

    /// Return the per-tier sampling/inference config for the given tier.
    pub fn tier_reasoning(&self, tier: ModelTier) -> &TierReasoningConfig {
        match tier {
            ModelTier::Heavy => &self.heavy_reasoning,
            ModelTier::Medium => &self.medium_reasoning,
            ModelTier::General => &self.general_reasoning,
            ModelTier::Fast => &self.fast_reasoning,
            // Embed never uses sampling options; return any default (all Nones).
            ModelTier::Embed => &self.heavy_reasoning,
        }
    }
}

/// Write `key = value` to the appropriate config file.
///
/// Writes to `./.oni/oni.toml` (project) if it exists, otherwise to the global
/// `~/.config/oni/oni.toml`. Creates the global config if neither exists.
///
/// Supports dotted keys: `models.heavy` → `[models]\nheavy = ...`.
/// Missing parent tables are created automatically.
///
/// Returns `(path_written, is_project)`.
pub fn config_set(key: &str, value: &str) -> Result<(PathBuf, bool)> {
    // Decide target file: project config wins if present.
    let project_path = std::env::current_dir()
        .ok()
        .map(|d| d.join(".oni").join("oni.toml"));
    let global_path = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("oni")
        .join("oni.toml");

    let (target_path, is_project) = match project_path {
        Some(ref p) if p.exists() => (p.clone(), true),
        _ => (global_path, false),
    };

    // Ensure parent directory exists.
    if let Some(parent) = target_path.parent() {
        std::fs::create_dir_all(parent).wrap_err("Failed to create config directory")?;
    }

    // Load existing TOML (or start empty).
    let existing = if target_path.exists() {
        std::fs::read_to_string(&target_path)
            .wrap_err_with(|| format!("Failed to read {}", target_path.display()))?
    } else {
        String::new()
    };

    let mut root: toml::Value = if existing.trim().is_empty() {
        toml::Value::Table(toml::map::Map::new())
    } else {
        toml::from_str(&existing).wrap_err("Failed to parse existing config")?
    };

    // Parse the value — try as a TOML inline value so numbers/booleans work;
    // fall back to treating it as a bare string.
    let parsed_value: toml::Value = {
        let snippet = format!("_v = {}", value);
        if let Ok(tbl) = toml::from_str::<toml::Value>(&snippet) {
            tbl.as_table()
                .and_then(|t| t.get("_v"))
                .cloned()
                .unwrap_or_else(|| toml::Value::String(value.to_string()))
        } else {
            toml::Value::String(value.to_string())
        }
    };

    // Navigate / create nested tables for dotted keys.
    let parts: Vec<&str> = key.split('.').collect();
    let (table_keys, leaf_key) = parts.split_at(parts.len() - 1);

    let mut current = root
        .as_table_mut()
        .ok_or_else(|| crate::error::err!("Config root is not a table"))?;

    for segment in table_keys {
        let entry = current
            .entry(segment.to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
        current = entry
            .as_table_mut()
            .ok_or_else(|| crate::error::err!("'{}' is not a table in config", segment))?;
    }

    current.insert(leaf_key[0].to_string(), parsed_value);

    let new_content =
        toml::to_string_pretty(&root).wrap_err("Failed to serialise updated config")?;
    std::fs::write(&target_path, &new_content)
        .wrap_err_with(|| format!("Failed to write {}", target_path.display()))?;

    Ok((target_path, is_project))
}
