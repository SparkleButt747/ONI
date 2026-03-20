use crate::budget::BudgetTracker;
use crate::conversation::Conversation;
use crate::message_bus::MessageBus;
use crate::orchestrator::Orchestrator;
use crate::parsing::{extract_text_tool_call, strip_thinking};
use crate::preferences::{PreferenceEngine, SignalType};
use crate::system_prompt::{
    build_system_prompt, build_system_prompt_with_context_opts,
    build_system_prompt_with_rules,
};
use crate::telemetry::{FeatureFlags, Telemetry};
use crate::tools::{AskUserChannel, ToolRegistry};
use crate::trace::{ExecutionTrace, TraceEventType};
use oni_core::config::CompactionConfig;
use oni_core::error::Result;
use oni_core::types::{AutonomyLevel, ModelTier};
use oni_llm::ModelRouter;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// LLM is generating a response (spinner cue).
    Thinking,
    /// Planner produced a step list.
    PlanGenerated { steps: Vec<String> },
    /// Executor is working on a specific step.
    ExecutorStep { step: usize, total: usize, description: String },
    /// A tool was executed.
    /// `args` holds the raw JSON arguments so the TUI can render rich previews
    /// (e.g. diff view for write_file, command block for bash).
    ToolExec {
        name: String,
        status: String,
        args: serde_json::Value,
        result: Option<String>,
    },
    /// Critic gave a verdict on a step.
    CriticVerdict { accepted: bool, reason: String },
    /// Orchestrator is replanning after a rejection.
    Replanning { cycle: usize, reason: String },
    Response(String),
    Error(String),
    Done { tokens: u64, duration_ms: u64 },
    /// Budget exhausted — session or monthly limit hit.
    BudgetExhausted { limit_type: String, used: u64, limit: u64 },
}

/// A tool call that needs user confirmation before execution.
/// Sent from agent to TUI; TUI sends back a `ConfirmResponse` via the oneshot.
pub struct ToolProposal {
    pub name: String,
    pub args: serde_json::Value,
    /// Human-readable summary of what the tool will do.
    pub summary: String,
    /// Channel to send the user's decision back.
    pub respond: oneshot::Sender<ConfirmResponse>,
}

// ToolProposal can't derive Debug because of oneshot::Sender
impl std::fmt::Debug for ToolProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolProposal")
            .field("name", &self.name)
            .field("summary", &self.summary)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmResponse {
    /// Proceed with execution.
    Yes,
    /// Skip this tool call.
    No,
    /// Show the diff/details before deciding.
    Diff,
    /// Always auto-approve this tool in future (learned preference).
    Always,
}

pub struct Agent {
    router: Arc<ModelRouter>,
    tools: ToolRegistry,
    conversation: Conversation,
    budget: BudgetTracker,
    max_tool_rounds: usize,
    default_tier: ModelTier,
    project_dir: Option<String>,
    allow_write: bool,
    allow_exec: bool,
    /// When true, route complex tasks through the Planner→Executor→Critic loop.
    pub use_orchestration: bool,
    /// Preference engine — None when no DB path is configured.
    pref_engine: Option<PreferenceEngine>,
    /// Session ID propagated to preference signals.
    session_id: Option<String>,
    /// Autonomy level — controls when confirmations are required.
    pub autonomy: AutonomyLevel,
    /// Channel to send tool proposals that need user confirmation.
    /// None = no confirmation (auto-approve everything, for headless mode).
    proposal_tx: Option<mpsc::UnboundedSender<ToolProposal>>,
    /// Channel for ask_user tool — allows agent to pause and ask the user a question.
    ask_user_channel: AskUserChannel,
    /// Per-session token budget (0 = unlimited).
    session_budget: u64,
    /// Monthly token limit (0 = unlimited).
    monthly_limit: u64,
    /// Compaction thresholds and retention config.
    compaction: CompactionConfig,
    /// Deep telemetry accumulator.
    pub telemetry: Telemetry,
    /// Execution trace — records every tool call, model response and error.
    /// Wrapped in Arc<Mutex<>> so the TUI can hold a reference and read it.
    pub trace: Arc<Mutex<ExecutionTrace>>,
    /// Pub/sub event bus — replaces the old mpsc channel for AgentEvent delivery.
    /// Shared (via Arc) between agent, orchestrator, and TUI/headless consumer.
    pub event_bus: MessageBus<AgentEvent>,
}

impl Agent {
    pub fn new(
        router: Arc<ModelRouter>,
        allow_write: bool,
        allow_exec: bool,
        max_tool_rounds: usize,
        default_tier: ModelTier,
        project_dir: Option<&str>,
    ) -> Self {
        Self::new_with_prefs(
            router,
            allow_write,
            allow_exec,
            max_tool_rounds,
            default_tier,
            project_dir,
            None,
            None,
            CompactionConfig::default(),
            FeatureFlags::default(),
            true,
        )
    }

    pub fn new_with_prefs(
        router: Arc<ModelRouter>,
        allow_write: bool,
        allow_exec: bool,
        max_tool_rounds: usize,
        default_tier: ModelTier,
        project_dir: Option<&str>,
        db_path: Option<PathBuf>,
        session_id: Option<String>,
        compaction: CompactionConfig,
        feature_flags: FeatureFlags,
        telemetry_enabled: bool,
    ) -> Self {
        let pref_engine = db_path.map(PreferenceEngine::new);

        let ask_user_channel = AskUserChannel::new();
        let tools = ToolRegistry::new_with_channels(
            allow_write,
            allow_exec,
            Some(ask_user_channel.clone()),
        );

        // Build initial system prompt — inject active learned rules if engine is available
        let system_prompt = if let Some(ref engine) = pref_engine {
            let rules = engine.get_active_rules();
            build_system_prompt_with_rules(project_dir, default_tier, &tools.tool_names(), &rules)
        } else {
            build_system_prompt(project_dir, default_tier, &tools.tool_names())
        };

        Self {
            router,
            tools,
            conversation: Conversation::new(system_prompt),
            budget: BudgetTracker::new(),
            max_tool_rounds,
            default_tier,
            project_dir: project_dir.map(|s| s.to_string()),
            allow_write,
            allow_exec,
            use_orchestration: true,
            pref_engine,
            session_id,
            autonomy: AutonomyLevel::Medium,
            proposal_tx: None,
            ask_user_channel,
            session_budget: 0,
            monthly_limit: 0,
            compaction,
            telemetry: Telemetry::new(telemetry_enabled, feature_flags),
            trace: Arc::new(Mutex::new(ExecutionTrace::new(500))),
            event_bus: MessageBus::new(500),
        }
    }

    /// Returns a clone of the trace handle so the TUI can read it.
    pub fn trace_handle(&self) -> Arc<Mutex<ExecutionTrace>> {
        self.trace.clone()
    }

    /// Returns a clone of the event bus (same underlying Arc).
    pub fn event_bus(&self) -> MessageBus<AgentEvent> {
        self.event_bus.clone()
    }

    /// Replace the event bus with an externally-created one (so the TUI/headless
    /// consumer shares the same buffer).
    pub fn set_event_bus(&mut self, bus: MessageBus<AgentEvent>) {
        self.event_bus = bus;
    }

    /// Set the proposal channel for tool confirmation prompts.
    pub fn set_proposal_channel(&mut self, tx: mpsc::UnboundedSender<ToolProposal>) {
        self.proposal_tx = Some(tx);
    }

    /// Wire in the TUI-side ask_user receiver. The TUI calls this with an unbounded sender
    /// so the ask_user tool can push questions and the TUI can respond.
    pub fn set_ask_user_channel(
        &mut self,
        tx: tokio::sync::mpsc::UnboundedSender<crate::tools::ask_user::AskUserRequest>,
    ) {
        self.ask_user_channel.set_sender(tx);
    }

    /// Expose the ask_user channel so the TUI can clone/access it if needed.
    pub fn ask_user_channel(&self) -> &AskUserChannel {
        &self.ask_user_channel
    }

    /// Set autonomy level.
    pub fn set_autonomy(&mut self, level: AutonomyLevel) {
        self.autonomy = level;
    }

    /// Set session budget.
    pub fn set_budget(&mut self, session: u64, monthly: u64) {
        self.session_budget = session;
        self.monthly_limit = monthly;
    }

    /// Check if budget allows continuing. Returns Err description if exhausted.
    fn check_budget(&self) -> Option<AgentEvent> {
        if self.session_budget > 0 && self.budget.total_tokens() >= self.session_budget {
            return Some(AgentEvent::BudgetExhausted {
                limit_type: "SESSION".into(),
                used: self.budget.total_tokens(),
                limit: self.session_budget,
            });
        }
        // Monthly limit would need to be checked against persisted state
        // For now just check session budget
        None
    }

    /// Determine if a tool call needs user confirmation based on autonomy level.
    fn needs_confirmation(&self, tool_name: &str, _args: &serde_json::Value) -> bool {
        match self.autonomy {
            AutonomyLevel::Low => {
                // Everything except read_file and list_directory needs confirmation
                !matches!(tool_name, "read_file" | "list_directory" | "search_files")
            }
            AutonomyLevel::Medium => {
                // Writes that overwrite and destructive bash need confirmation
                match tool_name {
                    "bash" => true, // always confirm bash at medium
                    "write_file" => {
                        // Check if file exists (overwrite) — for now, always confirm writes
                        true
                    }
                    "edit_file" => true,
                    _ => false,
                }
            }
            AutonomyLevel::High => {
                // Only blocklisted bash commands need confirmation
                // (handled separately by the bash tool itself)
                false
            }
        }
    }

    /// Request confirmation from TUI. Returns true if approved.
    async fn request_confirmation(
        &self,
        name: &str,
        args: &serde_json::Value,
    ) -> bool {
        let Some(ref tx) = self.proposal_tx else {
            return true; // No channel = headless mode, auto-approve
        };

        let summary = build_tool_summary(name, args);
        let (respond_tx, respond_rx) = oneshot::channel();

        let proposal = ToolProposal {
            name: name.to_string(),
            args: args.clone(),
            summary,
            respond: respond_tx,
        };

        if tx.send(proposal).is_err() {
            return true; // Channel closed, auto-approve
        }

        let result = match respond_rx.await {
            Ok(ConfirmResponse::Yes | ConfirmResponse::Always) => true,
            Ok(ConfirmResponse::No) => false,
            Ok(ConfirmResponse::Diff) => true, // Show diff then proceed
            Err(_) => true, // Channel dropped
        };
        self.telemetry.confirmation(result);
        result
    }

    /// Decide whether this prompt should be routed through the orchestrator.
    /// Only triggers for genuinely multi-step tasks. Single-file prompts stay flat.
    ///
    /// Ablation data (2026-03-19): over-triggering cost 30% pass rate.
    /// The old heuristic matched "create"/"fix"/"write" — nearly everything.
    fn should_orchestrate(prompt: &str) -> bool {
        let lower = prompt.to_lowercase();

        // Strong multi-step signals — these almost always need planning
        let strong_signals = [
            "multiple files",
            "several files",
            "across the codebase",
            "step by step",
            "first do",
            "then do",
            "and then",
            "scaffold",
            "migrate",
            "set up a full",
            "build a full",
            "end to end",
            "e2e",
        ];
        if strong_signals.iter().any(|s| lower.contains(s)) {
            return true;
        }

        // Weak signals — need 2+ to trigger orchestration
        let weak_signals = [
            "refactor", "rewrite", "implement", "create", "build",
            "add feature", "configure", "generate", "fix",
        ];
        let weak_count = weak_signals.iter().filter(|s| lower.contains(**s)).count();
        if weak_count >= 2 {
            return true;
        }

        // Numbered lists in the prompt suggest multi-step intent
        let has_numbered_steps = (lower.contains("1)") || lower.contains("1."))
            && (lower.contains("2)") || lower.contains("2."));
        if has_numbered_steps {
            return true;
        }

        // Long prompts (>500 chars) with any weak signal suggest complexity
        if lower.len() > 500 && weak_count >= 1 {
            return true;
        }

        false
    }

    /// Entry point: routes to orchestrated or flat mode based on prompt complexity.
    pub async fn run_turn(
        &mut self,
        user_message: &str,
    ) -> Result<String> {
        if self.use_orchestration && Self::should_orchestrate(user_message) {
            self.run_orchestrated(user_message).await
        } else {
            self.run_single_turn(user_message).await
        }
    }

    /// Orchestrated mode: Planner → Executor → Critic loop.
    pub async fn run_orchestrated(
        &mut self,
        user_message: &str,
    ) -> Result<String> {
        // Record the decision to orchestrate in the trace
        if let Ok(mut t) = self.trace.lock() {
            t.record(
                "agent",
                TraceEventType::Decision { description: "routing to orchestrator".into() },
                user_message,
            );
        }

        let mut orchestrator = Orchestrator::new_with_telemetry(
            self.router.clone(),
            self.allow_write,
            self.allow_exec,
            self.max_tool_rounds,
            self.project_dir.as_deref(),
            self.telemetry.clone(),
            self.trace.clone(),
        );
        orchestrator.set_event_bus(self.event_bus.clone());
        let result = orchestrator.run_task(user_message).await?;
        // Merge orchestrator budget into our own
        let ob = orchestrator.budget();
        // No direct merge API — track separately via Done event token count
        let _ = ob; // budget reported through Done event
        Ok(result)
    }

    /// Flat single-agent mode (original behaviour). Used for simple questions.
    pub async fn run_single_turn(
        &mut self,
        user_message: &str,
    ) -> Result<String> {
        // Update system prompt: context retrieval + active learned rules
        let flags = self.telemetry.flags();
        let context_prompt = build_system_prompt_with_context_opts(
            self.project_dir.as_deref(),
            self.default_tier,
            &self.tools.tool_names(),
            user_message,
            flags.knowledge_graph,
            flags.callbacks,
        );
        let final_prompt = if let Some(ref engine) = self.pref_engine {
            let rules = engine.get_active_rules();
            if rules.is_empty() {
                context_prompt
            } else {
                // Append the learned preferences section on top of the context prompt
                let mut p = context_prompt;
                p.push_str("\n\n## LEARNED PREFERENCES\n");
                for rule in &rules {
                    p.push_str(&format!(
                        "- {} (confidence: {:.0}%)\n",
                        rule.description,
                        rule.confidence * 100.0
                    ));
                }
                p
            }
        } else {
            context_prompt
        };
        self.conversation.update_system(final_prompt);

        // Context compaction — config-driven thresholds
        let est_tokens = self.conversation.estimated_tokens();
        let msg_count = self.conversation.message_count();
        if (est_tokens > self.compaction.token_threshold
            || msg_count > self.compaction.message_threshold)
            && msg_count > self.compaction.retention_window
        {
            let summary = format!(
                "Previous conversation covered {} messages ({} estimated tokens). The user has been working on a coding task.",
                msg_count,
                est_tokens
            );
            self.conversation.compact(&summary, self.compaction.retention_window);
            self.telemetry.compaction_triggered(est_tokens as u64, 0);
        }

        self.conversation.add_user(user_message);
        self.event_bus.publish(AgentEvent::Thinking);

        let tool_schemas = self.tools.tool_schemas();
        let mut tool_round = 0;

        loop {
            let messages = self.conversation.to_messages();

            // Use native tool calling if tier supports it and tools are available
            let start = std::time::Instant::now();
            let response = if self.default_tier.supports_tools() && !tool_schemas.is_empty() {
                self.router
                    .chat_with_tools(self.default_tier, messages, tool_schemas.clone())
                    .await?
            } else {
                self.router.chat(self.default_tier, messages).await?
            };
            let duration_ns = start.elapsed().as_nanos() as u64;

            // Track budget + telemetry
            let prompt_tokens = response.prompt_tokens();
            let completion_tokens = response.completion_tokens();
            self.budget.record_turn(prompt_tokens, completion_tokens, duration_ns);
            self.telemetry.model_inference(
                self.default_tier.display_name(),
                prompt_tokens,
                completion_tokens,
                duration_ns / 1_000_000,
            );

            // Trace: record model response
            if let Ok(mut t) = self.trace.lock() {
                t.record(
                    "agent",
                    TraceEventType::Decision {
                        description: format!(
                            "model_response tier={} tokens={}",
                            self.default_tier.display_name(),
                            prompt_tokens + completion_tokens
                        ),
                    },
                    "",
                );
            }

            let content = &response.message().content.clone();
            let display_content = strip_thinking(content);

            // Check for native tool calls
            if response.message().has_tool_calls() {
                if tool_round >= self.max_tool_rounds {
                    self.conversation.add_assistant(content);
                    self.event_bus.publish(AgentEvent::Response(
                        "Reached maximum tool rounds. Stopping.".into(),
                    ));
                    return Ok(display_content);
                }

                let tool_calls = response.message().tool_calls.as_ref().unwrap().clone();
                let tool_calls = &tool_calls;

                // Add the assistant message with tool calls to conversation
                self.conversation.add_assistant_with_tool_calls(
                    content,
                    tool_calls.clone(),
                );

                // Execute each tool call
                for tc in tool_calls {
                    let name = &tc.function.name;
                    let args = &tc.function.arguments;

                    // Check budget before each tool call
                    if let Some(budget_event) = self.check_budget() {
                        self.event_bus.publish(budget_event);
                        self.conversation.add_assistant(content);
                        return Ok(display_content);
                    }

                    // Check if confirmation is needed
                    if self.needs_confirmation(name, args) {
                        self.event_bus.publish(AgentEvent::ToolExec {
                            name: name.clone(),
                            status: "PENDING".into(),
                            args: args.clone(),
                            result: None,
                        });
                        if !self.request_confirmation(name, args).await {
                            self.event_bus.publish(AgentEvent::ToolExec {
                                name: name.clone(),
                                status: "SKIPPED".into(),
                                args: args.clone(),
                                result: Some("User declined".into()),
                            });
                            self.conversation.add_tool_result("Tool call skipped by user.");
                            continue;
                        }
                    }

                    self.event_bus.publish(AgentEvent::ToolExec {
                        name: name.clone(),
                        status: "EXECUTING".into(),
                        args: args.clone(),
                        result: None,
                    });

                    // Trace: record tool call
                    if let Ok(mut t) = self.trace.lock() {
                        let args_summary = args.to_string();
                        let safe_end = args_summary.len().min(120);
                        t.record(
                            "agent",
                            TraceEventType::ToolCall {
                                tool: name.clone(),
                                args_summary: args_summary[..safe_end].to_string(),
                            },
                            "",
                        );
                    }

                    let result_text = match self.tools.execute(name, args.clone()) {
                        Ok(output) => output,
                        Err(e) => format!("Tool error: {}", e),
                    };

                    let is_error = result_text.starts_with("Tool error:");
                    self.telemetry.tool_call(name, !is_error);

                    // Trace: record tool result
                    if let Ok(mut t) = self.trace.lock() {
                        t.record(
                            "agent",
                            TraceEventType::ToolResult { tool: name.clone(), success: !is_error },
                            "",
                        );
                        if is_error {
                            let safe_end = result_text.len().min(200);
                            t.record(
                                "agent",
                                TraceEventType::Error { message: result_text[..safe_end].to_string() },
                                "",
                            );
                        }
                    }

                    self.event_bus.publish(AgentEvent::ToolExec {
                        name: name.clone(),
                        status: "DONE".into(),
                        args: args.clone(),
                        result: Some(result_text.clone()),
                    });

                    // Auto-lint after writes
                    if (name == "write_file" || name == "edit_file") && !is_error {
                        // Trace: file change
                        if let Ok(mut t) = self.trace.lock() {
                            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                            let action = if name == "write_file" { "modify" } else { "modify" };
                            t.record(
                                "agent",
                                TraceEventType::FileChange {
                                    path: path.to_string(),
                                    action: action.to_string(),
                                },
                                "",
                            );
                        }
                        if self.telemetry.flags().auto_lint {
                            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                            if let Some(lint_output) = crate::linter::lint_file(path) {
                                self.telemetry.lint_triggered();
                                self.event_bus.publish(AgentEvent::ToolExec {
                                    name: "lint".into(),
                                    status: "WARN".into(),
                                    args: serde_json::json!({"file": path}),
                                    result: Some(lint_output),
                                });
                            }
                        }
                    }

                    // Record preference signal: successful executions are ACCEPT
                    if !is_error {
                        if let Some(ref engine) = self.pref_engine {
                            engine.record_signal(
                                name,
                                SignalType::Accept,
                                user_message,
                                self.session_id.as_deref(),
                            );
                        }
                    }

                    // Add tool result to conversation
                    self.conversation.add_tool_result(&result_text);
                }

                tool_round += 1;
                continue;
            }

            // Fallback: check for text-based tool calls in the response content
            // Models like Qwen3-Coder emit <function=name> XML-style or ```json blocks
            if let Some(text_call) = extract_text_tool_call(content) {
                if tool_round >= self.max_tool_rounds {
                    self.conversation.add_assistant(content);
                    self.event_bus.publish(AgentEvent::Response(
                        "Reached maximum tool rounds. Stopping.".into(),
                    ));
                    return Ok(display_content);
                }

                // Check budget
                if let Some(budget_event) = self.check_budget() {
                    self.event_bus.publish(budget_event);
                    self.conversation.add_assistant(content);
                    return Ok(display_content);
                }

                // Check if confirmation is needed
                if self.needs_confirmation(&text_call.name, &text_call.args) {
                    self.event_bus.publish(AgentEvent::ToolExec {
                        name: text_call.name.clone(),
                        status: "PENDING".into(),
                        args: text_call.args.clone(),
                        result: None,
                    });
                    if !self.request_confirmation(&text_call.name, &text_call.args).await {
                        self.event_bus.publish(AgentEvent::ToolExec {
                            name: text_call.name.clone(),
                            status: "SKIPPED".into(),
                            args: text_call.args.clone(),
                            result: Some("User declined".into()),
                        });
                        self.conversation.add_assistant(content);
                        self.conversation.add_tool_result("Tool call skipped by user.");
                        tool_round += 1;
                        continue;
                    }
                }

                self.event_bus.publish(AgentEvent::ToolExec {
                    name: text_call.name.clone(),
                    status: "EXECUTING".into(),
                    args: text_call.args.clone(),
                    result: None,
                });

                let result_text = match self.tools.execute(&text_call.name, text_call.args.clone()) {
                    Ok(output) => output,
                    Err(e) => format!("Tool error: {}", e),
                };

                let tc_name = text_call.name.clone();
                let is_error = result_text.starts_with("Tool error:");
                self.telemetry.tool_call(&tc_name, !is_error);

                // Trace: record text-based tool call + result
                if let Ok(mut t) = self.trace.lock() {
                    let args_summary = text_call.args.to_string();
                    let safe_end = args_summary.len().min(120);
                    t.record(
                        "agent",
                        TraceEventType::ToolCall {
                            tool: tc_name.clone(),
                            args_summary: args_summary[..safe_end].to_string(),
                        },
                        "",
                    );
                    t.record(
                        "agent",
                        TraceEventType::ToolResult { tool: tc_name.clone(), success: !is_error },
                        "",
                    );
                    if is_error {
                        let safe_end = result_text.len().min(200);
                        t.record(
                            "agent",
                            TraceEventType::Error { message: result_text[..safe_end].to_string() },
                            "",
                        );
                    }
                }

                self.event_bus.publish(AgentEvent::ToolExec {
                    name: tc_name.clone(),
                    status: "DONE".into(),
                    args: text_call.args.clone(),
                    result: Some(result_text.clone()),
                });

                // Auto-lint after writes
                if (tc_name == "write_file" || tc_name == "edit_file") && !is_error {
                    if self.telemetry.flags().auto_lint {
                        let path = text_call.args.get("path").and_then(|v| v.as_str()).unwrap_or("");
                        if let Some(lint_output) = crate::linter::lint_file(path) {
                            self.telemetry.lint_triggered();
                            self.event_bus.publish(AgentEvent::ToolExec {
                                name: "lint".into(),
                                status: "WARN".into(),
                                args: serde_json::json!({"file": path}),
                                result: Some(lint_output),
                            });
                        }
                    }
                }

                // Record preference signal
                if !is_error {
                    if let Some(ref engine) = self.pref_engine {
                        engine.record_signal(
                            &tc_name,
                            SignalType::Accept,
                            user_message,
                            self.session_id.as_deref(),
                        );
                    }
                }

                self.conversation.add_assistant(content);
                self.conversation.add_tool_result(&result_text);
                tool_round += 1;
                continue;
            }

            // No tool calls — this is the final response
            self.conversation.add_assistant(content);
            self.event_bus.publish(AgentEvent::Response(display_content.clone()));
            self.event_bus.publish(AgentEvent::Done {
                tokens: self.budget.total_tokens(),
                duration_ms: duration_ns / 1_000_000,
            });

            return Ok(display_content);
        }
    }

    pub fn budget(&self) -> &BudgetTracker {
        &self.budget
    }

    pub fn set_tier(&mut self, tier: ModelTier) {
        self.default_tier = tier;
    }

    pub fn current_tier(&self) -> ModelTier {
        self.default_tier
    }

    pub fn router_clone(&self) -> Arc<ModelRouter> {
        self.router.clone()
    }
}

/// Build a human-readable summary of what a tool call will do.
fn build_tool_summary(name: &str, args: &serde_json::Value) -> String {
    match name {
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            let content_len = args
                .get("content")
                .and_then(|v| v.as_str())
                .map(|s| s.len())
                .unwrap_or(0);
            format!("Write {} ({} bytes)", path, content_len)
        }
        "edit_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Edit {}", path)
        }
        "bash" => {
            let cmd = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            if cmd.len() > 60 {
                let end = {
                    let mut e = 60_usize.min(cmd.len());
                    while e > 0 && !cmd.is_char_boundary(e) { e -= 1; }
                    e
                };
                format!("Run: {}...", &cmd[..end])
            } else {
                format!("Run: {}", cmd)
            }
        }
        "get_url" => {
            let url = args.get("url").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Fetch {}", url)
        }
        _ => format!("{} {:?}", name, args),
    }
}

