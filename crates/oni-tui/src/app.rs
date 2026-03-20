use crate::ui;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use oni_agent::agent::{Agent, AgentEvent, ConfirmResponse, ToolProposal};
use oni_agent::message_bus::MessageBus;
use oni_agent::tools::ask_user::AskUserRequest;
use oni_agent::preferences::PreferenceEngine;
use oni_agent::trace::ExecutionTrace;
use oni_core::config::{AgentConfig, ModelConfig, ServerConfig, UiConfig};
use oni_core::error::Result;
use oni_core::types::{AutonomyLevel, ModelTier};
use oni_llm::ModelRouter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use throbber_widgets_tui::ThrobberState;
use tokio::sync::mpsc;
use tui_textarea::TextArea;

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Chat,
    MissionControl,
    Preferences,
}

#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub timestamp: String,
    pub name: String,
    pub args_summary: String,
    pub status: String,
    pub latency_ms: u64,
}

/// Details of a completed write_file or bash call, used to render rich inline
/// previews (diff view / command block) in the chat pane.
#[derive(Debug, Clone)]
pub struct ToolDetail {
    /// Tool name — "write_file" | "bash"
    pub name: String,
    /// Raw tool args (JSON)
    pub args: serde_json::Value,
    /// Tool output / result string
    pub result: String,
}

#[derive(Debug, Clone)]
pub struct SubAgentStatus {
    pub mimir: &'static str,   // "IDLE" | "ACTIVE" | "DONE"  — Planner
    pub fenrir: &'static str,  // Executor
    pub skuld: &'static str,   // Critic
}

impl Default for SubAgentStatus {
    fn default() -> Self {
        Self {
            mimir: "IDLE",
            fenrir: "IDLE",
            skuld: "IDLE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LearnedRule {
    pub description: String,
    pub context: String,
    pub observations: u32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub enum DisplayMessage {
    User(String),
    Assistant(String),
    /// Brief status line (EXECUTING / DONE) — shown while the tool runs.
    ToolExec { name: String, status: String },
    /// Rich inline preview shown after a write_file or bash tool completes.
    ToolDetail(ToolDetail),
    Error(String),
    System(String),
    /// Orchestrator: planner produced a numbered step list.
    Plan(Vec<String>),
    /// Orchestrator: executor working on step N of M.
    Step { current: usize, total: usize, description: String },
    /// Orchestrator: critic verdict (accepted / rejected with reason).
    CriticVerdict { accepted: bool, reason: String },
    /// Orchestrator: replanning after rejection.
    Replanning { cycle: usize, reason: String },
}

/// Messages sent from TUI to the agent task
#[derive(Debug)]
pub enum AgentCommand {
    RunTurn(String),
    SetTier(ModelTier),
    SetAutonomy(AutonomyLevel),
    ClearHistory,
    SetAgent(String),
}

pub const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/tier", "SWITCH_MODEL_TIER"),
    ("/model", "SHOW_CURRENT_MODEL"),
    ("/clear", "CLEAR_CHAT_HISTORY"),
    ("/sidebar", "TOGGLE_SIDEBAR"),
    ("/mc", "MISSION_CONTROL_VIEW"),
    ("/chat", "RETURN_TO_CHAT_VIEW"),
    ("/prefs", "LEARNED_PREFERENCES"),
    ("/tools", "LIST_AVAILABLE_TOOLS"),
    ("/autonomy", "SET_AUTONOMY_LEVEL"),
    ("/diff", "REVIEW_PENDING_DIFFS"),
    ("/review", "REVIEW_STAGED_CHANGES"),
    ("/spec", "GENERATE_SPECIFICATION"),
    ("/research", "DEEP_RESEARCH_MODE"),
    ("/plan", "SHOW_ACTIVE_PLAN"),
    ("/doctor", "SYSTEM_HEALTH_CHECK"),
    ("/mimir", "PLANNING_MODE"),
    ("/fenrir", "IMPLEMENTATION_MODE"),
    ("/hecate", "RESEARCH_MODE"),
    ("/loki", "REFACTOR_MODE"),
    ("/agent", "LIST_AGENTS"),
    ("/trace", "SHOW_EXECUTION_TRACE"),
    ("/undo", "UNDO_LAST_CHANGE"),
    ("/help", "SHOW_ALL_COMMANDS"),
    ("/quit", "EXIT_ONI"),
];

/// Generate a short session ID like "CONV_8FK2A9" using system entropy (no uuid dep).
fn generate_session_id() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    let h = hasher.finish();
    // Take 6 alphanumeric chars from the hash
    let chars: Vec<char> = "ABCDEFGHJKLMNPQRSTUVWXYZ0123456789".chars().collect();
    let n = chars.len() as u64;
    let c0 = chars[((h >> 0) & 0x3f) as usize % chars.len()];
    let c1 = chars[((h >> 6) & 0x3f) as usize % chars.len()];
    let c2 = chars[((h >> 12) & 0x3f) as usize % chars.len()];
    let c3 = chars[((h >> 18) & 0x3f) as usize % chars.len()];
    let c4 = chars[((h >> 24) & 0x3f) as usize % chars.len()];
    let c5 = chars[((h >> 30) & 0x3f) as usize % n as usize];
    format!("CONV_{}{}{}{}{}{}", c0, c1, c2, c3, c4, c5)
}

/// Returns current wall-clock time as HH:MM:SS using std only (no chrono dep).
fn timestamp_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// Onboarding conversation state.
#[derive(Debug, Clone, PartialEq)]
pub enum OnboardingStep {
    Intro,       // Show introduction, wait for any key
    AskName,     // "What should I call you?"
    AskRole,     // "What do you do?"
    AskStyle,    // "How do you prefer to work?"
    Complete,    // Done — transition to normal chat
}

pub struct App {
    pub session_id: String,
    pub messages: Vec<DisplayMessage>,
    /// Onboarding state — None means onboarding complete or not needed.
    pub onboarding: Option<OnboardingStep>,
    pub onboarding_name: String,
    pub onboarding_role: String,
    pub onboarding_style: String,
    pub input: TextArea<'static>,
    pub is_thinking: bool,
    pub current_tier: ModelTier,
    pub current_model_name: String,
    pub total_tokens: u64,
    pub last_tokens_per_sec: f64,
    pub turn_count: u32,
    pub should_quit: bool,
    pub throbber_state: ThrobberState,
    pub model_config: ModelConfig,
    pub slash_menu_visible: bool,
    pub slash_menu_selected: usize,
    pub slash_menu_filter: String,
    pub show_sidebar: bool,
    pub view_mode: ViewMode,
    pub learned_rules: Vec<LearnedRule>,
    pub tool_history: Vec<ToolCallRecord>,
    pub scroll_offset: u16,
    pub scroll_locked_to_bottom: bool,
    /// Names of all registered tools (populated at runtime based on permissions).
    pub tool_names: Vec<String>,
    // Boot sequence animation
    pub boot_frame: u16,
    pub boot_complete: bool,
    pub boot_file_count: usize,
    /// Set when a critical (unrecoverable) error occurs.
    /// When `Some`, the full-screen error state is rendered instead of the normal UI.
    pub critical_error: Option<String>,
    /// Glitch pulse frame counter for error transitions. `Some(0..=2)` = active.
    pub glitch_frame: Option<u8>,
    /// Previous critical_error state — used to detect transitions to trigger glitch.
    pub prev_had_error: bool,
    /// Scan reveal progress (0.0 to 1.0) for the latest assistant message.
    pub reveal_progress: f32,
    /// Number of messages when the last reveal started — used to detect new responses.
    pub reveal_msg_count: usize,
    /// Submitted messages available for Up/Down history navigation.
    pub command_history: Vec<String>,
    /// `None` = not browsing history; `Some(i)` = currently at index i.
    pub history_index: Option<usize>,
    /// Session start time (for burn rate calculation).
    pub session_start: std::time::Instant,
    /// Tokens per minute (burn rate).
    pub burn_rate: f64,
    /// Current autonomy level.
    pub autonomy: AutonomyLevel,
    /// Pending tool proposal awaiting user confirmation.
    pub pending_proposal: Option<PendingProposal>,
    /// Pending ask-user request awaiting free-text input from the user.
    pub pending_ask: Option<PendingAsk>,
    /// Sub-agent status for Mission Control panel.
    pub sub_agent_status: SubAgentStatus,
    /// Currently active named agent mode.
    pub active_agent: &'static str,
    /// Project directory this session is rooted in (for plan_store isolation).
    pub project_dir: String,
    /// Shared execution trace — readable by the TUI for /trace command.
    pub trace: Arc<Mutex<ExecutionTrace>>,
    /// Shared event bus — same instance as the Agent's bus.
    pub event_bus: MessageBus<AgentEvent>,
    /// Base server URL for the default tier (displayed in splash/status).
    pub server_url: String,
    /// Server config for auto-starting tier servers.
    pub server_config: Option<ServerConfig>,
}

/// A tool proposal currently shown to the user awaiting y/n/d response.
pub struct PendingProposal {
    pub name: String,
    pub summary: String,
    pub args: serde_json::Value,
    pub respond: tokio::sync::oneshot::Sender<ConfirmResponse>,
}

/// A pending ask-user request — agent is blocked waiting for free-text input.
pub struct PendingAsk {
    pub question: String,
    pub respond: std::sync::mpsc::SyncSender<String>,
}

impl App {
    pub fn new(model_name: &str, tier: ModelTier, model_config: ModelConfig) -> Self {
        let mut input = TextArea::default();
        input.set_cursor_line_style(ratatui::style::Style::default());
        input.set_style(oni_core::palette::data_style());

        // Check if onboarding is needed
        let onboarding = if oni_core::personality::needs_onboarding() {
            Some(OnboardingStep::Intro)
        } else {
            None
        };

        Self {
            session_id: generate_session_id(),
            messages: Vec::new(),
            onboarding,
            onboarding_name: String::new(),
            onboarding_role: String::new(),
            onboarding_style: String::new(),
            input,
            is_thinking: false,
            current_tier: tier,
            current_model_name: model_name.to_string(),
            total_tokens: 0,
            last_tokens_per_sec: 0.0,
            turn_count: 0,
            should_quit: false,
            throbber_state: ThrobberState::default(),
            model_config,
            slash_menu_visible: false,
            slash_menu_selected: 0,
            slash_menu_filter: String::new(),
            show_sidebar: true,
            view_mode: ViewMode::Chat,
            learned_rules: Vec::new(),
            tool_history: Vec::new(),
            scroll_offset: 0,
            scroll_locked_to_bottom: true,
            boot_frame: 0,
            boot_complete: false,
            boot_file_count: 0,
            critical_error: None,
            glitch_frame: None,
            prev_had_error: false,
            reveal_progress: 1.0, // start fully revealed (no animation)
            reveal_msg_count: 0,
            command_history: Self::load_history(),
            history_index: None,
            session_start: std::time::Instant::now(),
            burn_rate: 0.0,
            autonomy: AutonomyLevel::Medium,
            pending_proposal: None,
            pending_ask: None,
            sub_agent_status: SubAgentStatus::default(),
            active_agent: "fenrir",
            project_dir: std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".to_string()),
            trace: Arc::new(Mutex::new(ExecutionTrace::new(500))),
            event_bus: MessageBus::new(500),
            // Default to read-only tools; updated in run() once permissions are known.
            tool_names: vec![
                "read_file".into(),
                "list_dir".into(),
                "search_files".into(),
                "get_url".into(),
                "undo".into(),
            ],
            server_url: String::new(),
            server_config: None,
        }
    }

    /// Set the server URL for display in splash/status.
    pub fn set_server_url(&mut self, url: &str) {
        self.server_url = url.to_string();
    }

    /// Set the server config for auto-starting tier servers.
    pub fn set_server_config(&mut self, config: ServerConfig) {
        self.server_config = Some(config);
    }

    fn load_history() -> Vec<String> {
        let path = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("oni")
            .join("history.txt");
        std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    }

    pub fn save_history(&self) {
        let path = dirs::data_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("oni")
            .join("history.txt");
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // Keep last 500 entries
        let start = self.command_history.len().saturating_sub(500);
        let entries = &self.command_history[start..];
        let _ = std::fs::write(&path, entries.join("\n"));
    }

    /// Handle a slash command. Returns `true` if handled.
    pub fn handle_slash_command(
        &mut self,
        input: &str,
        cmd_tx: &mpsc::UnboundedSender<AgentCommand>,
    ) -> bool {
        // Inline shell: `:cmd` prefix — execute directly via bash, don't send to LLM.
        if input.starts_with(':') {
            let cmd = input[1..].trim();
            if cmd.is_empty() {
                return true;
            }
            // Special shell-like exit commands — bail out before spawning bash
            match cmd {
                "q" | "quit" | "exit" => {
                    self.should_quit = true;
                    return true;
                }
                _ => {}
            }
            // Blocklist check — same patterns as the agent bash tool
            let normalised: String = cmd
                .to_lowercase()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            const BLOCKED_PATTERNS: &[&str] = &[
                "rm -rf /",
                "rm -rf /*",
                "rm -rf ~",
                "mkfs",
                "dd if=",
                ":(){:|:&};:",
                "chmod -r 777 /",
                "sudo rm",
                "sudo dd",
                "sudo mkfs",
                "> /dev/sda",
                "curl | sh",
                "curl | bash",
                "wget | sh",
                "wget | bash",
            ];
            if BLOCKED_PATTERNS.iter().any(|p| normalised.contains(*p)) {
                self.messages.push(DisplayMessage::Error(
                    "BLOCKED: Command matches security blocklist.".into(),
                ));
                return true;
            }
            let output = std::process::Command::new("bash").arg("-c").arg(cmd).output();
            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    self.messages.push(DisplayMessage::System(format!(
                        "$ {}\n{}{}",
                        cmd, stdout, stderr
                    )));
                }
                Err(e) => {
                    self.messages
                        .push(DisplayMessage::Error(format!("Shell error: {}", e)));
                }
            }
            return true;
        }

        if !input.starts_with('/') {
            return false;
        }

        let mut parts = input.splitn(2, ' ');
        let cmd = parts.next().unwrap_or("").to_lowercase();
        let arg = parts.next().map(str::trim).unwrap_or("");

        match cmd.as_str() {
            "/tier" | "/t" => {
                let new_tier = match arg.to_lowercase().as_str() {
                    "heavy" | "h" => Some(ModelTier::Heavy),
                    "medium" | "code" | "m" | "c" => Some(ModelTier::Medium),
                    "general" | "gen" | "g" => Some(ModelTier::General),
                    "fast" | "f" | "quick" => Some(ModelTier::Fast),
                    _ => None,
                };
                match new_tier {
                    Some(tier) => {
                        self.current_tier = tier;
                        self.current_model_name =
                            self.model_config.model_for_tier(tier).to_string();
                        let _ = cmd_tx.send(AgentCommand::SetTier(tier));
                        self.messages.push(DisplayMessage::System(format!(
                            "TIER_SWITCH -> {} [{}]",
                            tier.display_name(),
                            self.current_model_name.to_uppercase()
                        )));
                    }
                    None => {
                        self.messages.push(DisplayMessage::Error(
                            "VALID_TIERS: heavy, medium, general, fast".into(),
                        ));
                    }
                }
            }
            "/clear" => {
                self.messages.clear();
                let _ = cmd_tx.send(AgentCommand::ClearHistory);
                self.total_tokens = 0;
                self.turn_count = 0;
                self.last_tokens_per_sec = 0.0;
            }
            "/model" | "/m" => {
                self.messages.push(DisplayMessage::System(format!(
                    "MODEL: {} | TIER: {} | TOKENS: {} | TURNS: {}",
                    self.current_model_name.to_uppercase(),
                    self.current_tier.display_name(),
                    self.total_tokens,
                    self.turn_count
                )));
            }
            "/sidebar" | "/sb" => {
                self.show_sidebar = !self.show_sidebar;
            }
            "/mc" => {
                self.view_mode = ViewMode::MissionControl;
            }
            "/prefs" => {
                self.view_mode = ViewMode::Preferences;
            }
            "/chat" | "/c" => {
                self.view_mode = ViewMode::Chat;
            }
            "/help" | "/h" | "/?" => {
                self.messages.push(DisplayMessage::System(
                    "/tier <heavy|medium|general|fast>  SWITCH_MODEL_TIER\n\
                     /model                             SHOW_CURRENT_MODEL\n\
                     /clear                             CLEAR_HISTORY\n\
                     /mc                                MISSION_CONTROL\n\
                     /prefs                             LEARNED_PREFERENCES\n\
                     /chat                              RETURN_TO_CHAT\n\
                     /sidebar                           TOGGLE_SIDEBAR\n\
                     /review                            REVIEW_STAGED_CHANGES\n\
                     /spec <description>                GENERATE_SPECIFICATION\n\
                     /research <topic>                  DEEP_RESEARCH_MODE\n\
                     /plan                              SHOW_ACTIVE_PLAN\n\
                     /doctor                            SYSTEM_HEALTH_CHECK\n\
                     /undo                              UNDO_LAST_CHANGE\n\
                     \n\
                     AGENTS:\n\
                     /mimir                             PLANNING MODE (heavy, no tools)\n\
                     /fenrir                            IMPLEMENT MODE (code, all tools)\n\
                     /hecate                            RESEARCH MODE (heavy, read/fetch)\n\
                     /loki                              REFACTOR MODE (code, edit/write)\n\
                     /agent                             LIST AGENTS\n\
                     \n\
                     /help                              SHOW_COMMANDS\n\
                     /quit                              EXIT_ONI"
                        .to_string(),
                ));
            }
            "/tools" => {
                let tools = [
                    "read_file     Read file contents (100KB max)",
                    "write_file    Write/create files (path safety enforced)",
                    "bash          Execute shell commands (blocklist enforced)",
                    "list_directory List directory contents",
                    "search_files  Regex search across files (ripgrep-style)",
                    "edit_file     Patch-based find-and-replace",
                    "get_url       HTTP fetch with HTML stripping",
                ];
                self.messages.push(DisplayMessage::System(
                    tools.join("\n"),
                ));
            }
            "/autonomy" | "/auto" => {
                let new_level = match arg.to_lowercase().as_str() {
                    "low" | "l" => Some(AutonomyLevel::Low),
                    "medium" | "med" | "m" => Some(AutonomyLevel::Medium),
                    "high" | "h" => Some(AutonomyLevel::High),
                    _ => None,
                };
                match new_level {
                    Some(level) => {
                        self.autonomy = level;
                        let _ = cmd_tx.send(AgentCommand::SetAutonomy(level));
                        self.messages.push(DisplayMessage::System(format!(
                            "AUTONOMY -> {}\n{}",
                            level.display_name(),
                            match level {
                                AutonomyLevel::Low => "All writes and exec require confirmation.",
                                AutonomyLevel::Medium => "Destructive operations require confirmation.",
                                AutonomyLevel::High => "All local operations auto-approved.",
                            }
                        )));
                    }
                    None => {
                        self.messages.push(DisplayMessage::System(format!(
                            "CURRENT: {} | VALID: low, medium, high",
                            self.autonomy.display_name()
                        )));
                    }
                }
            }
            "/diff" => {
                self.messages.push(DisplayMessage::System(
                    "No pending diffs. Diffs are shown inline after write_file operations.".into(),
                ));
            }
            "/review" | "/rev" => {
                match oni_agent::review::get_staged_diff() {
                    Some(diff) => {
                        let line_count = diff.lines().count();
                        self.messages.push(DisplayMessage::System(format!(
                            "Reviewing {} lines of diff... (results will appear shortly)",
                            line_count
                        )));
                        let review_prompt = format!(
                            "Review this git diff for bugs, security issues, and style problems. Be concise.\n\n```diff\n{}\n```",
                            if diff.len() > 4000 { &diff[..4000] } else { &diff }
                        );
                        let _ = cmd_tx.send(AgentCommand::RunTurn(review_prompt));
                        self.is_thinking = true;
                    }
                    None => {
                        self.messages.push(DisplayMessage::System(
                            "No changes to review. Stage changes with `git add` first.".into(),
                        ));
                    }
                }
            }
            "/spec" => {
                if arg.is_empty() {
                    self.messages.push(DisplayMessage::Error(
                        "Usage: /spec <description of what to build>".into(),
                    ));
                } else {
                    let spec_prompt = format!(
                        "Generate a structured specification document for the following:\n\n{}\n\n\
                        Format as markdown with these sections:\n\
                        ## Goal\nOne sentence.\n\n\
                        ## Requirements\nNumbered list of functional requirements.\n\n\
                        ## Architecture\nHow it should be structured.\n\n\
                        ## File Structure\nExact files to create/modify.\n\n\
                        ## Acceptance Criteria\nChecklist of what \"done\" looks like.\n\n\
                        Be specific. Reference exact file paths where possible.",
                        arg
                    );
                    self.messages.push(DisplayMessage::User(format!("/spec {}", arg)));
                    let _ = cmd_tx.send(AgentCommand::RunTurn(spec_prompt));
                    self.is_thinking = true;
                }
            }
            "/research" => {
                if arg.is_empty() {
                    self.messages.push(DisplayMessage::Error(
                        "Usage: /research <topic to investigate>".into(),
                    ));
                } else {
                    let research_prompt = format!(
                        "Research the following topic thoroughly:\n\n{}\n\n\
                        Use these tools to gather information:\n\
                        1. search_files — check the current codebase for related code\n\
                        2. read_file — read relevant files you find\n\
                        3. get_url — fetch external documentation if needed\n\n\
                        After gathering information, synthesise your findings into a clear report with:\n\
                        - What you found\n\
                        - Key code locations (file:line)\n\
                        - Recommendations\n\n\
                        Be thorough but concise.",
                        arg
                    );
                    self.messages.push(DisplayMessage::User(format!("/research {}", arg)));
                    let _ = cmd_tx.send(AgentCommand::RunTurn(research_prompt));
                    self.is_thinking = true;
                }
            }
            "/plan" => {
                if arg == "clear" {
                    oni_agent::plan_store::PersistedPlan::clear(&self.project_dir);
                    self.messages.push(DisplayMessage::System("Active plan cleared.".into()));
                } else {
                    match oni_agent::plan_store::PersistedPlan::load(&self.project_dir) {
                        Some(plan) => {
                            let mut lines = vec![format!("ACTIVE PLAN: {}\n", plan.task)];
                            for step in &plan.steps {
                                let icon = match step.status {
                                    oni_agent::plan_store::StepStatus::Done => "[x]",
                                    oni_agent::plan_store::StepStatus::InProgress => "[>]",
                                    oni_agent::plan_store::StepStatus::Failed => "[!]",
                                    _ => "[ ]",
                                };
                                lines.push(format!(
                                    "  {} {}. {}",
                                    icon, step.index, step.description
                                ));
                            }
                            lines.push(format!("\n{}", plan.summary()));
                            self.messages.push(DisplayMessage::System(lines.join("\n")));
                        }
                        None => {
                            self.messages.push(DisplayMessage::System(
                                "No active plan. Start a complex task and the orchestrator will create one.".into(),
                            ));
                        }
                    }
                }
            }
            "/doctor" | "/doc" => {
                self.messages.push(DisplayMessage::System(format!(
                    "SYSTEM_ONI V0.1.0\n\
                     TIER: {} | MODEL: {}\n\
                     TOKENS: {} | TURNS: {} | {:.1} TOK/S\n\
                     TOOLS: {}",
                    self.current_tier.display_name(),
                    self.current_model_name.to_uppercase(),
                    self.total_tokens,
                    self.turn_count,
                    self.last_tokens_per_sec,
                    self.tool_names.join(", ")
                )));
            }
            "/mimir" => {
                self.active_agent = "mimir";
                self.current_tier = ModelTier::Heavy;
                let _ = cmd_tx.send(AgentCommand::SetTier(ModelTier::Heavy));
                let _ = cmd_tx.send(AgentCommand::SetAgent("mimir".to_string()));
                self.messages.push(DisplayMessage::System(
                    "[\u{03A3}] MIMIR ACTIVE — Planning mode. Strategic thinking, no tools. Heavy tier.".into(),
                ));
            }
            "/fenrir" => {
                self.active_agent = "fenrir";
                self.current_tier = ModelTier::Medium;
                let _ = cmd_tx.send(AgentCommand::SetTier(ModelTier::Medium));
                let _ = cmd_tx.send(AgentCommand::SetAgent("fenrir".to_string()));
                self.messages.push(DisplayMessage::System(
                    "[\u{03A8}] FENRIR ACTIVE — Implementation mode. All tools. Code tier.".into(),
                ));
            }
            "/hecate" => {
                self.active_agent = "hecate";
                self.current_tier = ModelTier::Heavy;
                let _ = cmd_tx.send(AgentCommand::SetTier(ModelTier::Heavy));
                let _ = cmd_tx.send(AgentCommand::SetAgent("hecate".to_string()));
                self.messages.push(DisplayMessage::System(
                    "[\u{25CA}] HECATE ACTIVE — Research mode. Read/search/fetch tools. Heavy tier.".into(),
                ));
            }
            "/loki" => {
                self.active_agent = "loki";
                self.current_tier = ModelTier::Medium;
                let _ = cmd_tx.send(AgentCommand::SetTier(ModelTier::Medium));
                let _ = cmd_tx.send(AgentCommand::SetAgent("loki".to_string()));
                self.messages.push(DisplayMessage::System(
                    "[\u{2206}] LOKI ACTIVE — Refactor mode. Edit/write/search tools. Code tier.".into(),
                ));
            }
            "/agent" => {
                self.messages.push(DisplayMessage::System(
                    "AVAILABLE AGENTS:\n\
                     [\u{03A3}] /mimir   — PLANNING    Heavy tier, strategic thinking\n\
                     [\u{03A8}] /fenrir  — IMPLEMENT   Code tier, all tools (DEFAULT)\n\
                     [\u{25CA}] /hecate  — RESEARCH    Heavy tier, investigation\n\
                     [\u{2206}] /loki    — REFACTOR    Code tier, transforms code\n\n\
                     Current: ".to_string() + self.active_agent.to_uppercase().as_str(),
                ));
            }
            "/trace" => {
                let summary = self
                    .trace
                    .lock()
                    .map(|t| {
                        let s = t.summary();
                        if s.is_empty() {
                            "No trace events recorded yet.".to_string()
                        } else {
                            s
                        }
                    })
                    .unwrap_or_else(|_| "Trace lock poisoned.".to_string());
                self.messages.push(DisplayMessage::System(
                    format!("=== EXECUTION TRACE ===\n{}", summary),
                ));
            }
            "/undo" => {
                self.messages.push(DisplayMessage::System(
                    "Undoing last change...".into(),
                ));
                let _ = cmd_tx.send(AgentCommand::RunTurn(
                    "Use the undo tool to revert the last file change.".into(),
                ));
                self.is_thinking = true;
            }
            "/quit" | "/exit" | "/q" => {
                self.should_quit = true;
            }
            _ => {
                self.messages.push(DisplayMessage::Error(format!(
                    "UNKNOWN_COMMAND: {} — TYPE /help",
                    cmd.to_uppercase()
                )));
            }
        }

        true
    }

    /// Load learned rules from the DB into `self.learned_rules`.
    pub fn refresh_learned_rules(&mut self, db_path: &PathBuf) {
        let engine = PreferenceEngine::new(db_path.clone());
        // Run crystallisation + confidence update first
        engine.update_rules();
        engine.crystallise_rules();
        let rules = engine.get_all_rules();
        self.learned_rules = rules
            .into_iter()
            .map(|r| LearnedRule {
                description: r.description,
                context: r.context,
                observations: r.observations as u32,
                confidence: r.confidence as f32,
            })
            .collect();
    }

    fn handle_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::Thinking => {
                self.is_thinking = true;
                self.sub_agent_status.mimir = "ACTIVE";
            }
            AgentEvent::PlanGenerated { steps } => {
                self.is_thinking = false;
                self.sub_agent_status.mimir = "DONE";
                self.sub_agent_status.fenrir = "ACTIVE";
                self.messages.push(DisplayMessage::Plan(steps));
            }
            AgentEvent::ExecutorStep { step, total, description } => {
                self.is_thinking = true;
                self.sub_agent_status.fenrir = "ACTIVE";
                self.messages.push(DisplayMessage::Step {
                    current: step,
                    total,
                    description,
                });
            }
            AgentEvent::CriticVerdict { accepted, reason } => {
                self.is_thinking = false;
                self.sub_agent_status.skuld = "DONE";
                self.sub_agent_status.fenrir = "IDLE";
                self.messages.push(DisplayMessage::CriticVerdict { accepted, reason });
            }
            AgentEvent::Replanning { cycle, reason } => {
                self.messages.push(DisplayMessage::Replanning { cycle, reason });
            }
            AgentEvent::Response(text) => {
                self.is_thinking = false;
                self.messages.push(DisplayMessage::Assistant(text));
            }
            AgentEvent::ToolExec { name, status, args, result } => {
                // Record in tool history for Mission Control
                let args_summary = summarise_args(&name, &args);
                let record = ToolCallRecord {
                    timestamp: timestamp_now(),
                    name: name.to_uppercase(),
                    args_summary: args_summary.clone(),
                    status: status.to_uppercase(),
                    latency_ms: 0,
                };
                self.tool_history.push(record);

                if status == "DONE" {
                    // For write_file and bash: emit a rich ToolDetail message instead of
                    // a plain status line. The plain EXECUTING line was already pushed.
                    if (name == "write_file" || name == "bash") && result.is_some() {
                        self.messages.push(DisplayMessage::ToolDetail(ToolDetail {
                            name: name.clone(),
                            args,
                            result: result.unwrap_or_default(),
                        }));
                        return;
                    }
                }

                self.messages.push(DisplayMessage::ToolExec { name, status });
            }
            AgentEvent::Error(text) => {
                self.is_thinking = false;

                // Detect critical LLM server connectivity failures
                let lower = text.to_lowercase();
                let is_critical = lower.contains("connection refused")
                    || lower.contains("failed to connect")
                    || (lower.contains("llama-server") && lower.contains("error"))
                    || lower.contains("os error 111")
                    || lower.contains("no such host");

                if is_critical {
                    self.critical_error = Some(text);
                } else {
                    self.messages.push(DisplayMessage::Error(text));
                }
            }
            AgentEvent::Done {
                tokens,
                duration_ms,
            } => {
                self.is_thinking = false;
                self.sub_agent_status.mimir = "IDLE";
                self.sub_agent_status.fenrir = "IDLE";
                self.sub_agent_status.skuld = "IDLE";
                self.total_tokens = tokens;
                self.turn_count += 1;
                if duration_ms > 0 {
                    self.last_tokens_per_sec =
                        tokens as f64 / (duration_ms as f64 / 1000.0);
                }
                // Calculate burn rate (tok/min) from session start
                let elapsed_min = self.session_start.elapsed().as_secs_f64() / 60.0;
                if elapsed_min > 0.1 {
                    self.burn_rate = self.total_tokens as f64 / elapsed_min;
                }
            }
            AgentEvent::BudgetExhausted { limit_type, used, limit } => {
                self.is_thinking = false;
                self.messages.push(DisplayMessage::Error(format!(
                    "BUDGET EXHAUSTED: {} limit reached ({}/{} tokens). Session paused.",
                    limit_type, used, limit
                )));
            }
        }
    }
}

/// Build a short human-readable summary of a tool's args (for Mission Control).
fn summarise_args(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "write_file" => args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string(),
        "bash" => {
            let cmd = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            if cmd.len() > 40 {
                format!("{}...", &cmd[..40])
            } else {
                cmd.to_string()
            }
        }
        "read_file" => args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string(),
        "list_directory" => args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("?")
            .to_string(),
        _ => String::new(),
    }
}

pub async fn run(
    router: Arc<ModelRouter>,
    db: oni_db::Database,
    db_path: PathBuf,
    agent_config: AgentConfig,
    ui_config: UiConfig,
    model_config: ModelConfig,
    server_config: ServerConfig,
) -> Result<()> {
    // Enable mouse capture for scroll support
    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::EnableMouseCapture
    )?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::cursor::SetCursorStyle::SteadyBlock
    )?;
    let mut terminal = ratatui::init();
    terminal.clear()?;

    let tier = model_config.default_tier;
    let model_name = router.model_name(tier).to_string();

    let mut app = App::new(&model_name, tier, model_config);
    app.set_server_url(router.client().base_url());
    app.set_server_config(server_config.clone());

    // Populate tool list based on granted permissions
    {
        let mut names = vec![
            "read_file".to_string(),
            "list_dir".to_string(),
            "search_files".to_string(),
            "get_url".to_string(),
            "ask_user".to_string(),
            "undo".to_string(),
        ];
        if agent_config.allow_write {
            names.push("write_file".to_string());
            names.push("edit_file".to_string());
        }
        if agent_config.allow_exec {
            names.push("bash".to_string());
            names.push("forge_tool".to_string());
        }
        names.sort();
        app.tool_names = names;
    }

    // If onboarding is needed, show intro and skip boot animation
    if app.onboarding.is_some() {
        app.boot_complete = true; // Skip boot animation during onboarding
        app.show_sidebar = false; // Cleaner onboarding experience
        app.messages.push(DisplayMessage::System(
            "SYSTEM_ONI INITIALISING...\n\n\
            I'm ONI — Onboard Native Intelligence.\n\
            I run locally on your machine. No cloud. No telemetry. No data leaves this box.\n\n\
            Before we start, I need to know a few things about you.\n\n\
            Press ENTER to begin."
                .into(),
        ));
    } else {
        // Record session start for relationship tracking
        let mut rel = oni_core::personality::RelationshipState::load();
        rel.on_session();
        rel.save();
        // Record interaction for emotional state
        let mut emotions = oni_core::personality::EmotionalState::load();
        emotions.on_interaction();
        emotions.save();

        // Check for an incomplete plan from a previous session
        if let Some(plan) = oni_agent::plan_store::PersistedPlan::load(&app.project_dir) {
            if !plan.is_complete() {
                app.messages.push(DisplayMessage::System(format!(
                    "RESUMING: {}\nType /plan to see progress, or continue working.",
                    plan.summary()
                )));
            }
        }
    }

    let project_dir = std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    // Start file watcher for incremental re-indexing (background thread)
    let _file_watcher = project_dir.as_ref().and_then(|dir| {
        let dir_path = std::path::Path::new(dir);
        let index_db = dir_path.join(".oni").join("index.db");
        if !index_db.exists() {
            return None; // No index yet — skip watcher
        }
        match oni_context::watcher::FileWatcher::start(dir_path) {
            Ok(watcher) => {
                tracing::info!("File watcher started for {}", dir);
                Some(watcher)
            }
            Err(e) => {
                tracing::warn!("File watcher failed to start: {}", e);
                None
            }
        }
    });

    // Shared execution trace — created here so both app and agent share the same instance.
    let shared_trace: Arc<Mutex<ExecutionTrace>> = Arc::new(Mutex::new(ExecutionTrace::new(500)));
    app.trace = shared_trace.clone();

    // Event bus: shared between agent and TUI (replaces the old mpsc channel)
    let shared_event_bus: MessageBus<AgentEvent> = MessageBus::new(500);
    app.event_bus = shared_event_bus.clone();

    // Command channel: TUI -> agent (kept as mpsc — needs request/response semantics)
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<AgentCommand>();

    // Spawn a persistent agent task that maintains conversation history
    let agent_router = router.clone();
    let agent_project_dir = project_dir.clone();
    let allow_write = agent_config.allow_write;
    let allow_exec = agent_config.allow_exec;
    let max_rounds = agent_config.max_tool_rounds;

    // Create conversation ID for persistence
    let conv_id = db
        .create_conversation(project_dir.as_deref().unwrap_or("."))
        .ok();

    let agent_db_path = db_path.clone();
    let agent_session_id = app.session_id.clone();
    let agent_autonomy = agent_config.autonomy;
    let session_budget = agent_config.session_budget;
    let monthly_limit = agent_config.monthly_limit;
    let agent_compaction = agent_config.compaction.clone();

    // Proposal channel: agent sends proposals, TUI receives and shows confirmation UI
    let (proposal_tx, mut proposal_rx) = mpsc::unbounded_channel::<ToolProposal>();
    let (ask_user_tx, mut ask_user_rx) = mpsc::unbounded_channel::<AskUserRequest>();

    // Pass shared trace and event bus into agent task
    let agent_trace = shared_trace;
    let agent_event_bus = shared_event_bus;
    let spawn_server_config = server_config;

    tokio::spawn(async move {
        let mut agent = Agent::new_with_prefs(
            agent_router,
            allow_write,
            allow_exec,
            max_rounds,
            tier,
            agent_project_dir.as_deref(),
            Some(agent_db_path.clone()),
            Some(agent_session_id.clone()),
            agent_compaction.clone(),
            oni_agent::telemetry::FeatureFlags::default(),
            true,
        );
        // Wire the shared trace so the TUI can read live events
        agent.trace = agent_trace;
        // Wire the shared event bus so the TUI can drain events
        agent.set_event_bus(agent_event_bus);
        agent.set_autonomy(agent_autonomy);
        agent.set_proposal_channel(proposal_tx);
        agent.set_ask_user_channel(ask_user_tx);
        agent.set_budget(session_budget, monthly_limit);

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                AgentCommand::RunTurn(message) => {
                    match agent.run_turn(&message).await {
                        Ok(_) => {}
                        Err(e) => {
                            agent.event_bus.publish(AgentEvent::Error(e.to_string()));
                        }
                    }
                }
                AgentCommand::SetTier(new_tier) => {
                    agent.set_tier(new_tier);
                    // Auto-start the server for the new tier
                    ensure_tier_server(new_tier, &spawn_server_config).await;
                }
                AgentCommand::SetAutonomy(level) => {
                    agent.set_autonomy(level);
                }
                AgentCommand::ClearHistory => {
                    // Recreate agent to clear conversation (preserve preference engine)
                    let prev_autonomy = agent.autonomy;
                    let prev_bus = agent.event_bus.clone();
                    agent = Agent::new_with_prefs(
                        agent.router_clone(),
                        allow_write,
                        allow_exec,
                        max_rounds,
                        agent.current_tier(),
                        agent_project_dir.as_deref(),
                        Some(agent_db_path.clone()),
                        Some(agent_session_id.clone()),
                        agent_compaction.clone(),
                        agent.telemetry.flags(),
                        true,
                    );
                    agent.set_autonomy(prev_autonomy);
                    // Preserve the shared event bus across clear
                    agent.set_event_bus(prev_bus);
                }
                AgentCommand::SetAgent(_) => {
                    // Reserved for future use
                }
            }
        }
    });

    let tick_rate = Duration::from_millis(1000 / ui_config.fps as u64);

    loop {
        terminal.draw(|frame| ui::draw(&mut app, frame))?;

        if event::poll(tick_rate)? {
            let ev = event::read()?;
            // Mouse scroll handling
            if let Event::Mouse(mouse) = ev {
                match mouse.kind {
                    crossterm::event::MouseEventKind::ScrollUp => {
                        app.scroll_offset = app.scroll_offset.saturating_add(3);
                        app.scroll_locked_to_bottom = false;
                    }
                    crossterm::event::MouseEventKind::ScrollDown => {
                        app.scroll_offset = app.scroll_offset.saturating_sub(3);
                        if app.scroll_offset == 0 {
                            app.scroll_locked_to_bottom = true;
                        }
                    }
                    _ => {}
                }
            }
            if let Event::Key(key) = ev {
                // Handle onboarding flow
                if let Some(ref step) = app.onboarding {
                    match step {
                        OnboardingStep::Intro => {
                            if key.code == KeyCode::Enter || key.code == KeyCode::Char(' ') {
                                app.onboarding = Some(OnboardingStep::AskName);
                                app.messages.push(DisplayMessage::System(
                                    "What should I call you?".into(),
                                ));
                            }
                            continue;
                        }
                        OnboardingStep::AskName => {
                            if key.code == KeyCode::Enter {
                                let text: String = app.input.lines().join("").trim().to_string();
                                if !text.is_empty() {
                                    app.onboarding_name = text.clone();
                                    app.messages.push(DisplayMessage::User(text));
                                    app.messages.push(DisplayMessage::System(
                                        "What do you do? (e.g. \"Year 3 CS student\", \"Senior backend engineer\", \"Data scientist\")".into(),
                                    ));
                                    app.input = TextArea::default();
                                    app.input.set_cursor_line_style(ratatui::style::Style::default());
                                    app.input.set_style(oni_core::palette::data_style());
                                    app.onboarding = Some(OnboardingStep::AskRole);
                                }
                            } else {
                                app.input.input(key);
                            }
                            continue;
                        }
                        OnboardingStep::AskRole => {
                            if key.code == KeyCode::Enter {
                                let text: String = app.input.lines().join("").trim().to_string();
                                if !text.is_empty() {
                                    app.onboarding_role = text.clone();
                                    app.messages.push(DisplayMessage::User(text));
                                    app.messages.push(DisplayMessage::System(
                                        "How do you prefer to work? (e.g. \"Direct, no hand-holding\", \"Explain your reasoning\", \"Just write the code\")".into(),
                                    ));
                                    app.input = TextArea::default();
                                    app.input.set_cursor_line_style(ratatui::style::Style::default());
                                    app.input.set_style(oni_core::palette::data_style());
                                    app.onboarding = Some(OnboardingStep::AskStyle);
                                }
                            } else {
                                app.input.input(key);
                            }
                            continue;
                        }
                        OnboardingStep::AskStyle => {
                            if key.code == KeyCode::Enter {
                                let text: String = app.input.lines().join("").trim().to_string();
                                if !text.is_empty() {
                                    app.onboarding_style = text.clone();
                                    app.messages.push(DisplayMessage::User(text));

                                    // Save onboarding data
                                    let _ = oni_core::personality::write_user(
                                        &app.onboarding_name,
                                        &app.onboarding_role,
                                        &app.onboarding_style,
                                        "",
                                    );
                                    let _ = oni_core::personality::write_soul(
                                        &oni_core::personality::default_soul(),
                                    );

                                    // Record relationship start
                                    let mut rel = oni_core::personality::RelationshipState::default();
                                    rel.on_session();
                                    rel.save();

                                    // Save default emotional state
                                    let emotions = oni_core::personality::EmotionalState::default();
                                    emotions.save();

                                    app.messages.push(DisplayMessage::System(format!(
                                        "Got it, {}. I'm ONI — your local AI coding assistant.\n\
                                        I run entirely on your machine. No cloud, no telemetry.\n\
                                        I'll learn how you work over time. The more we work together, the better I get.\n\n\
                                        Your profile is saved. Edit ~/.local/share/oni/SOUL.md to shape my personality.\n\
                                        Type anything to start working.",
                                        app.onboarding_name
                                    )));

                                    app.input = TextArea::default();
                                    app.input.set_cursor_line_style(ratatui::style::Style::default());
                                    app.input.set_style(oni_core::palette::data_style());
                                    app.onboarding = Some(OnboardingStep::Complete);
                                }
                            } else {
                                app.input.input(key);
                            }
                            continue;
                        }
                        OnboardingStep::Complete => {
                            if key.code == KeyCode::Enter {
                                app.onboarding = None;
                                app.messages.clear();
                                app.boot_frame = 0;
                                app.boot_complete = false;
                            } else {
                                app.input.input(key);
                                // Don't consume — let normal flow handle the first real message
                                app.onboarding = None;
                            }
                            continue;
                        }
                    }
                }

                // Handle pending tool proposal confirmation first
                if app.pending_proposal.is_some() {
                    let response = match key.code {
                        KeyCode::Char('y') | KeyCode::Char('Y') => Some(ConfirmResponse::Yes),
                        KeyCode::Char('n') | KeyCode::Char('N') => Some(ConfirmResponse::No),
                        KeyCode::Char('d') | KeyCode::Char('D') => Some(ConfirmResponse::Diff),
                        KeyCode::Char('a') | KeyCode::Char('A') => Some(ConfirmResponse::Always),
                        KeyCode::Enter => Some(ConfirmResponse::Yes), // Enter = yes
                        KeyCode::Esc => Some(ConfirmResponse::No),   // Esc = no
                        _ => None,
                    };
                    if let Some(resp) = response {
                        if let Some(proposal) = app.pending_proposal.take() {
                            let status = match resp {
                                ConfirmResponse::Yes => "APPROVED",
                                ConfirmResponse::No => "DENIED",
                                ConfirmResponse::Diff => "APPROVED (DIFF)",
                                ConfirmResponse::Always => "ALWAYS APPROVE",
                            };
                            app.messages.push(DisplayMessage::System(
                                format!("{}: {}", status, proposal.summary),
                            ));
                            let _ = proposal.respond.send(resp);
                        }
                        continue;
                    }
                    continue; // Swallow all other keys while proposal is pending
                }

                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('d'))
                {
                    app.should_quit = true;
                }
                // Ctrl+L — reset to chat view
                else if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('l')
                {
                    app.view_mode = ViewMode::Chat;
                    app.scroll_offset = 0;
                    app.scroll_locked_to_bottom = true;
                }
                // Ctrl+R — fuzzy reverse history search
                else if key.modifiers.contains(KeyModifiers::CONTROL)
                    && key.code == KeyCode::Char('r')
                {
                    if !app.command_history.is_empty() {
                        let current_text: String = app.input.lines().join("");
                        let search_term = current_text.trim().to_lowercase();

                        let start_idx = app
                            .history_index
                            .map(|i| i.saturating_sub(1))
                            .unwrap_or(app.command_history.len().saturating_sub(1));

                        // Search backwards from start_idx
                        for offset in 0..app.command_history.len() {
                            let idx = (start_idx + app.command_history.len() - offset)
                                % app.command_history.len();
                            if search_term.is_empty()
                                || app.command_history[idx]
                                    .to_lowercase()
                                    .contains(&search_term)
                            {
                                // Skip if already at this index
                                if app.history_index == Some(idx) && offset == 0 {
                                    continue;
                                }
                                app.history_index = Some(idx);
                                let entry = app.command_history[idx].clone();
                                app.input = TextArea::default();
                                app.input.set_cursor_line_style(
                                    ratatui::style::Style::default(),
                                );
                                app.input.set_style(oni_core::palette::data_style());
                                app.input.insert_str(&entry);
                                break;
                            }
                        }
                    }
                }
                // Escape — return to chat from other views, or close slash menu
                else if key.code == KeyCode::Esc && app.slash_menu_visible {
                    app.slash_menu_visible = false;
                    app.slash_menu_filter.clear();
                    app.slash_menu_selected = 0;
                } else if app.slash_menu_visible && key.code == KeyCode::Up {
                    let filtered_count =
                        crate::ui::command_menu::filtered_commands(&app.slash_menu_filter)
                            .len();
                    if filtered_count > 0 {
                        app.slash_menu_selected =
                            app.slash_menu_selected.saturating_sub(1);
                    }
                } else if app.slash_menu_visible && key.code == KeyCode::Down {
                    let filtered_count =
                        crate::ui::command_menu::filtered_commands(&app.slash_menu_filter)
                            .len();
                    if filtered_count > 0 {
                        app.slash_menu_selected = (app.slash_menu_selected + 1)
                            .min(filtered_count.saturating_sub(1));
                    }
                // History navigation — Up/Down when slash menu is NOT open
                } else if !app.slash_menu_visible && key.code == KeyCode::Up {
                    if !app.command_history.is_empty() {
                        let new_idx = match app.history_index {
                            None => app.command_history.len() - 1,
                            Some(i) => i.saturating_sub(1),
                        };
                        app.history_index = Some(new_idx);
                        let entry = app.command_history[new_idx].clone();
                        app.input = TextArea::default();
                        app.input.set_cursor_line_style(ratatui::style::Style::default());
                        app.input.set_style(oni_core::palette::data_style());
                        app.input.insert_str(&entry);
                    }
                } else if !app.slash_menu_visible && key.code == KeyCode::Down {
                    if let Some(i) = app.history_index {
                        if i + 1 < app.command_history.len() {
                            let new_idx = i + 1;
                            app.history_index = Some(new_idx);
                            let entry = app.command_history[new_idx].clone();
                            app.input = TextArea::default();
                            app.input
                                .set_cursor_line_style(ratatui::style::Style::default());
                            app.input.set_style(oni_core::palette::data_style());
                            app.input.insert_str(&entry);
                        } else {
                            // Past the end — clear input, exit history mode
                            app.history_index = None;
                            app.input = TextArea::default();
                            app.input
                                .set_cursor_line_style(ratatui::style::Style::default());
                            app.input.set_style(oni_core::palette::data_style());
                        }
                    }
                } else if app.slash_menu_visible
                    && (key.code == KeyCode::Tab || key.code == KeyCode::Enter)
                    && !app.is_thinking
                {
                    let filtered =
                        crate::ui::command_menu::filtered_commands(&app.slash_menu_filter);
                    if let Some(&(cmd, _)) = filtered.get(app.slash_menu_selected) {
                        // Commands with no required args submit immediately on Enter
                        let no_args = matches!(
                            cmd,
                            "/clear"
                                | "/model"
                                | "/doctor"
                                | "/help"
                                | "/quit"
                                | "/mc"
                                | "/prefs"
                                | "/chat"
                                | "/sidebar"
                                | "/undo"
                        );
                        if key.code == KeyCode::Enter && no_args {
                            let text = cmd.to_string();
                            app.input = TextArea::default();
                            app.input
                                .set_cursor_line_style(ratatui::style::Style::default());
                            app.input.set_style(oni_core::palette::data_style());
                            app.slash_menu_visible = false;
                            app.slash_menu_filter.clear();
                            app.slash_menu_selected = 0;
                            app.handle_slash_command(&text, &cmd_tx);
                            if app.view_mode == ViewMode::Preferences {
                                app.refresh_learned_rules(&db_path);
                            }
                            continue;
                        }
                        // Tab or Enter on /tier → fill input with trailing space for arg entry
                        let completed = format!("{} ", cmd);
                        app.input = TextArea::default();
                        app.input
                            .set_cursor_line_style(ratatui::style::Style::default());
                        app.input.set_style(oni_core::palette::data_style());
                        app.input.insert_str(&completed);
                    }
                    app.slash_menu_visible = false;
                    app.slash_menu_filter.clear();
                    app.slash_menu_selected = 0;
                } else if key.code == KeyCode::Enter
                    && key.modifiers.contains(KeyModifiers::SHIFT)
                    && !app.is_thinking
                {
                    // Shift+Enter — insert a newline for multiline input without submitting
                    app.input.insert_newline();
                } else if key.code == KeyCode::Enter && !app.is_thinking {
                    let text: String = app.input.lines().join("\n");
                    let text = text.trim().to_string();
                    // Always close the menu on enter
                    app.slash_menu_visible = false;
                    app.slash_menu_filter.clear();
                    app.slash_menu_selected = 0;

                    // If agent is waiting for a free-text answer, send it back directly.
                    if !text.is_empty() {
                        if let Some(pending) = app.pending_ask.take() {
                            app.messages.push(DisplayMessage::User(text.clone()));
                            app.input = TextArea::default();
                            app.input.set_cursor_line_style(ratatui::style::Style::default());
                            app.input.set_style(oni_core::palette::data_style());
                            let _ = pending.respond.send(text);
                            continue;
                        }
                    }

                    if !text.is_empty() {
                        // Push to history (avoid consecutive duplicates)
                        if app.command_history.last().map(String::as_str) != Some(&text) {
                            app.command_history.push(text.clone());
                        }
                        app.history_index = None;

                        app.input = TextArea::default();
                        app.input
                            .set_cursor_line_style(ratatui::style::Style::default());
                        app.input.set_style(oni_core::palette::data_style());

                        if app.handle_slash_command(&text, &cmd_tx) {
                            if app.view_mode == ViewMode::Preferences {
                                app.refresh_learned_rules(&db_path);
                            }
                            continue;
                        }

                        app.messages.push(DisplayMessage::User(text.clone()));
                        app.is_thinking = true;
                        app.scroll_locked_to_bottom = true;
                        app.scroll_offset = 0;

                        // Save user message to DB
                        if let Some(ref cid) = conv_id {
                            let _ = db.add_message(cid, "user", &text);
                        }

                        // Send to persistent agent (maintains conversation history!)
                        let _ = cmd_tx.send(AgentCommand::RunTurn(text));
                    }
                // Scroll handling
                } else if key.code == KeyCode::PageUp {
                    app.scroll_offset = app.scroll_offset.saturating_add(10);
                    app.scroll_locked_to_bottom = false;
                } else if key.code == KeyCode::PageDown {
                    app.scroll_offset = app.scroll_offset.saturating_sub(10);
                    if app.scroll_offset == 0 {
                        app.scroll_locked_to_bottom = true;
                    }
                } else if key.modifiers.contains(KeyModifiers::SHIFT) && key.code == KeyCode::Up {
                    app.scroll_offset = app.scroll_offset.saturating_add(3);
                    app.scroll_locked_to_bottom = false;
                } else if key.modifiers.contains(KeyModifiers::SHIFT) && key.code == KeyCode::Down {
                    app.scroll_offset = app.scroll_offset.saturating_sub(3);
                    if app.scroll_offset == 0 {
                        app.scroll_locked_to_bottom = true;
                    }
                } else {
                    app.input.input(key);
                    // Any keystroke exits history browsing mode
                    app.history_index = None;
                    // When user types, lock scroll to bottom
                    app.scroll_locked_to_bottom = true;
                    app.scroll_offset = 0;
                    // Update slash menu visibility after every keystroke
                    let current = app.input.lines().join("\n");
                    if current.starts_with('/') {
                        app.slash_menu_filter = current.clone();
                        app.slash_menu_visible = true;
                        let filtered_count =
                            crate::ui::command_menu::filtered_commands(
                                &app.slash_menu_filter,
                            )
                            .len();
                        if app.slash_menu_selected >= filtered_count {
                            app.slash_menu_selected = filtered_count.saturating_sub(1);
                        }
                    } else {
                        app.slash_menu_visible = false;
                        app.slash_menu_filter.clear();
                        app.slash_menu_selected = 0;
                    }
                }
            }
        }

        // Process agent events from the shared bus
        for event in app.event_bus.drain() {
            // Save assistant responses to DB
            if let AgentEvent::Response(ref text) = event {
                if let Some(ref cid) = conv_id {
                    let _ = db.add_message(cid, "assistant", text);
                }
            }
            app.handle_agent_event(event);
        }

        // Check for tool proposals that need user confirmation
        while let Ok(proposal) = proposal_rx.try_recv() {
            app.messages.push(DisplayMessage::System(format!(
                "CONFIRM: {} [y/n/d(iff)/a(lways)]",
                proposal.summary
            )));
            app.pending_proposal = Some(PendingProposal {
                name: proposal.name,
                summary: proposal.summary,
                args: proposal.args,
                respond: proposal.respond,
            });
        }

        // Check for ask_user requests from the agent
        while let Ok(ask_req) = ask_user_rx.try_recv() {
            app.messages.push(DisplayMessage::System(format!(
                "ONI ASKS: {} (type your answer and press Enter)",
                ask_req.question
            )));
            app.pending_ask = Some(PendingAsk {
                question: ask_req.question,
                respond: ask_req.respond,
            });
        }

        if app.is_thinking {
            app.throbber_state.calc_next();
        }

        // Boot sequence tick — advance frame until animation completes
        if !app.boot_complete {
            app.boot_frame = app.boot_frame.saturating_add(1);
            if app.boot_frame >= 22 {
                app.boot_complete = true;
            }
        }

        // Glitch pulse tick — advance frame counter on error transition
        {
            let has_error = app.critical_error.is_some();
            if has_error && !app.prev_had_error {
                // Error just appeared — start glitch sequence
                app.glitch_frame = Some(0);
            }
            app.prev_had_error = has_error;
            if let Some(f) = app.glitch_frame {
                if f >= 3 {
                    app.glitch_frame = None; // Done — show static error
                } else {
                    app.glitch_frame = Some(f + 1);
                }
            }
        }

        // Scan reveal tick — advance progress when a new assistant message arrived
        {
            let assistant_count = app.messages.iter().filter(|m| matches!(m, DisplayMessage::Assistant(_))).count();
            if assistant_count > app.reveal_msg_count {
                // New response — start reveal
                app.reveal_msg_count = assistant_count;
                app.reveal_progress = 0.0;
            }
            if app.reveal_progress < 1.0 {
                // Slower reveal: ~1.5 columns per tick at 30fps => full screen in ~80 frames (~2.7s)
                app.reveal_progress = (app.reveal_progress + 0.012).min(1.0);
            }
        }

        // File watcher: poll for changed files and trigger re-index
        if let Some(ref watcher) = _file_watcher {
            let changed = watcher.poll();
            if !changed.is_empty() {
                app.boot_file_count = changed.len();
                // Re-index changed files in the background
                if let Some(ref dir) = project_dir {
                    let index_db = std::path::Path::new(dir).join(".oni").join("index.db");
                    if let Ok(conn) = rusqlite::Connection::open(&index_db) {
                        for path in &changed {
                            let _ = oni_context::indexer::index_single_file(&conn, path);
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    crossterm::execute!(
        std::io::stdout(),
        crossterm::event::DisableMouseCapture
    )?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::cursor::SetCursorStyle::DefaultUserShape
    )?;
    app.save_history();

    // Run reflection engine (background analysis of patterns)
    if app.turn_count > 0 {
        let reflection = oni_agent::reflection::reflect(&db_path);
        if !reflection.mutations.is_empty() {
            // For now, auto-apply mutations at High autonomy, log at Medium/Low
            for mutation in &reflection.mutations {
                if app.autonomy == AutonomyLevel::High {
                    oni_agent::reflection::apply_mutation(mutation);
                }
                // Always journal the mutation proposal
                oni_core::personality::append_journal(&format!(
                    "- Reflection: {} [{}]",
                    mutation.description,
                    if app.autonomy == AutonomyLevel::High { "APPLIED" } else { "PROPOSED" }
                ));
            }
        }
    }

    // Write session journal entry
    if app.turn_count > 0 {
        let project = project_dir.as_deref().unwrap_or("unknown");
        let highlights: Vec<String> = app.messages.iter().filter_map(|m| {
            match m {
                DisplayMessage::Plan(steps) => Some(format!("Plan: {} steps", steps.len())),
                DisplayMessage::CriticVerdict { accepted, reason } => {
                    if *accepted { Some("Critic: ACCEPTED".into()) } else { Some(format!("Critic: REJECTED — {}", reason)) }
                }
                _ => None,
            }
        }).take(5).collect();
        oni_core::personality::write_session_summary(
            &app.session_id,
            project,
            app.turn_count,
            app.total_tokens,
            &highlights,
        );

        // Save emotional state
        let emotions = oni_core::personality::EmotionalState::load();
        emotions.save();
    }

    ratatui::restore();
    Ok(())
}

/// Auto-start a llama-server for a given tier if it's not already running.
/// Called from the SetTier command handler inside the async task.
async fn ensure_tier_server(tier: ModelTier, server_config: &ServerConfig) {
    let tier_name = match tier {
        ModelTier::Heavy => "heavy",
        ModelTier::Medium => "medium",
        ModelTier::General => "general",
        ModelTier::Fast => "fast",
        ModelTier::Embed => "embed",
    };

    let tier_url = match server_config.tier_urls.get(tier_name) {
        Some(url) => url.clone(),
        None => return,
    };

    let tier_cfg = match server_config.tiers.get(tier_name) {
        Some(cfg) => cfg.clone(),
        None => return,
    };

    // Health check — already running?
    let healthy = reqwest::Client::new()
        .get(format!("{}/health", tier_url))
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    if healthy {
        return;
    }

    // Find llama-server
    let llama_server = match which::which("llama-server") {
        Ok(p) => p,
        Err(_) => return,
    };

    // Resolve model path
    let models_dir = if let Some(rest) = server_config.models_dir.strip_prefix("~/") {
        dirs::home_dir().map(|h| h.join(rest)).unwrap_or_else(|| std::path::PathBuf::from(&server_config.models_dir))
    } else {
        std::path::PathBuf::from(&server_config.models_dir)
    };
    let gguf_path = models_dir.join(&tier_cfg.gguf);
    if !gguf_path.exists() {
        return;
    }

    // Extract port
    let port = tier_url
        .rsplit(':')
        .next()
        .and_then(|s| s.trim_end_matches('/').parse::<u16>().ok())
        .unwrap_or(8080);

    let log_path = format!("/tmp/oni-{}.log", tier_name);
    let log_file = match std::fs::File::create(&log_path) {
        Ok(f) => f,
        Err(_) => return,
    };
    let log_stderr = match log_file.try_clone() {
        Ok(f) => f,
        Err(_) => return,
    };

    let mut cmd = std::process::Command::new(&llama_server);
    cmd.arg("--model").arg(&gguf_path)
        .arg("--port").arg(port.to_string())
        .arg("--ctx-size").arg(tier_cfg.ctx_size.to_string())
        .arg("--n-gpu-layers").arg(tier_cfg.gpu_layers.to_string())
        .arg("--threads").arg(tier_cfg.threads.to_string())
        .arg("--threads-batch").arg(tier_cfg.threads_batch.to_string())
        .arg("--parallel").arg(tier_cfg.parallel.to_string());

    if tier_cfg.flash_attn {
        cmd.arg("-fa").arg("on");
    }
    if let Some(ref k) = tier_cfg.cache_type_k {
        cmd.arg("--cache-type-k").arg(k);
    }
    if let Some(ref v) = tier_cfg.cache_type_v {
        cmd.arg("--cache-type-v").arg(v);
    }
    for arg in &tier_cfg.extra_args {
        cmd.arg(arg);
    }

    cmd.stdout(log_file)
        .stderr(log_stderr)
        .stdin(std::process::Stdio::null());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    if cmd.spawn().is_ok() {
        // Wait for health (up to 120s)
        let start = std::time::Instant::now();
        while start.elapsed() < std::time::Duration::from_secs(120) {
            let ok = reqwest::Client::new()
                .get(format!("{}/health", tier_url))
                .timeout(std::time::Duration::from_secs(2))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            if ok {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
}
