use oni_core::{
    config::{AgentConfig, ModelConfig, OniConfig, UiConfig, load_config},
    types::ModelTier,
};
use std::fs;
use tempfile::TempDir;

// ── Default values ────────────────────────────────────────────────────────────

#[test]
/// T-CFG-1: OniConfig::default() produces the expected llama-server base URL.
fn t_cfg_1_default_ollama_base_url() {
    let cfg = OniConfig::default();
    assert_eq!(cfg.server.base_url, "http://localhost:8082");
}

#[test]
/// T-CFG-2: OniConfig::default() has the correct default timeout.
fn t_cfg_2_default_ollama_timeout() {
    let cfg = OniConfig::default();
    assert_eq!(cfg.server.timeout_secs, 300);
}

#[test]
/// T-CFG-3: ModelConfig::default() sets the heavy model name to the expected string.
fn t_cfg_3_default_heavy_model() {
    let cfg = ModelConfig::default();
    assert_eq!(cfg.heavy, "qwen3.5:35b");
}

#[test]
/// T-CFG-4: ModelConfig::default() sets the embed model to nomic-embed-text.
fn t_cfg_4_default_embed_model() {
    let cfg = ModelConfig::default();
    assert_eq!(cfg.embed, "nomic-embed-text");
}

#[test]
/// T-CFG-5: UiConfig::default() enables token stats and disables thinking display.
fn t_cfg_5_default_ui_flags() {
    let cfg = UiConfig::default();
    assert!(cfg.show_token_stats);
    assert!(!cfg.show_thinking);
}

#[test]
/// T-CFG-6: UiConfig::default() sets fps to 30.
fn t_cfg_6_default_fps() {
    let cfg = UiConfig::default();
    assert_eq!(cfg.fps, 30);
}

#[test]
/// T-CFG-7: AgentConfig::default() disables write and exec permissions.
fn t_cfg_7_default_agent_permissions() {
    let cfg = AgentConfig::default();
    assert!(!cfg.allow_write);
    assert!(!cfg.allow_exec);
}

#[test]
/// T-CFG-8: AgentConfig::default() sets max_tool_rounds to 10 and context budget to 8192.
fn t_cfg_8_default_agent_limits() {
    let cfg = AgentConfig::default();
    assert_eq!(cfg.max_tool_rounds, 10);
    assert_eq!(cfg.context_budget_tokens, 8192);
}

// ── ModelConfig::model_for_tier ───────────────────────────────────────────────

#[test]
/// T-CFG-9: model_for_tier(Heavy) returns the heavy model name.
fn t_cfg_9_model_for_tier_heavy() {
    let cfg = ModelConfig::default();
    assert_eq!(cfg.model_for_tier(ModelTier::Heavy), "qwen3.5:35b");
}

#[test]
/// T-CFG-10: model_for_tier(Medium) returns the medium model name.
fn t_cfg_10_model_for_tier_medium() {
    let cfg = ModelConfig::default();
    assert_eq!(cfg.model_for_tier(ModelTier::Medium), "qwen3-coder:30b");
}

#[test]
/// T-CFG-11: model_for_tier(General) returns the general model name.
fn t_cfg_11_model_for_tier_general() {
    let cfg = ModelConfig::default();
    assert_eq!(cfg.model_for_tier(ModelTier::General), "glm-4.7-flash:q4_k_m");
}

#[test]
/// T-CFG-12: model_for_tier(Fast) returns the fast model name.
fn t_cfg_12_model_for_tier_fast() {
    let cfg = ModelConfig::default();
    assert_eq!(cfg.model_for_tier(ModelTier::Fast), "qwen3.5:9b");
}

#[test]
/// T-CFG-13: model_for_tier(Embed) returns the embed model name.
fn t_cfg_13_model_for_tier_embed() {
    let cfg = ModelConfig::default();
    assert_eq!(cfg.model_for_tier(ModelTier::Embed), "nomic-embed-text");
}

// ── ModelTier::supports_tools ─────────────────────────────────────────────────

#[test]
/// T-CFG-14: Heavy tier supports tools (fixed — was incorrectly excluded).
fn t_cfg_14_heavy_supports_tools() {
    assert!(ModelTier::Heavy.supports_tools());
}

#[test]
/// T-CFG-15: Medium tier supports tools.
fn t_cfg_15_medium_supports_tools() {
    assert!(ModelTier::Medium.supports_tools());
}

#[test]
/// T-CFG-16: General tier supports tools.
fn t_cfg_16_general_supports_tools() {
    assert!(ModelTier::General.supports_tools());
}

#[test]
/// T-CFG-17: Fast tier supports tools.
fn t_cfg_17_fast_supports_tools() {
    assert!(ModelTier::Fast.supports_tools());
}

#[test]
/// T-CFG-18: Embed tier does NOT support tools.
fn t_cfg_18_embed_no_tools() {
    assert!(!ModelTier::Embed.supports_tools());
}

// ── load_config ───────────────────────────────────────────────────────────────

#[test]
/// T-CFG-19: load_config with None falls back to defaults without error.
fn t_cfg_19_load_config_no_project_dir() {
    // This may pick up a real global config on the test machine; just verify it doesn't error.
    let result = load_config(None);
    assert!(result.is_ok(), "load_config failed: {:?}", result.err());
}

#[test]
/// T-CFG-20: load_config applies project-level overrides over the defaults.
fn t_cfg_20_load_config_project_override() {
    let dir = TempDir::new().unwrap();
    let oni_dir = dir.path().join(".oni");
    fs::create_dir_all(&oni_dir).unwrap();
    fs::write(
        oni_dir.join("oni.toml"),
        r#"
[models]
heavy = "custom-model:latest"
"#,
    )
    .unwrap();

    let cfg = load_config(Some(dir.path())).unwrap();
    assert_eq!(cfg.models.heavy, "custom-model:latest");
}

#[test]
/// T-CFG-21: load_config returns an error for a project config that has invalid TOML.
fn t_cfg_21_load_config_invalid_toml() {
    let dir = TempDir::new().unwrap();
    let oni_dir = dir.path().join(".oni");
    fs::create_dir_all(&oni_dir).unwrap();
    fs::write(oni_dir.join("oni.toml"), "[[[[invalid toml").unwrap();

    let result = load_config(Some(dir.path()));
    assert!(result.is_err(), "expected error for invalid TOML");
}
