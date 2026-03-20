use serde::{Deserialize, Serialize};

/// Fine-grained capability flags for tool access gating.
///
/// Each tool declares which capabilities it requires. The `ToolRegistry`
/// (or execution context) checks these before dispatching a tool call.
///
/// Capability sets per role:
/// - Planner (Heavy tier): no tool access — it plans, doesn't execute.
/// - Executor (Medium tier): all capabilities.
/// - Critic (Fast tier): read-only (`ReadFs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolCapability {
    /// Read files and directories from the local filesystem.
    ReadFs,
    /// Write or edit files on the local filesystem.
    WriteFs,
    /// Execute shell commands (bash).
    ExecShell,
    /// Fetch content from the network (HTTP GET).
    NetworkFetch,
    /// Interact with the user via ask_user prompts.
    UserInteraction,
}

impl std::fmt::Display for ToolCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadFs => write!(f, "READ_FS"),
            Self::WriteFs => write!(f, "WRITE_FS"),
            Self::ExecShell => write!(f, "EXEC_SHELL"),
            Self::NetworkFetch => write!(f, "NETWORK_FETCH"),
            Self::UserInteraction => write!(f, "USER_INTERACTION"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "tool")]
    Tool,
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
        matches!(
            self,
            ModelTier::Heavy | ModelTier::Medium | ModelTier::General | ModelTier::Fast
        )
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ModelTier::Heavy => "HEAVY",
            ModelTier::Medium => "CODE",
            ModelTier::General => "GENERAL",
            ModelTier::Fast => "FAST",
            ModelTier::Embed => "EMBED",
        }
    }

    /// Parse a tier from a lowercase string key (used in config tier_urls).
    pub fn from_key(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "heavy" => Some(Self::Heavy),
            "medium" => Some(Self::Medium),
            "general" => Some(Self::General),
            "fast" => Some(Self::Fast),
            "embed" => Some(Self::Embed),
            _ => None,
        }
    }
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
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

/// Autonomy level controls how much confirmation ONI requires before acting.
/// This IS the permission system — write/exec flags are derived from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutonomyLevel {
    /// Read-only + edits shown but not applied without confirmation.
    /// Every write_file and bash call requires [y/n/diff] confirmation.
    Low,
    /// Reversible changes auto-approved. Destructive ops (rm, overwrite) still prompt.
    /// bash commands that match safety blocklist always prompt.
    Medium,
    /// All local operations auto-approved. Only remote/external actions prompt.
    /// Still respects the bash blocklist (rm -rf /, sudo, etc).
    High,
}

impl Default for AutonomyLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl AutonomyLevel {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
        }
    }

    /// Whether writes should auto-proceed without confirmation.
    pub fn auto_write(&self) -> bool {
        matches!(self, Self::High)
    }

    /// Whether bash commands should auto-proceed without confirmation.
    /// Destructive commands (blocklisted) NEVER auto-proceed regardless of level.
    pub fn auto_exec(&self) -> bool {
        matches!(self, Self::High)
    }

    /// Whether overwriting existing files should prompt.
    pub fn prompt_overwrite(&self) -> bool {
        matches!(self, Self::Low | Self::Medium)
    }

    /// Whether new file creation should prompt.
    pub fn prompt_new_file(&self) -> bool {
        matches!(self, Self::Low)
    }
}

impl std::fmt::Display for AutonomyLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionScope {
    Once,
    Session,
    Permanent,
}

impl PermissionScope {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Once => "YES (ONCE)",
            Self::Session => "YES (SESSION)",
            Self::Permanent => "YES (ALWAYS)",
        }
    }
}
