use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
#[cfg(unix)]
use libc;
use oni_core::config::{data_dir, load_config};
use oni_core::types::ModelTier;
use oni_llm::{ModelRouter, LlmClient};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

mod server_manager;

/// Convert config tier_urls (String keys) to ModelTier keys for the router.
fn parse_tier_urls(config_urls: &HashMap<String, String>) -> HashMap<ModelTier, String> {
    config_urls
        .iter()
        .filter_map(|(k, v)| ModelTier::from_key(k).map(|tier| (tier, v.clone())))
        .collect()
}

#[derive(Parser)]
#[command(name = "oni", about = "ONBOARD NATIVE INTELLIGENCE — local AI assistant")]
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
        #[arg(long, alias = "execute")]
        exec: bool,
        /// Model tier to use (heavy/medium/general/fast)
        #[arg(long, default_value = "medium")]
        tier: String,
        /// Autonomy level (low/medium/high)
        #[arg(long, default_value = "medium")]
        autonomy: String,
        /// Per-session token budget (0 = unlimited)
        #[arg(long, default_value = "0")]
        budget: u64,
        /// Wipe all data and start fresh (triggers onboarding)
        #[arg(long)]
        fresh: bool,
    },
    /// Ask a one-shot question (supports stdin pipe: echo "q" | oni ask)
    Ask {
        /// Model tier to use (heavy/medium/general/fast)
        #[arg(long, default_value = "fast")]
        tier: String,
        /// Output NDJSON event stream instead of plain text
        #[arg(long)]
        json: bool,
        /// The question (reads from stdin if empty)
        question: Vec<String>,
    },
    /// Run a task headlessly (no TUI) — for benchmarking and debugging
    Run {
        /// Model tier to use
        #[arg(long, default_value = "code")]
        tier: String,
        /// Maximum tool rounds
        #[arg(long, default_value = "15")]
        max_rounds: usize,
        /// Run in background (detached process)
        #[arg(long)]
        background: bool,
        /// List all background tasks
        #[arg(long)]
        list: bool,
        /// Kill a background task by ID
        #[arg(long)]
        kill: Option<String>,
        /// Show logs for a background task
        #[arg(long)]
        logs: Option<String>,
        /// Show status details for a specific task
        #[arg(long)]
        status: Option<String>,
        /// Enable deep telemetry (saves JSON report to ~/.local/share/oni/telemetry/)
        #[arg(long)]
        telemetry: bool,
        /// Save telemetry to a specific file path
        #[arg(long)]
        telemetry_out: Option<String>,
        // ── Feature flags (disable specific features for A/B testing) ──
        /// Disable knowledge graph context injection
        #[arg(long)]
        no_knowledge_graph: bool,
        /// Disable reflection engine
        #[arg(long)]
        no_reflection: bool,
        /// Disable personality (SOUL.md)
        #[arg(long)]
        no_personality: bool,
        /// Disable memory callbacks
        #[arg(long)]
        no_callbacks: bool,
        /// Disable context compaction
        #[arg(long)]
        no_compaction: bool,
        /// Disable multi-trajectory sampling
        #[arg(long)]
        no_multi_trajectory: bool,
        /// Disable orchestrator (flat mode only)
        #[arg(long)]
        no_orchestrator: bool,
        /// Disable auto-lint after writes
        #[arg(long)]
        no_auto_lint: bool,
        /// Disable emotional state tracking
        #[arg(long)]
        no_emotional_state: bool,
        /// Disable forge tool (dynamic tool generation)
        #[arg(long)]
        no_forge_tool: bool,
        /// Disable undo tracking
        #[arg(long)]
        no_undo: bool,
        /// The task/prompt to execute
        prompt: Vec<String>,
    },
    /// Check system health
    Doctor,
    /// Index the current project for context retrieval
    Init,
    /// Show or rebuild the project index
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
    /// Show or manage learned preferences
    Prefs {
        #[command(subcommand)]
        action: Option<PrefsAction>,
    },
    /// Show or set configuration values
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    /// Pin context retrieval to a subtree
    Pin {
        /// Path to pin to, or --reset to clear
        #[arg(default_value = "")]
        path: String,
        /// Clear the pin
        #[arg(long)]
        reset: bool,
    },
    /// Review staged git changes
    Review {
        /// Model tier for review (default: general for speed)
        #[arg(long, default_value = "general")]
        tier: String,
    },
    /// Run a multi-step autonomous task (codebase-wide operations)
    Sweep {
        /// Model tier for planning
        #[arg(long, default_value = "heavy")]
        tier: String,
        /// Maximum total tool rounds
        #[arg(long, default_value = "30")]
        max_rounds: usize,
        /// Preview changes without writing (default: dry-run)
        #[arg(long)]
        write: bool,
        /// Filter files by glob pattern (e.g. "src/**/*.ts")
        #[arg(long)]
        glob: Option<String>,
        /// The goal to accomplish
        goal: Vec<String>,
    },
}

#[derive(Subcommand)]
enum IndexAction {
    /// Show index statistics
    Stats,
    /// Rebuild the project index from scratch
    Rebuild,
}

#[derive(Subcommand)]
enum PrefsAction {
    /// Show learned preferences
    Show,
    /// Reset all learned preferences
    Reset,
    /// Export preferences to JSONL file
    Export {
        /// Output file path (default: stdout)
        #[arg(default_value = "-")]
        path: String,
    },
    /// Import preferences from JSONL file
    Import {
        /// Input file path
        path: String,
    },
    /// Forget preferences for a specific tool
    Forget {
        /// Tool name to forget
        tool: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current config
    Show,
    /// Set a config value
    Set { key: String, value: String },
}

// ---------------------------------------------------------------------------
// Background task tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackgroundTask {
    id: String,
    prompt: String,
    tier: String,
    status: String, // "running", "done", "error", "killed"
    pid: u32,
    start_time: String,
    log_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
}

fn tasks_file() -> std::path::PathBuf {
    data_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        .join("tasks.json")
}

fn load_tasks() -> Vec<BackgroundTask> {
    let path = tasks_file();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_tasks(tasks: &[BackgroundTask]) {
    let path = tasks_file();
    let _ = std::fs::write(
        &path,
        serde_json::to_string_pretty(tasks).unwrap_or_default(),
    );
}

fn generate_task_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("task_{}", ts % 1_000_000)
}

fn timestamp_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

/// Char-boundary-safe string truncation (returns a &str slice or full string).
fn truncate_str(s: &str, max_chars: usize) -> &str {
    let mut char_count = 0;
    for (byte_idx, _) in s.char_indices() {
        if char_count == max_chars {
            return &s[..byte_idx];
        }
        char_count += 1;
    }
    s
}

/// Returns true if a process with the given PID exists (Unix: kill(pid, 0)).
fn pid_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
        result == 0
    }
    #[cfg(not(unix))]
    {
        // On Windows, try to open the process; fall back to assuming alive
        let _ = pid;
        true
    }
}

// ---------------------------------------------------------------------------

fn parse_tier(s: &str) -> ModelTier {
    match s.to_lowercase().as_str() {
        "heavy" | "h" => ModelTier::Heavy,
        "medium" | "code" | "m" | "c" => ModelTier::Medium,
        "general" | "gen" | "g" => ModelTier::General,
        "fast" | "f" | "quick" => ModelTier::Fast,
        _ => ModelTier::Fast,
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Log to file, never stdout (TUI owns terminal)
    let log_dir = data_dir().unwrap_or_else(|_| std::path::PathBuf::from("/tmp"));
    let log_file = std::fs::File::create(log_dir.join("oni.log")).ok();
    if let Some(file) = log_file {
        tracing_subscriber::fmt()
            .with_env_filter("oni=info")
            .with_writer(file)
            .init();
    }

    let cli = Cli::parse();

    // Auto-start llama-server instances for commands that need LLM access.
    // Only start the tiers actually needed for the command.
    let needed_tiers: Option<Vec<&str>> = match &cli.command {
        Some(Commands::Chat { tier, .. }) => {
            // Chat needs the specified tier + heavy+general for orchestrator
            let t = tier.as_str();
            Some(vec![t, "heavy", "general"])
        }
        Some(Commands::Ask { tier, .. }) => {
            Some(vec![tier.as_str()])
        }
        Some(Commands::Review { .. }) | Some(Commands::Sweep { .. }) => {
            Some(vec!["medium", "general"])
        }
        Some(Commands::Run { list, kill, logs, status, background, .. }) => {
            if *list || kill.is_some() || logs.is_some() || status.is_some() || *background {
                None // management flags don't need LLM
            } else {
                Some(vec!["medium"])
            }
        }
        None => Some(vec!["medium"]), // default command
        _ => None,
    };

    if let Some(ref tiers) = needed_tiers {
        let config = load_config(Some(&std::env::current_dir()?))?;
        if let Err(e) = server_manager::ensure_servers_running(&config, Some(tiers)).await {
            eprintln!("  server auto-start failed: {}", e);
        }
    }

    match cli.command {
        Some(Commands::Chat {
            write,
            exec,
            tier,
            autonomy,
            budget,
            fresh,
        }) => {
            if fresh {
                eprintln!("Resetting ONI to factory state...");
                oni_core::personality::fresh_reset()?;
                let db_path = data_dir()?.join("oni.db");
                if db_path.exists() {
                    let _ = std::fs::remove_file(&db_path);
                }
                eprintln!("Done. Starting onboarding.");
            }
            run_chat(write, exec, &tier, &autonomy, budget).await
        }
        Some(Commands::Ask {
            tier,
            json,
            question,
        }) => {
            let q = if question.is_empty() {
                // Read from stdin
                use std::io::Read;
                let mut input = String::new();
                std::io::stdin().read_to_string(&mut input)?;
                input.trim().to_string()
            } else {
                question.join(" ")
            };
            if json {
                run_ask_json(&q, parse_tier(&tier)).await
            } else {
                run_ask(&q, parse_tier(&tier)).await
            }
        }
        Some(Commands::Run {
            tier,
            max_rounds,
            background,
            list,
            kill,
            logs,
            status: status_id,
            telemetry,
            telemetry_out,
            no_knowledge_graph,
            no_reflection,
            no_personality,
            no_callbacks,
            no_compaction,
            no_multi_trajectory,
            no_orchestrator,
            no_auto_lint,
            no_emotional_state,
            no_forge_tool,
            no_undo,
            prompt,
        }) => {
            if list {
                let mut tasks = load_tasks();
                // Refresh liveness of running tasks
                let mut dirty = false;
                for t in tasks.iter_mut() {
                    if t.status == "running" && !pid_is_alive(t.pid) {
                        t.status = "failed".into();
                        t.completed_at = Some(timestamp_string());
                        dirty = true;
                    }
                }
                if dirty {
                    save_tasks(&tasks);
                }
                if tasks.is_empty() {
                    println!("No background tasks.");
                } else {
                    println!("{:<14} {:<10} {:<8} {}", "ID", "STATUS", "TIER", "PROMPT");
                    for t in &tasks {
                        let preview = truncate_str(&t.prompt, 50);
                        println!("{:<14} {:<10} {:<8} {}", t.id, t.status, t.tier, preview);
                    }
                }
                return Ok(());
            }
            if let Some(id) = kill {
                let mut tasks = load_tasks();
                if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                    #[cfg(unix)]
                    unsafe {
                        libc::kill(task.pid as libc::pid_t, libc::SIGTERM);
                    }
                    #[cfg(not(unix))]
                    {
                        let _ = std::process::Command::new("taskkill")
                            .args(["/PID", &task.pid.to_string(), "/F"])
                            .status();
                    }
                    task.status = "killed".into();
                    task.completed_at = Some(timestamp_string());
                    save_tasks(&tasks);
                    println!("Killed task {}", id);
                } else {
                    println!("Task {} not found", id);
                }
                return Ok(());
            }
            if let Some(id) = logs {
                let tasks = load_tasks();
                if let Some(task) = tasks.iter().find(|t| t.id == id) {
                    let content = std::fs::read_to_string(&task.log_path)
                        .unwrap_or_else(|_| "No logs yet.".into());
                    println!("{}", content);
                } else {
                    println!("Task {} not found", id);
                }
                return Ok(());
            }
            if let Some(id) = status_id {
                let mut tasks = load_tasks();
                if let Some(task) = tasks.iter_mut().find(|t| t.id == id) {
                    // Refresh liveness for running tasks
                    if task.status == "running" && !pid_is_alive(task.pid) {
                        task.status = "failed".into();
                        task.completed_at = Some(timestamp_string());
                        save_tasks(&tasks);
                    }
                    // Re-borrow after potential save
                    let task = tasks.iter().find(|t| t.id == id).unwrap();
                    println!("ID:           {}", task.id);
                    println!("STATUS:       {}", task.status);
                    println!("TIER:         {}", task.tier);
                    println!("PID:          {}", task.pid);
                    println!("STARTED:      {}", task.start_time);
                    if let Some(ref c) = task.completed_at {
                        println!("COMPLETED:    {}", c);
                    }
                    if let Some(code) = task.exit_code {
                        println!("EXIT CODE:    {}", code);
                    }
                    println!("LOG:          {}", task.log_path);
                    println!("PROMPT:       {}", task.prompt);
                } else {
                    println!("Task {} not found", id);
                }
                return Ok(());
            }
            if background {
                let p = prompt.join(" ");
                let task_id = generate_task_id();
                let log_path = data_dir()?.join(format!("{}.log", task_id));

                let log_file = std::fs::File::create(&log_path)?;
                #[allow(unused_mut)]
                let mut cmd = std::process::Command::new(std::env::current_exe()?);
                cmd.args(["run", "--tier", &tier, "--max-rounds", &max_rounds.to_string()])
                    .args(&prompt)
                    .stdout(log_file.try_clone()?)
                    .stderr(log_file)
                    .stdin(std::process::Stdio::null());

                // Detach from the parent's process group so the child survives parent exit
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

                let child = cmd.spawn()?;

                let mut tasks = load_tasks();
                tasks.push(BackgroundTask {
                    id: task_id.clone(),
                    prompt: p,
                    tier: tier.clone(),
                    status: "running".into(),
                    pid: child.id(),
                    start_time: timestamp_string(),
                    log_path: log_path.to_string_lossy().to_string(),
                    completed_at: None,
                    exit_code: None,
                });
                save_tasks(&tasks);

                println!("Background task started: {}", task_id);
                println!("  Logs:   oni run --logs {}", task_id);
                println!("  Status: oni run --status {}", task_id);
                return Ok(());
            }
            let p = prompt.join(" ");
            let flags = oni_agent::telemetry::FeatureFlags {
                knowledge_graph: !no_knowledge_graph,
                reflection: !no_reflection,
                personality: !no_personality,
                callbacks: !no_callbacks,
                compaction: !no_compaction,
                multi_trajectory: !no_multi_trajectory,
                orchestrator: !no_orchestrator,
                auto_lint: !no_auto_lint,
                emotional_state: !no_emotional_state,
                forge_tool: !no_forge_tool,
                undo_tracking: !no_undo,
            };
            run_headless(&p, parse_tier(&tier), max_rounds, telemetry, telemetry_out, flags).await
        }
        Some(Commands::Doctor) => run_doctor().await,
        Some(Commands::Init) => run_init().await,
        Some(Commands::Index { action }) => match action {
            IndexAction::Stats => run_index_stats().await,
            IndexAction::Rebuild => run_init().await,
        },
        Some(Commands::Prefs { action }) => match action {
            Some(PrefsAction::Show) | None => run_prefs_show().await,
            Some(PrefsAction::Reset) => run_prefs_reset().await,
            Some(PrefsAction::Export { path }) => run_prefs_export(&path).await,
            Some(PrefsAction::Import { path }) => run_prefs_import(&path).await,
            Some(PrefsAction::Forget { tool }) => run_prefs_forget(&tool).await,
        },
        Some(Commands::Config { action }) => match action {
            Some(ConfigAction::Show) | None => run_config_show().await,
            Some(ConfigAction::Set { key, value }) => run_config_set(&key, &value).await,
        },
        Some(Commands::Pin { path, reset }) => {
            let cwd = std::env::current_dir()?;
            if reset || path.is_empty() {
                oni_context::retriever::set_pin(&cwd, None)
                    .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
                println!("Pin cleared. Context retrieval now uses full project.");
            } else {
                oni_context::retriever::set_pin(&cwd, Some(&path))
                    .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
                println!("Pinned to: {}", path);
            }
            Ok(())
        }
        Some(Commands::Review { tier }) => run_review(parse_tier(&tier)).await,
        Some(Commands::Sweep {
            tier,
            max_rounds,
            write,
            glob,
            goal,
        }) => {
            let g = goal.join(" ");
            let mode = if write { "WRITE" } else { "DRY-RUN" };
            let glob_info = glob.as_deref().unwrap_or("*");
            eprintln!("ONI SWEEP [{}] glob={}", mode, glob_info);
            if !write {
                eprintln!("  (Preview only. Use --write to apply changes.)");
            }

            // Build prompt with sweep context
            let prompt = if let Some(ref pattern) = glob {
                format!(
                    "{}. Only modify files matching the glob pattern '{}'. {}",
                    g,
                    pattern,
                    if !write {
                        "Show what you WOULD change but do NOT write any files."
                    } else {
                        ""
                    }
                )
            } else if !write {
                format!(
                    "{}. Show what you WOULD change but do NOT write any files — this is a dry run.",
                    g
                )
            } else {
                g
            };

            run_headless(
                &prompt,
                parse_tier(&tier),
                max_rounds,
                false,
                None,
                oni_agent::telemetry::FeatureFlags::default(),
            ).await
        }
        None => {
            // Default: launch chat with all permissions, medium autonomy
            run_chat(true, true, "medium", "medium", 0).await
        }
    }
}

fn parse_autonomy(s: &str) -> oni_core::types::AutonomyLevel {
    match s.to_lowercase().as_str() {
        "low" | "l" => oni_core::types::AutonomyLevel::Low,
        "high" | "h" => oni_core::types::AutonomyLevel::High,
        _ => oni_core::types::AutonomyLevel::Medium,
    }
}

async fn run_chat(
    write: bool,
    exec: bool,
    tier_str: &str,
    autonomy_str: &str,
    budget: u64,
) -> Result<()> {
    let mut config = load_config(Some(&std::env::current_dir()?))?;
    let client = LlmClient::new(&config.server.base_url, config.server.timeout_secs);

    // Wire --tier flag through
    config.models.default_tier = parse_tier(tier_str);

    let router = Arc::new(
        ModelRouter::new_with_tier_urls(client, config.models.clone(), parse_tier_urls(&config.server.tier_urls))
            .with_reasoning(config.agent.reasoning.clone()),
    );

    let db_path = data_dir()?.join("oni.db");
    let db = oni_db::Database::open(&db_path)?;

    let mut agent_config = config.agent.clone();
    agent_config.allow_write = write;
    agent_config.allow_exec = exec;
    agent_config.autonomy = parse_autonomy(autonomy_str);
    if budget > 0 {
        agent_config.session_budget = budget;
    }

    oni_tui::run(router, db, db_path, agent_config, config.ui, config.models, config.server).await
}

async fn run_ask(question: &str, tier: ModelTier) -> Result<()> {
    let config = load_config(Some(&std::env::current_dir()?))?;
    let client = LlmClient::new(&config.server.base_url, config.server.timeout_secs);
    let router = ModelRouter::new_with_tier_urls(client, config.models.clone(), parse_tier_urls(&config.server.tier_urls));

    let messages = vec![
        oni_llm::ChatMessage::system("You are ONI, a concise local AI assistant. Answer directly."),
        oni_llm::ChatMessage::user(question),
    ];

    eprintln!("PROCESSING... [{}]", router.model_name(tier));
    let response = router.chat(tier, messages).await?;
    println!("{}", response.message().content);

    if let Some(usage) = &response.usage {
        eprintln!("\n[{} tokens]", usage.total_tokens.unwrap_or(usage.completion_tokens));
    }

    Ok(())
}

async fn run_headless(
    prompt: &str,
    tier: ModelTier,
    max_rounds: usize,
    telemetry_enabled: bool,
    telemetry_out: Option<String>,
    feature_flags: oni_agent::telemetry::FeatureFlags,
) -> Result<()> {
    use oni_agent::agent::{Agent, AgentEvent};

    let config = load_config(Some(&std::env::current_dir()?))?;
    let client = LlmClient::new(&config.server.base_url, config.server.timeout_secs);
    let router = Arc::new(
        ModelRouter::new_with_tier_urls(client, config.models.clone(), parse_tier_urls(&config.server.tier_urls))
            .with_reasoning(config.agent.reasoning.clone()),
    );

    let project_dir = std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    // Show disabled features if any
    let disabled: Vec<&str> = [
        (!feature_flags.knowledge_graph, "knowledge_graph"),
        (!feature_flags.reflection, "reflection"),
        (!feature_flags.personality, "personality"),
        (!feature_flags.callbacks, "callbacks"),
        (!feature_flags.compaction, "compaction"),
        (!feature_flags.multi_trajectory, "multi_trajectory"),
        (!feature_flags.orchestrator, "orchestrator"),
        (!feature_flags.auto_lint, "auto_lint"),
        (!feature_flags.emotional_state, "emotional_state"),
        (!feature_flags.forge_tool, "forge_tool"),
        (!feature_flags.undo_tracking, "undo_tracking"),
    ]
    .iter()
    .filter(|(off, _)| *off)
    .map(|(_, name)| *name)
    .collect();

    eprintln!("══════════════════════════════════════════════════");
    eprintln!("  ONI DEBUG RUN");
    eprintln!(
        "  MODEL: {} [{}]",
        router.model_name(tier),
        tier.display_name()
    );
    eprintln!("  MAX_ROUNDS: {}", max_rounds);
    eprintln!("  TELEMETRY: {}", if telemetry_enabled { "ON" } else { "OFF" });
    if !disabled.is_empty() {
        eprintln!("  DISABLED: {}", disabled.join(", "));
    }
    eprintln!("  CWD: {}", project_dir.as_deref().unwrap_or("?"));
    eprintln!("══════════════════════════════════════════════════");
    eprintln!();
    eprintln!("PROMPT: {}", prompt);
    eprintln!();

    let event_bus = oni_agent::message_bus::MessageBus::<AgentEvent>::new(500);

    let mut agent = Agent::new_with_prefs(
        router,
        true, // allow_write
        true, // allow_exec
        max_rounds,
        tier,
        project_dir.as_deref(),
        None,
        None,
        config.agent.compaction.clone(),
        feature_flags,
        telemetry_enabled,
    );
    agent.set_event_bus(event_bus.clone());

    // If orchestrator is disabled via feature flag, force flat mode
    if !agent.telemetry.flags().orchestrator {
        agent.use_orchestration = false;
    }

    // Clone telemetry handle before moving agent (Arc<Mutex> — cheap clone)
    let telemetry_handle = agent.telemetry.clone();

    // Run agent in background task
    let prompt_owned = prompt.to_string();
    let mut agent_handle = tokio::spawn(async move { agent.run_turn(&prompt_owned).await });

    // Poll the bus for events while the agent task runs
    let mut final_response = String::new();
    let mut done = false;
    loop {
        // Check if agent task has finished
        match tokio::time::timeout(std::time::Duration::from_millis(50), &mut agent_handle).await {
            Ok(result) => {
                // Agent task completed — drain remaining events then break
                for event in event_bus.drain() {
                    handle_headless_event(&event, &mut final_response, &mut done);
                }
                match result {
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => eprintln!("[AGENT ERROR] {}", e),
                    Err(e) => eprintln!("[TASK ERROR] {}", e),
                }
                break;
            }
            Err(_) => {
                // Timeout — drain events published so far
                for event in event_bus.drain() {
                    handle_headless_event(&event, &mut final_response, &mut done);
                }
                if done {
                    break;
                }
            }
        }
    }

    // Print the final response
    if !final_response.is_empty() {
        println!("{}", final_response);
    }

    // Save telemetry if enabled
    if telemetry_enabled {
        let telem_path = if let Some(ref out) = telemetry_out {
            std::path::PathBuf::from(out)
        } else {
            let telem_dir = data_dir()?.join("telemetry");
            std::fs::create_dir_all(&telem_dir)?;
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            telem_dir.join(format!("run_{}.json", ts))
        };
        telemetry_handle.save_to_file(&telem_path);
        eprintln!();
        eprintln!("TELEMETRY: {}", telem_path.display());
        eprintln!("SUMMARY: {}", telemetry_handle.summary_string());
    }

    Ok(())
}

/// Process a single headless event — prints to stderr, sets done flag on terminal events.
fn handle_headless_event(
    event: &oni_agent::agent::AgentEvent,
    final_response: &mut String,
    done: &mut bool,
) {
    use oni_agent::agent::AgentEvent;
    match event {
        AgentEvent::Thinking => {
            eprintln!("[THINKING] Waiting for LLM response...");
        }
        AgentEvent::PlanGenerated { steps } => {
            eprintln!("[MIMIR] Generated {} steps:", steps.len());
            for (i, step) in steps.iter().enumerate() {
                eprintln!("  [MIMIR:STEP] {}. {}", i + 1, step);
            }
            eprintln!();
        }
        AgentEvent::ExecutorStep { step, total, description } => {
            eprintln!("[FENRIR] Step {}/{}: {}", step, total, description);
        }
        AgentEvent::CriticVerdict { accepted, reason } => {
            if *accepted {
                eprintln!("[SKULD] ACCEPTED");
            } else {
                eprintln!("[SKULD] REJECTED: {}", reason);
            }
            eprintln!();
        }
        AgentEvent::Replanning { cycle, reason } => {
            eprintln!("[MIMIR:REPLAN] Cycle {}: {}", cycle, reason);
            eprintln!();
        }
        AgentEvent::ToolExec { name, status, args, result } => {
            eprintln!("[TOOL] {} — {}", name.to_uppercase(), status.to_uppercase());
            if !args.is_null() {
                let args_str = serde_json::to_string_pretty(args).unwrap_or_default();
                if args_str.len() < 500 {
                    eprintln!("  ARGS: {}", args_str);
                } else {
                    eprintln!("  ARGS: {}...", &args_str[..500]);
                }
            }
            if let Some(ref r) = result {
                let preview = if r.len() > 300 { &r[..300] } else { r.as_str() };
                eprintln!("  RESULT: {}...", preview);
            }
            eprintln!();
        }
        AgentEvent::Response(text) => {
            *final_response = text.clone();
        }
        AgentEvent::Error(text) => {
            eprintln!("[ERROR] {}", text);
        }
        AgentEvent::BudgetExhausted { limit_type, used, limit } => {
            eprintln!("[BUDGET] {} LIMIT REACHED: {}/{} tokens", limit_type, used, limit);
            *done = true;
        }
        AgentEvent::Done { tokens, duration_ms } => {
            eprintln!();
            eprintln!("══════════════════════════════════════════════════");
            eprintln!("  DONE — {} tokens, {}ms", tokens, duration_ms);
            eprintln!("══════════════════════════════════════════════════");
            *done = true;
        }
    }
}

async fn run_init() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let oni_dir = cwd.join(".oni");
    std::fs::create_dir_all(&oni_dir)?;
    let db_path = oni_dir.join("index.db");

    let conn = Connection::open(&db_path)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to open index DB: {}", e))?;

    println!("Indexing {} ...", cwd.display());
    let count = oni_context::indexer::index_project(&conn, &cwd)
        .map_err(|e| color_eyre::eyre::eyre!("Indexing failed: {}", e))?;

    println!("Indexed {} files -> {}", count, db_path.display());
    Ok(())
}

async fn run_doctor() -> Result<()> {
    let config = load_config(None)?;
    let client = LlmClient::new(&config.server.base_url, config.server.timeout_secs);
    let router = ModelRouter::new_with_tier_urls(client, config.models.clone(), parse_tier_urls(&config.server.tier_urls));

    println!("ONI DOCTOR\n");

    // Check llama-server
    print!("  LLAMA-SERVER ... ");
    match router.client().health_check().await {
        Ok(_) => {
            println!("RUNNING");
        }
        Err(e) => {
            println!("ERROR: {}", e);
            return Ok(());
        }
    }

    // Check each model tier
    let model_status = router.check_all_models().await;
    for tier in [
        oni_core::types::ModelTier::Heavy,
        oni_core::types::ModelTier::Medium,
        oni_core::types::ModelTier::General,
        oni_core::types::ModelTier::Fast,
        oni_core::types::ModelTier::Embed,
    ] {
        let name = router.model_name(tier);
        let available = model_status.get(&tier).copied().unwrap_or(false);
        let status = if available { "OK" } else { "MISSING" };
        println!("  {} ({}) ... {}", tier.display_name(), name, status);
        if !available {
            println!("    -> load the GGUF file for '{}'", name);
        }
    }

    // Check data directory
    let data = data_dir()?;
    println!("\n  DATA_DIR: {}", data.display());

    // System info
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    println!("\n  SYSTEM: {} / {}", os, arch);

    println!("\nDONE");
    Ok(())
}

async fn run_review(tier: ModelTier) -> Result<()> {
    let config = load_config(Some(&std::env::current_dir()?))?;
    let client = LlmClient::new(&config.server.base_url, config.server.timeout_secs);
    let router = ModelRouter::new_with_tier_urls(client, config.models.clone(), parse_tier_urls(&config.server.tier_urls));

    let diff = oni_agent::review::get_staged_diff();
    let diff = match diff {
        Some(d) => d,
        None => {
            println!("No changes to review. Stage changes with `git add` first.");
            return Ok(());
        }
    };

    println!("ONI CODE REVIEW\n");
    println!("Reviewing {} lines of diff...\n", diff.lines().count());

    let result = oni_agent::review::review_diff(&router, &diff, tier, None).await?;

    // Print issues
    for issue in &result.issues {
        let prefix = match issue.severity {
            oni_agent::review::IssueSeverity::Error => "  [ERROR]",
            oni_agent::review::IssueSeverity::Warning => "  [WARN] ",
            oni_agent::review::IssueSeverity::Info => "  [INFO] ",
        };
        println!("{} {}: {}", prefix, issue.file, issue.description);
    }

    println!("\n{}", result.summary);
    println!("VERDICT: {}", result.verdict);
    Ok(())
}

async fn run_ask_json(question: &str, tier: ModelTier) -> Result<()> {
    let config = load_config(Some(&std::env::current_dir()?))?;
    let client = LlmClient::new(&config.server.base_url, config.server.timeout_secs);
    let router = ModelRouter::new_with_tier_urls(client, config.models.clone(), parse_tier_urls(&config.server.tier_urls));

    let messages = vec![
        oni_llm::ChatMessage::system(
            "You are ONI, a concise local AI assistant. Answer directly.",
        ),
        oni_llm::ChatMessage::user(question),
    ];

    // NDJSON event stream
    println!(
        "{}",
        serde_json::json!({"event": "turn_start", "model": router.model_name(tier), "tier": tier.display_name()})
    );
    println!("{}", serde_json::json!({"event": "thinking"}));

    let response = router.chat(tier, messages).await?;
    let msg = response.message();

    println!(
        "{}",
        serde_json::json!({
            "event": "response",
            "content": msg.content,
            "model": response.model,
            "prompt_tokens": response.prompt_tokens(),
            "completion_tokens": response.completion_tokens(),
        })
    );

    println!("{}", serde_json::json!({"event": "turn_end"}));
    Ok(())
}

async fn run_index_stats() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let db_path = cwd.join(".oni").join("index.db");

    if !db_path.exists() {
        println!("No index found. Run `oni init` first.");
        return Ok(());
    }

    let conn = Connection::open(&db_path)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to open index DB: {}", e))?;

    let file_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
        .unwrap_or(0);
    let symbol_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))
        .unwrap_or(0);

    println!("ONI INDEX STATS\n");
    println!("  DB: {}", db_path.display());
    println!("  FILES: {}", file_count);
    println!("  SYMBOLS: {}", symbol_count);
    Ok(())
}

async fn run_prefs_show() -> Result<()> {
    let db_path = data_dir()?.join("oni.db");
    let db = oni_db::Database::open(&db_path)?;

    let mut stmt = db
        .conn()
        .prepare(
            "SELECT description, context, confidence, observations, active FROM learned_rules ORDER BY confidence DESC",
        )
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    let rules: Vec<(String, String, f64, i64, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?
        .filter_map(|r| r.ok())
        .collect();

    println!("ONI LEARNED PREFERENCES\n");
    if rules.is_empty() {
        println!("  No learned rules yet. Use ONI and it will learn your preferences over time.");
    } else {
        for (desc, ctx, conf, obs, active) in &rules {
            let status = if *active == 1 {
                "ACTIVE"
            } else if *conf >= 0.5 {
                "LEARNING"
            } else {
                "WEAK"
            };
            println!("  [{:>5.0}%] [{}] {}", conf * 100.0, status, desc);
            println!("         {} | {} observations", ctx, obs);
            println!();
        }
    }
    Ok(())
}

async fn run_prefs_reset() -> Result<()> {
    let db_path = data_dir()?.join("oni.db");
    let db = oni_db::Database::open(&db_path)?;
    db.conn()
        .execute("DELETE FROM learned_rules", [])
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
    db.conn()
        .execute("DELETE FROM preference_signals", [])
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
    println!("All learned preferences reset.");
    Ok(())
}

async fn run_prefs_export(path: &str) -> Result<()> {
    let db_path = data_dir()?.join("oni.db");
    let db = oni_db::Database::open(&db_path)?;

    let mut stmt = db
        .conn()
        .prepare(
            "SELECT description, context, confidence, observations, active FROM learned_rules",
        )
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    let rules: Vec<serde_json::Value> = stmt
        .query_map([], |row| {
            Ok(serde_json::json!({
                "description": row.get::<_, String>(0)?,
                "context": row.get::<_, String>(1)?,
                "confidence": row.get::<_, f64>(2)?,
                "observations": row.get::<_, i64>(3)?,
                "active": row.get::<_, i64>(4)?,
            }))
        })
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?
        .filter_map(|r| r.ok())
        .collect();

    let output = rules
        .iter()
        .map(|r| serde_json::to_string(r).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");

    if path == "-" {
        println!("{}", output);
    } else {
        std::fs::write(path, &output)?;
        println!("Exported {} rules to {}", rules.len(), path);
    }
    Ok(())
}

async fn run_prefs_import(path: &str) -> Result<()> {
    let db_path = data_dir()?.join("oni.db");
    let db = oni_db::Database::open(&db_path)?;

    let content = std::fs::read_to_string(path)?;
    let mut imported = 0;

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(rule) = serde_json::from_str::<serde_json::Value>(line) {
            let desc = rule
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let ctx = rule
                .get("context")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let conf = rule
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5);
            let obs = rule
                .get("observations")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let active = rule
                .get("active")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let _ = db.conn().execute(
                "INSERT INTO learned_rules (description, context, confidence, observations, active) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![desc, ctx, conf, obs, active],
            );
            imported += 1;
        }
    }

    println!("Imported {} rules from {}", imported, path);
    Ok(())
}

async fn run_prefs_forget(tool: &str) -> Result<()> {
    let db_path = data_dir()?.join("oni.db");
    let db = oni_db::Database::open(&db_path)?;

    let deleted = db
        .conn()
        .execute(
            "DELETE FROM preference_signals WHERE tool_name = ?1",
            [tool],
        )
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    // Also remove any learned rules that mention this tool
    let rules_deleted = db
        .conn()
        .execute(
            "DELETE FROM learned_rules WHERE description LIKE ?1 OR context LIKE ?1",
            [&format!("%{}%", tool)],
        )
        .map_err(|e| color_eyre::eyre::eyre!("{}", e))?;

    println!(
        "Forgot {} signals and {} rules for tool '{}'",
        deleted, rules_deleted, tool
    );
    Ok(())
}

async fn run_config_show() -> Result<()> {
    let config = load_config(Some(&std::env::current_dir()?))?;
    let toml_str = toml::to_string_pretty(&config).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
    println!("{}", toml_str);
    Ok(())
}

async fn run_config_set(key: &str, value: &str) -> Result<()> {
    let (path, is_project) =
        oni_core::config::config_set(key, value).map_err(|e| color_eyre::eyre::eyre!("{}", e))?;
    let location = if is_project { path.display().to_string() } else { path.display().to_string() };
    println!("Set {} = {} in {}", key, value, location);
    Ok(())
}
