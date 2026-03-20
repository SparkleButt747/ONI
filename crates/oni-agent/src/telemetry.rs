//! Deep Telemetry — per-call instrumentation proving each feature's contribution.
//!
//! Every feature in ONI emits telemetry events. At session end, the telemetry
//! is serialized to JSON for analysis. This answers: "did this feature help,
//! hurt, or make no difference?"
//!
//! Categories:
//!   ORCHESTRATOR — routing, planning, critic verdicts
//!   TOOL — tool calls, success/fail, which tools used
//!   CONTEXT — knowledge graph, FTS5, callbacks, .oni-context
//!   PERSONALITY — emotional state, relationship, SOUL.md effects
//!   COMPACTION — when triggered, what was removed
//!   AUTONOMY — confirmations, user decisions
//!   MODEL — token counts, inference time, tier used

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Feature flags — each can be disabled for A/B testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub knowledge_graph: bool,
    pub reflection: bool,
    pub personality: bool,
    pub callbacks: bool,
    pub compaction: bool,
    pub multi_trajectory: bool,
    pub orchestrator: bool,
    pub auto_lint: bool,
    pub emotional_state: bool,
    pub forge_tool: bool,
    pub undo_tracking: bool,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            knowledge_graph: true,
            reflection: true,
            personality: true,
            callbacks: true,
            compaction: true,
            multi_trajectory: true,
            orchestrator: true,
            auto_lint: true,
            emotional_state: true,
            forge_tool: true,
            undo_tracking: true,
        }
    }
}

/// Classification of why a task succeeded or failed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CapabilityFlag {
    /// Everything worked correctly.
    CleanPass,
    /// Model fundamentally can't solve this (e.g., cryptography at 30B).
    ModelLimit,
    /// Model could solve it but framework routed/presented wrong.
    FrameworkLimit,
    /// Model was on track but hit max_rounds or time limit.
    TimeoutLimit,
    /// Test verification was wrong (false negative).
    HarnessIssue,
    /// Unknown — needs manual inspection.
    Unknown,
}

impl std::fmt::Display for CapabilityFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CleanPass => write!(f, "CLEAN_PASS"),
            Self::ModelLimit => write!(f, "MODEL_LIMIT"),
            Self::FrameworkLimit => write!(f, "FRAMEWORK_LIMIT"),
            Self::TimeoutLimit => write!(f, "TIMEOUT_LIMIT"),
            Self::HarnessIssue => write!(f, "HARNESS_ISSUE"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// A single telemetry event with timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryEvent {
    /// Milliseconds since session start.
    pub timestamp_ms: u64,
    /// Which layer generated this event.
    pub layer: TelemetryLayer,
    /// Event name (e.g., "tool_call", "compaction_triggered").
    pub event: String,
    /// Structured details.
    pub data: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TelemetryLayer {
    Orchestrator,
    Tool,
    Context,
    Personality,
    Compaction,
    Autonomy,
    Model,
}

impl std::fmt::Display for TelemetryLayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Orchestrator => write!(f, "ORCH"),
            Self::Tool => write!(f, "TOOL"),
            Self::Context => write!(f, "CTX"),
            Self::Personality => write!(f, "PERS"),
            Self::Compaction => write!(f, "COMPACT"),
            Self::Autonomy => write!(f, "AUTO"),
            Self::Model => write!(f, "MODEL"),
        }
    }
}

/// Session telemetry accumulator. Thread-safe for use across async tasks.
#[derive(Clone)]
pub struct Telemetry {
    inner: Arc<Mutex<TelemetryInner>>,
}

struct TelemetryInner {
    enabled: bool,
    start: Instant,
    events: Vec<TelemetryEvent>,
    features_used: HashMap<String, u32>,
    feature_flags: FeatureFlags,
    /// Counters for quick summary.
    tool_calls: u32,
    tool_successes: u32,
    tool_failures: u32,
    orchestrator_plans: u32,
    critic_accepts: u32,
    critic_rejects: u32,
    replans: u32,
    trajectories_tried: u32,
    compaction_triggers: u32,
    context_injections: u32,
    kg_nodes_injected: u32,
    callbacks_fired: u32,
    confirmations_shown: u32,
    confirmations_approved: u32,
    lints_triggered: u32,
    forge_tools_created: u32,
    total_prompt_tokens: u64,
    total_completion_tokens: u64,
    total_inference_ms: u64,
}

impl Telemetry {
    pub fn new(enabled: bool, flags: FeatureFlags) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TelemetryInner {
                enabled,
                start: Instant::now(),
                events: Vec::new(),
                features_used: HashMap::new(),
                feature_flags: flags,
                tool_calls: 0,
                tool_successes: 0,
                tool_failures: 0,
                orchestrator_plans: 0,
                critic_accepts: 0,
                critic_rejects: 0,
                replans: 0,
                trajectories_tried: 0,
                compaction_triggers: 0,
                context_injections: 0,
                kg_nodes_injected: 0,
                callbacks_fired: 0,
                confirmations_shown: 0,
                confirmations_approved: 0,
                lints_triggered: 0,
                forge_tools_created: 0,
                total_prompt_tokens: 0,
                total_completion_tokens: 0,
                total_inference_ms: 0,
            })),
        }
    }

    pub fn disabled() -> Self {
        Self::new(false, FeatureFlags::default())
    }

    pub fn flags(&self) -> FeatureFlags {
        self.inner.lock().unwrap().feature_flags.clone()
    }

    /// Record a telemetry event.
    pub fn record(&self, layer: TelemetryLayer, event: &str, data: HashMap<String, serde_json::Value>) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.enabled {
            return;
        }
        let ts = inner.start.elapsed().as_millis() as u64;
        inner.events.push(TelemetryEvent {
            timestamp_ms: ts,
            layer,
            event: event.to_string(),
            data,
        });
    }

    /// Record a feature being used (for activation counting).
    pub fn feature_used(&self, feature: &str) {
        let mut inner = self.inner.lock().unwrap();
        *inner.features_used.entry(feature.to_string()).or_insert(0) += 1;
    }

    // ── Convenience methods for common events ───────────────────────────

    pub fn tool_call(&self, name: &str, success: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.tool_calls += 1;
        if success {
            inner.tool_successes += 1;
        } else {
            inner.tool_failures += 1;
        }
        if inner.enabled {
            let ts = inner.start.elapsed().as_millis() as u64;
            inner.events.push(TelemetryEvent {
                timestamp_ms: ts,
                layer: TelemetryLayer::Tool,
                event: "tool_call".into(),
                data: [
                    ("tool".to_string(), serde_json::json!(name)),
                    ("success".to_string(), serde_json::json!(success)),
                ].into(),
            });
        }
    }

    pub fn orchestrator_plan(&self, steps: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.orchestrator_plans += 1;
        inner.features_used.entry("orchestrator".to_string()).and_modify(|c| *c += 1).or_insert(1);
        if inner.enabled {
            let ts = inner.start.elapsed().as_millis() as u64;
            inner.events.push(TelemetryEvent {
                timestamp_ms: ts,
                layer: TelemetryLayer::Orchestrator,
                event: "plan_generated".into(),
                data: [
                    ("steps".to_string(), serde_json::json!(steps)),
                ].into(),
            });
        }
    }

    pub fn critic_verdict(&self, accepted: bool) {
        let mut inner = self.inner.lock().unwrap();
        if accepted {
            inner.critic_accepts += 1;
        } else {
            inner.critic_rejects += 1;
        }
    }

    pub fn replan(&self) {
        self.inner.lock().unwrap().replans += 1;
    }

    pub fn trajectory(&self) {
        self.inner.lock().unwrap().trajectories_tried += 1;
        self.feature_used("multi_trajectory");
    }

    pub fn compaction_triggered(&self, tokens_before: u64, tokens_after: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.compaction_triggers += 1;
        inner.features_used.entry("compaction".to_string()).and_modify(|c| *c += 1).or_insert(1);
        if inner.enabled {
            let ts = inner.start.elapsed().as_millis() as u64;
            inner.events.push(TelemetryEvent {
                timestamp_ms: ts,
                layer: TelemetryLayer::Compaction,
                event: "compaction_triggered".into(),
                data: [
                    ("tokens_before".to_string(), serde_json::json!(tokens_before)),
                    ("tokens_after".to_string(), serde_json::json!(tokens_after)),
                ].into(),
            });
        }
    }

    pub fn context_injection(&self, source: &str, items: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.context_injections += 1;
        if source == "knowledge_graph" {
            inner.kg_nodes_injected += items as u32;
            inner.features_used.entry("knowledge_graph".to_string()).and_modify(|c| *c += 1).or_insert(1);
        }
    }

    pub fn callback_fired(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.callbacks_fired += 1;
        inner.features_used.entry("callbacks".to_string()).and_modify(|c| *c += 1).or_insert(1);
    }

    pub fn confirmation(&self, approved: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.confirmations_shown += 1;
        if approved {
            inner.confirmations_approved += 1;
        }
    }

    pub fn lint_triggered(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.lints_triggered += 1;
        inner.features_used.entry("auto_lint".to_string()).and_modify(|c| *c += 1).or_insert(1);
    }

    pub fn forge_tool_created(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.forge_tools_created += 1;
        inner.features_used.entry("forge_tool".to_string()).and_modify(|c| *c += 1).or_insert(1);
    }

    pub fn model_inference(&self, tier: &str, prompt_tokens: u64, completion_tokens: u64, duration_ms: u64) {
        let mut inner = self.inner.lock().unwrap();
        inner.total_prompt_tokens += prompt_tokens;
        inner.total_completion_tokens += completion_tokens;
        inner.total_inference_ms += duration_ms;
        if inner.enabled {
            let ts = inner.start.elapsed().as_millis() as u64;
            inner.events.push(TelemetryEvent {
                timestamp_ms: ts,
                layer: TelemetryLayer::Model,
                event: "inference".into(),
                data: [
                    ("tier".to_string(), serde_json::json!(tier)),
                    ("prompt_tokens".to_string(), serde_json::json!(prompt_tokens)),
                    ("completion_tokens".to_string(), serde_json::json!(completion_tokens)),
                    ("duration_ms".to_string(), serde_json::json!(duration_ms)),
                ].into(),
            });
        }
    }

    pub fn personality_effect(&self, modifier: &str) {
        self.feature_used("personality");
        self.record(TelemetryLayer::Personality, "modifier_active", [
            ("modifier".to_string(), serde_json::json!(modifier)),
        ].into());
    }

    pub fn emotional_state_delta(&self, emotion: &str, before: f64, after: f64) {
        self.feature_used("emotional_state");
        self.record(TelemetryLayer::Personality, "emotion_change", [
            ("emotion".to_string(), serde_json::json!(emotion)),
            ("before".to_string(), serde_json::json!(before)),
            ("after".to_string(), serde_json::json!(after)),
        ].into());
    }

    // ── Export ───────────────────────────────────────────────────────────

    /// Generate the full telemetry report as JSON.
    pub fn to_json(&self) -> serde_json::Value {
        let inner = self.inner.lock().unwrap();
        serde_json::json!({
            "enabled": inner.enabled,
            "duration_ms": inner.start.elapsed().as_millis() as u64,
            "feature_flags": inner.feature_flags,
            "features_activated": inner.features_used,
            "summary": {
                "tool_calls": inner.tool_calls,
                "tool_successes": inner.tool_successes,
                "tool_failures": inner.tool_failures,
                "orchestrator_plans": inner.orchestrator_plans,
                "critic_accepts": inner.critic_accepts,
                "critic_rejects": inner.critic_rejects,
                "replans": inner.replans,
                "trajectories_tried": inner.trajectories_tried,
                "compaction_triggers": inner.compaction_triggers,
                "context_injections": inner.context_injections,
                "kg_nodes_injected": inner.kg_nodes_injected,
                "callbacks_fired": inner.callbacks_fired,
                "confirmations_shown": inner.confirmations_shown,
                "confirmations_approved": inner.confirmations_approved,
                "lints_triggered": inner.lints_triggered,
                "forge_tools_created": inner.forge_tools_created,
                "total_prompt_tokens": inner.total_prompt_tokens,
                "total_completion_tokens": inner.total_completion_tokens,
                "total_inference_ms": inner.total_inference_ms,
            },
            "events": if inner.enabled { serde_json::json!(inner.events) } else { serde_json::json!([]) },
        })
    }

    /// Save telemetry JSON to a file.
    pub fn save_to_file(&self, path: &std::path::Path) {
        let json = self.to_json();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, serde_json::to_string_pretty(&json).unwrap_or_default());
    }

    /// Quick summary string for display.
    pub fn summary_string(&self) -> String {
        let inner = self.inner.lock().unwrap();
        let features: Vec<String> = inner.features_used.iter()
            .map(|(k, v)| format!("{}({})", k, v))
            .collect();
        format!(
            "Tools: {}/{} ok | Orch: {} plans, {}/{} accept | Features: [{}] | Tokens: {}p/{}c in {}ms",
            inner.tool_successes,
            inner.tool_calls,
            inner.orchestrator_plans,
            inner.critic_accepts,
            inner.critic_accepts + inner.critic_rejects,
            features.join(", "),
            inner.total_prompt_tokens,
            inner.total_completion_tokens,
            inner.total_inference_ms,
        )
    }
}
