use crate::agent::AgentEvent;
use crate::budget::BudgetTracker;
use crate::conversation::Conversation;
use crate::message_bus::MessageBus;
use crate::parsing::{extract_text_tool_call, strip_thinking};
use crate::plan_store::PersistedPlan;
use crate::prompts;
use crate::telemetry::Telemetry;
use crate::tools::ToolRegistry;
use crate::trace::{ExecutionTrace, TraceEventType};
use oni_core::error::Result;
use oni_core::types::ModelTier;
use oni_llm::ModelRouter;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub struct Orchestrator {
    router: Arc<ModelRouter>,
    tools: ToolRegistry,
    budget: BudgetTracker,
    max_tool_rounds: usize,
    max_replan_cycles: usize,
    /// Maximum alternative trajectories to try per rejected step.
    max_trajectories: usize,
    project_dir: Option<String>,
    /// Shared telemetry accumulator.
    telemetry: Telemetry,
    /// Shared execution trace — written by orchestrator, readable by TUI.
    trace: Arc<Mutex<ExecutionTrace>>,
    /// Pub/sub event bus — shared with the Agent and TUI/headless consumer.
    event_bus: MessageBus<AgentEvent>,
}

impl Orchestrator {
    pub fn new(
        router: Arc<ModelRouter>,
        allow_write: bool,
        allow_exec: bool,
        max_tool_rounds: usize,
        project_dir: Option<&str>,
    ) -> Self {
        Self::new_with_telemetry(
            router,
            allow_write,
            allow_exec,
            max_tool_rounds,
            project_dir,
            Telemetry::disabled(),
            Arc::new(Mutex::new(ExecutionTrace::new(500))),
        )
    }

    pub fn new_with_telemetry(
        router: Arc<ModelRouter>,
        allow_write: bool,
        allow_exec: bool,
        max_tool_rounds: usize,
        project_dir: Option<&str>,
        telemetry: Telemetry,
        trace: Arc<Mutex<ExecutionTrace>>,
    ) -> Self {
        Self {
            router,
            tools: ToolRegistry::new(allow_write, allow_exec),
            budget: BudgetTracker::new(),
            max_tool_rounds,
            max_replan_cycles: 2,
            max_trajectories: 2,
            project_dir: project_dir.map(|s| s.to_string()),
            telemetry,
            trace,
            event_bus: MessageBus::new(500),
        }
    }

    /// Replace the event bus with an externally-provided one (shared with Agent).
    pub fn set_event_bus(&mut self, bus: MessageBus<AgentEvent>) {
        self.event_bus = bus;
    }

    /// Full Planner → Executor → Critic orchestration loop.
    pub async fn run_task(
        &mut self,
        user_prompt: &str,
    ) -> Result<String> {
        let started = Instant::now();

        self.event_bus.publish(AgentEvent::Thinking);

        let mut steps = self.plan(user_prompt).await?;
        self.telemetry.orchestrator_plan(steps.len());
        self.event_bus.publish(AgentEvent::PlanGenerated { steps: steps.clone() });

        // Trace: plan created
        if let Ok(mut t) = self.trace.lock() {
            t.record(
                "mimir",
                TraceEventType::PlanStep { step: 0, total: steps.len() },
                &format!("plan created: {} steps", steps.len()),
            );
        }

        // Persist the plan so it survives across sessions
        let project_dir = self.project_dir.clone().unwrap_or_else(|| ".".to_string());
        let mut persisted = PersistedPlan::new(user_prompt, steps.clone(), &project_dir);
        persisted.save();

        let mut replan_cycle = 0;
        let mut start_from = 0;

        'replan: loop {
            let total = steps.len();

            for (idx, step) in steps.iter().enumerate().skip(start_from) {
                let step_num = idx + 1;
                self.event_bus.publish(AgentEvent::ExecutorStep {
                    step: step_num,
                    total,
                    description: step.clone(),
                });

                // Trace: step execution start
                if let Ok(mut t) = self.trace.lock() {
                    t.record(
                        "fenrir",
                        TraceEventType::PlanStep { step: step_num, total },
                        step,
                    );
                }

                persisted.start_step(step_num);

                let step_result = self
                    .execute_step(user_prompt, &steps, idx)
                    .await?;

                self.event_bus.publish(AgentEvent::Thinking);
                let verdict = self
                    .critique(user_prompt, &steps, idx, &step_result)
                    .await?;

                match verdict {
                    CriticVerdict::Accept => {
                        self.telemetry.critic_verdict(true);
                        // Trace: critic accept
                        if let Ok(mut t) = self.trace.lock() {
                            t.record(
                                "skuld",
                                TraceEventType::CriticVerdict { accepted: true },
                                &format!("step {} accepted", step_num),
                            );
                        }
                        self.event_bus.publish(AgentEvent::CriticVerdict {
                            accepted: true,
                            reason: String::new(),
                        });
                        persisted.complete_step(step_num);
                    }
                    CriticVerdict::Reject(reason) => {
                        self.telemetry.critic_verdict(false);
                        // Trace: critic reject
                        if let Ok(mut t) = self.trace.lock() {
                            t.record(
                                "skuld",
                                TraceEventType::CriticVerdict { accepted: false },
                                &format!("step {} rejected: {}", step_num, reason),
                            );
                        }
                        // Multi-trajectory: try an alternative approach before replanning
                        let mut recovered = false;
                        if self.max_trajectories > 1 {
                            self.telemetry.trajectory();
                            self.event_bus.publish(AgentEvent::CriticVerdict {
                                accepted: false,
                                reason: format!("{}. Trying alternative trajectory...", reason),
                            });

                            // Execute the same step again — the model may take a different approach
                            let alt_result = self
                                .execute_step(user_prompt, &steps, idx)
                                .await?;
                            let alt_verdict = self
                                .critique(user_prompt, &steps, idx, &alt_result)
                                .await?;

                            if matches!(alt_verdict, CriticVerdict::Accept) {
                                self.event_bus.publish(AgentEvent::CriticVerdict {
                                    accepted: true,
                                    reason: "Accepted on alternative trajectory".into(),
                                });
                                persisted.complete_step(step_num);
                                recovered = true;
                            }
                        }

                        if !recovered {
                            self.event_bus.publish(AgentEvent::CriticVerdict {
                                accepted: false,
                                reason: reason.clone(),
                            });

                            if replan_cycle >= self.max_replan_cycles {
                                persisted.complete_step(step_num);
                                break;
                            }

                            replan_cycle += 1;
                            self.telemetry.replan();
                            // Trace: replan event
                            if let Ok(mut t) = self.trace.lock() {
                                t.record(
                                    "mimir",
                                    TraceEventType::Decision {
                                        description: format!("replan cycle {}", replan_cycle),
                                    },
                                    &reason,
                                );
                            }
                            self.event_bus.publish(AgentEvent::Replanning {
                                cycle: replan_cycle,
                                reason: reason.clone(),
                            });

                            steps = self
                                .replan(user_prompt, &steps, idx, &reason)
                                .await?;
                            self.event_bus.publish(AgentEvent::PlanGenerated { steps: steps.clone() });

                            persisted = PersistedPlan::new(user_prompt, steps.clone(), &project_dir);
                            persisted.save();

                            start_from = idx; // Resume from the failed step
                            continue 'replan;
                        }
                    }
                }
            }

            // All steps completed
            break;
        }

        // Final summary from executor
        self.event_bus.publish(AgentEvent::Thinking);
        let summary = self.summarise(user_prompt, &steps).await?;

        // Plan is complete — clear it from disk
        PersistedPlan::clear(&project_dir);

        self.event_bus.publish(AgentEvent::Response(summary.clone()));
        self.event_bus.publish(AgentEvent::Done {
            tokens: self.budget.total_tokens(),
            duration_ms: started.elapsed().as_millis() as u64,
        });

        Ok(summary)
    }

    /// Ask Planner to decompose the task into steps.
    async fn plan(&mut self, prompt: &str) -> Result<Vec<String>> {
        let system = prompts::PLANNER.to_string();
        let context = self.project_context();
        let full_system = if context.is_empty() {
            system
        } else {
            format!("{}\n\n{}", system, context)
        };

        let mut conv = Conversation::new(full_system);
        conv.add_user(prompt);

        let start = std::time::Instant::now();
        let response = self
            .router
            .chat(ModelTier::Heavy, conv.to_messages())
            .await?;
        let dur = start.elapsed().as_nanos() as u64;

        let pt = response.prompt_tokens();
        let ct = response.completion_tokens();
        self.budget.record_turn(pt, ct, dur);
        self.telemetry.model_inference("MIMIR", pt, ct, dur / 1_000_000);

        let content = strip_thinking(&response.message().content);
        Ok(parse_steps(&content))
    }

    /// Ask Planner to revise after a rejection, incorporating the critic's reason.
    async fn replan(
        &mut self,
        original_prompt: &str,
        current_steps: &[String],
        failed_step_idx: usize,
        rejection_reason: &str,
    ) -> Result<Vec<String>> {
        let system = prompts::PLANNER.to_string();
        let steps_text = format_steps(current_steps);
        let user_msg = format!(
            "Original task: {}\n\nPrevious plan:\n{}\n\nStep {} was rejected by the critic: {}\n\n\
             Revise the plan to fix this. Output the full revised numbered list only.",
            original_prompt,
            steps_text,
            failed_step_idx + 1,
            rejection_reason
        );

        let mut conv = Conversation::new(system);
        conv.add_user(&user_msg);

        let start = std::time::Instant::now();
        let response = self
            .router
            .chat(ModelTier::Heavy, conv.to_messages())
            .await?;
        let dur = start.elapsed().as_nanos() as u64;

        let pt = response.prompt_tokens();
        let ct = response.completion_tokens();
        self.budget.record_turn(pt, ct, dur);
        self.telemetry.model_inference("MIMIR:REPLAN", pt, ct, dur / 1_000_000);

        let content = strip_thinking(&response.message().content);
        Ok(parse_steps(&content))
    }

    /// Execute a single step using the Executor (Code tier) with tools.
    async fn execute_step(
        &mut self,
        original_prompt: &str,
        steps: &[String],
        step_idx: usize,
    ) -> Result<String> {
        let step = &steps[step_idx];
        let steps_text = format_steps(steps);
        let context = self.project_context();

        let system = if context.is_empty() {
            prompts::EXECUTOR.to_string()
        } else {
            format!("{}\n\n{}", prompts::EXECUTOR, context)
        };

        let user_msg = format!(
            "Original task: {}\n\nFull plan:\n{}\n\nNow execute step {}: {}",
            original_prompt,
            steps_text,
            step_idx + 1,
            step
        );

        let mut conv = Conversation::new(system);
        conv.add_user(&user_msg);

        let tool_schemas = self.tools.tool_schemas();
        let mut tool_round = 0;
        let mut last_content: String;

        loop {
            let messages = conv.to_messages();

            let start = std::time::Instant::now();
            let response = if ModelTier::Medium.supports_tools() && !tool_schemas.is_empty() {
                self.router
                    .chat_with_tools(ModelTier::Medium, messages, tool_schemas.clone())
                    .await?
            } else {
                self.router.chat(ModelTier::Medium, messages).await?
            };
            let dur = start.elapsed().as_nanos() as u64;

            let pt = response.prompt_tokens();
            let ct = response.completion_tokens();
            self.budget.record_turn(pt, ct, dur);
            self.telemetry.model_inference("FENRIR", pt, ct, dur / 1_000_000);

            let content = response.message().content.clone();
            let content = &content;
            last_content = strip_thinking(content);

            // Native tool calls
            if response.message().has_tool_calls() {
                if tool_round >= self.max_tool_rounds {
                    conv.add_assistant(content);
                    break;
                }

                let tool_calls = response.message().tool_calls.as_ref().unwrap().clone();
                let tool_calls = &tool_calls;
                conv.add_assistant_with_tool_calls(content, tool_calls.clone());

                for tc in tool_calls {
                    let name = &tc.function.name;
                    let args = &tc.function.arguments;

                    self.event_bus.publish(AgentEvent::ToolExec {
                        name: name.clone(),
                        status: "EXECUTING".into(),
                        args: args.clone(),
                        result: None,
                    });

                    // Trace: tool call
                    if let Ok(mut t) = self.trace.lock() {
                        let args_str = args.to_string();
                        let safe_end = args_str.len().min(120);
                        t.record(
                            "fenrir",
                            TraceEventType::ToolCall {
                                tool: name.clone(),
                                args_summary: args_str[..safe_end].to_string(),
                            },
                            "",
                        );
                    }

                    let result_text = match self.tools.execute(name, args.clone()) {
                        Ok(out) => out,
                        Err(e) => format!("Tool error: {}", e),
                    };

                    let is_err = result_text.starts_with("Tool error:");
                    self.telemetry.tool_call(name, !is_err);

                    // Trace: tool result
                    if let Ok(mut t) = self.trace.lock() {
                        t.record(
                            "fenrir",
                            TraceEventType::ToolResult { tool: name.clone(), success: !is_err },
                            "",
                        );
                        if is_err {
                            let safe_end = result_text.len().min(200);
                            t.record(
                                "fenrir",
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

                    conv.add_tool_result(&result_text);
                }

                tool_round += 1;
                continue;
            }

            // Text-based tool calls (Qwen-style / JSON)
            if let Some(text_call) = extract_text_tool_call(content) {
                if tool_round >= self.max_tool_rounds {
                    conv.add_assistant(content);
                    break;
                }

                self.event_bus.publish(AgentEvent::ToolExec {
                    name: text_call.name.clone(),
                    status: "EXECUTING".into(),
                    args: text_call.args.clone(),
                    result: None,
                });

                // Trace: text tool call
                if let Ok(mut t) = self.trace.lock() {
                    let args_str = text_call.args.to_string();
                    let safe_end = args_str.len().min(120);
                    t.record(
                        "fenrir",
                        TraceEventType::ToolCall {
                            tool: text_call.name.clone(),
                            args_summary: args_str[..safe_end].to_string(),
                        },
                        "",
                    );
                }

                let result_text = match self.tools.execute(&text_call.name, text_call.args.clone()) {
                    Ok(out) => out,
                    Err(e) => format!("Tool error: {}", e),
                };

                let is_err = result_text.starts_with("Tool error:");
                self.telemetry.tool_call(&text_call.name, !is_err);

                // Trace: text tool result
                if let Ok(mut t) = self.trace.lock() {
                    t.record(
                        "fenrir",
                        TraceEventType::ToolResult { tool: text_call.name.clone(), success: !is_err },
                        "",
                    );
                }

                self.event_bus.publish(AgentEvent::ToolExec {
                    name: text_call.name.clone(),
                    status: "DONE".into(),
                    args: text_call.args,
                    result: Some(result_text.clone()),
                });

                conv.add_assistant(content);
                conv.add_tool_result(&result_text);
                tool_round += 1;
                continue;
            }

            // No tool calls — step is done
            conv.add_assistant(content);
            break;
        }

        Ok(last_content)
    }

    /// Ask Critic to review the completed step.
    async fn critique(
        &mut self,
        original_prompt: &str,
        steps: &[String],
        step_idx: usize,
        step_result: &str,
    ) -> Result<CriticVerdict> {
        let step = &steps[step_idx];
        let steps_text = format_steps(steps);

        let user_msg = format!(
            "Original task: {}\n\nFull plan:\n{}\n\nStep {} to execute: {}\n\nExecutor output:\n{}",
            original_prompt,
            steps_text,
            step_idx + 1,
            step,
            if step_result.len() > 2000 { &step_result[..2000] } else { step_result }
        );

        let mut conv = Conversation::new(prompts::CRITIC.to_string());
        conv.add_user(&user_msg);

        let start = std::time::Instant::now();
        let response = self
            .router
            .chat(ModelTier::General, conv.to_messages())
            .await?;
        let dur = start.elapsed().as_nanos() as u64;

        let pt = response.prompt_tokens();
        let ct = response.completion_tokens();
        self.budget.record_turn(pt, ct, dur);
        self.telemetry.model_inference("SKULD", pt, ct, dur / 1_000_000);

        let content = strip_thinking(&response.message().content).trim().to_string();
        Ok(parse_critic_verdict(&content))
    }

    /// Generate a final summary from the executor after all steps complete.
    async fn summarise(&mut self, original_prompt: &str, steps: &[String]) -> Result<String> {
        let steps_text = format_steps(steps);
        let user_msg = format!(
            "Task completed: {}\n\nSteps executed:\n{}\n\n\
             Provide a concise summary of what was done and the outcome.",
            original_prompt, steps_text
        );

        let mut conv = Conversation::new(prompts::EXECUTOR.to_string());
        conv.add_user(&user_msg);

        let start = std::time::Instant::now();
        let response = self
            .router
            .chat(ModelTier::Medium, conv.to_messages())
            .await?;
        let dur = start.elapsed().as_nanos() as u64;

        let pt = response.prompt_tokens();
        let ct = response.completion_tokens();
        self.budget.record_turn(pt, ct, dur);
        self.telemetry.model_inference("FENRIR:SUMMARY", pt, ct, dur / 1_000_000);

        Ok(strip_thinking(&response.message().content))
    }

    fn project_context(&self) -> String {
        match &self.project_dir {
            Some(dir) => format!("Working directory: {}", dir),
            None => String::new(),
        }
    }

    pub fn budget(&self) -> &BudgetTracker {
        &self.budget
    }
}

// ─── Verdict ─────────────────────────────────────────────────────────────────

enum CriticVerdict {
    Accept,
    Reject(String),
}

fn parse_critic_verdict(content: &str) -> CriticVerdict {
    let upper = content.to_uppercase();
    if upper.starts_with("ACCEPT") {
        CriticVerdict::Accept
    } else if let Some(rest) = content.strip_prefix("REJECT: ") {
        CriticVerdict::Reject(rest.trim().to_string())
    } else if upper.contains("REJECT") {
        // Lenient: extract anything after REJECT
        if let Some(pos) = content.to_uppercase().find("REJECT") {
            let after = content[pos + 6..].trim_start_matches(':').trim();
            CriticVerdict::Reject(after.to_string())
        } else {
            CriticVerdict::Accept
        }
    } else {
        // Ambiguous — accept to keep moving
        CriticVerdict::Accept
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Parse "1. step\n2. step\n..." into Vec<String>.
fn parse_steps(text: &str) -> Vec<String> {
    let mut steps = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Match "N. " or "N) " prefix
        if let Some(dot_pos) = trimmed.find(". ") {
            let prefix = &trimmed[..dot_pos];
            if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
                steps.push(trimmed[dot_pos + 2..].trim().to_string());
                continue;
            }
        }
        if let Some(paren_pos) = trimmed.find(") ") {
            let prefix = &trimmed[..paren_pos];
            if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
                steps.push(trimmed[paren_pos + 2..].trim().to_string());
                continue;
            }
        }
        // Non-numbered lines inside a plan — skip silently
    }

    if steps.is_empty() {
        // Fallback: treat each non-empty line as one step
        for line in text.lines() {
            let t = line.trim();
            if !t.is_empty() {
                steps.push(t.to_string());
            }
        }
    }

    steps
}

fn format_steps(steps: &[String]) -> String {
    steps
        .iter()
        .enumerate()
        .map(|(i, s)| format!("{}. {}", i + 1, s))
        .collect::<Vec<_>>()
        .join("\n")
}

