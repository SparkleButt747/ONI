//! Custom Agent Definitions — load user-defined agents from .oni/agents/ and ~/.local/share/oni/agents/
//!
//! Format: Markdown files with YAML frontmatter.
//! YAML header defines: id, title, description, tier, tools, temperature
//! Markdown body defines: system prompt

use oni_core::types::ModelTier;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AgentDefinition {
    pub id: String,
    pub title: String,
    pub description: String,
    pub tier: ModelTier,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub temperature: Option<f64>,
    pub custom_rules: Option<String>,
    pub source: String, // "built-in", "global", "project"
}

#[derive(Debug, Deserialize)]
struct AgentFrontmatter {
    id: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    tier: Option<String>,
    #[serde(default)]
    tools: Option<Vec<String>>,
    #[serde(default)]
    temperature: Option<f64>,
    #[serde(default)]
    custom_rules: Option<String>,
}

/// Load all agent definitions from both global and project directories.
/// Project-local agents (.oni/agents/) override global ones (~/.local/share/oni/agents/).
pub fn load_agent_definitions(project_dir: Option<&Path>) -> Vec<AgentDefinition> {
    let mut agents = builtin_agents();

    // Global agents
    let global_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("oni")
        .join("agents");
    for agent in load_from_dir(&global_dir, "global") {
        // Override built-in if same ID
        if let Some(existing) = agents.iter_mut().find(|a| a.id == agent.id) {
            *existing = agent;
        } else {
            agents.push(agent);
        }
    }

    // Project-local agents (higher priority)
    if let Some(dir) = project_dir {
        let project_agent_dir = dir.join(".oni").join("agents");
        for agent in load_from_dir(&project_agent_dir, "project") {
            if let Some(existing) = agents.iter_mut().find(|a| a.id == agent.id) {
                *existing = agent;
            } else {
                agents.push(agent);
            }
        }
    }

    agents
}

fn load_from_dir(dir: &Path, source: &str) -> Vec<AgentDefinition> {
    let mut agents = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return agents,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map(|e| e == "md").unwrap_or(false) {
            if let Some(agent) = parse_agent_file(&path, source) {
                agents.push(agent);
            }
        }
    }

    agents
}

fn parse_agent_file(path: &Path, source: &str) -> Option<AgentDefinition> {
    let content = std::fs::read_to_string(path).ok()?;

    // Split YAML frontmatter from markdown body
    if !content.starts_with("---") {
        return None;
    }
    let after_first = &content[3..];
    let end = after_first.find("\n---").map(|pos| pos + 1)?;
    let yaml_str = &after_first[..end];
    let markdown = after_first[end + 3..].trim().to_string();

    let fm: AgentFrontmatter = serde_yaml::from_str(yaml_str).ok()?;

    let tier = match fm.tier.as_deref() {
        Some("heavy" | "h") => ModelTier::Heavy,
        Some("general" | "g") => ModelTier::General,
        Some("fast" | "f") => ModelTier::Fast,
        _ => ModelTier::Medium,
    };

    Some(AgentDefinition {
        id: fm.id.clone(),
        title: fm.title.unwrap_or_else(|| fm.id.clone()),
        description: fm.description.unwrap_or_default(),
        tier,
        system_prompt: markdown,
        tools: fm.tools.unwrap_or_else(|| {
            vec![
                "read_file".into(),
                "write_file".into(),
                "bash".into(),
                "list_directory".into(),
                "search_files".into(),
                "edit_file".into(),
                "get_url".into(),
                "undo".into(),
            ]
        }),
        temperature: fm.temperature,
        custom_rules: fm.custom_rules,
        source: source.to_string(),
    })
}

/// Built-in demon agents.
fn builtin_agents() -> Vec<AgentDefinition> {
    vec![
        AgentDefinition {
            id: "fenrir".into(),
            title: "FENRIR".into(),
            description: "Implementation — all tools, Code tier".into(),
            tier: ModelTier::Medium,
            system_prompt: crate::prompts::FENRIR.to_string(),
            tools: vec![
                "read_file".into(),
                "write_file".into(),
                "bash".into(),
                "list_directory".into(),
                "search_files".into(),
                "edit_file".into(),
                "get_url".into(),
                "undo".into(),
            ],
            temperature: None,
            custom_rules: None,
            source: "built-in".into(),
        },
        AgentDefinition {
            id: "mimir".into(),
            title: "MIMIR".into(),
            description: "Planning — strategic thinking, Heavy tier".into(),
            tier: ModelTier::Heavy,
            system_prompt: crate::prompts::MIMIR.to_string(),
            tools: vec![
                "read_file".into(),
                "list_directory".into(),
                "search_files".into(),
            ],
            temperature: Some(0.3),
            custom_rules: None,
            source: "built-in".into(),
        },
        AgentDefinition {
            id: "hecate".into(),
            title: "HECATE".into(),
            description: "Research — deep investigation, Heavy tier".into(),
            tier: ModelTier::Heavy,
            system_prompt: crate::prompts::HECATE.to_string(),
            tools: vec![
                "read_file".into(),
                "search_files".into(),
                "get_url".into(),
                "list_directory".into(),
            ],
            temperature: Some(0.3),
            custom_rules: None,
            source: "built-in".into(),
        },
        AgentDefinition {
            id: "skuld".into(),
            title: "SKULD".into(),
            description: "Judgement — code review and verification".into(),
            tier: ModelTier::General,
            system_prompt: crate::prompts::SKULD.to_string(),
            tools: vec!["read_file".into(), "search_files".into()],
            temperature: Some(0.1),
            custom_rules: None,
            source: "built-in".into(),
        },
        AgentDefinition {
            id: "loki".into(),
            title: "LOKI".into(),
            description: "Refactoring — transforms code, Code tier".into(),
            tier: ModelTier::Medium,
            system_prompt: crate::prompts::LOKI.to_string(),
            tools: vec![
                "read_file".into(),
                "write_file".into(),
                "edit_file".into(),
                "search_files".into(),
                "list_directory".into(),
                "undo".into(),
            ],
            temperature: Some(0.2),
            custom_rules: None,
            source: "built-in".into(),
        },
    ]
}
