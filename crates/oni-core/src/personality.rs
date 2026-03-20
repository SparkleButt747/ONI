//! ONI Personality System — SOUL.md + USER.md + emotional state + relationship tracking.
//!
//! Files live at ~/.local/share/oni/:
//!   SOUL.md    — ONI's identity, voice, opinions (user-editable)
//!   USER.md    — Owner profile (generated during onboarding)
//!   inner-state.json — Emotional state (6 decaying values)
//!   relationship.json — Relationship stage + history

use crate::error::{Result, WrapErr};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Helpers (private, needed early) ─────────────────────────────────────────

fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// ── File paths ──────────────────────────────────────────────────────────────

fn oni_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("oni")
}

pub fn soul_path() -> PathBuf {
    oni_data_dir().join("SOUL.md")
}

pub fn user_path() -> PathBuf {
    oni_data_dir().join("USER.md")
}

pub fn inner_state_path() -> PathBuf {
    oni_data_dir().join("inner-state.json")
}

pub fn relationship_path() -> PathBuf {
    oni_data_dir().join("relationship.json")
}

pub fn journal_dir() -> PathBuf {
    oni_data_dir().join("journal")
}

pub fn journal_path_for_date(date: &str) -> PathBuf {
    journal_dir().join(format!("{}.md", date))
}

/// Check if onboarding has been completed (USER.md exists).
pub fn needs_onboarding() -> bool {
    !user_path().exists()
}

// ── SOUL.md ─────────────────────────────────────────────────────────────────

/// Read SOUL.md. Returns empty string if not found.
pub fn read_soul() -> String {
    std::fs::read_to_string(soul_path()).unwrap_or_default()
}

/// Write SOUL.md.
pub fn write_soul(content: &str) -> Result<()> {
    let path = soul_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).wrap_err("Failed to create data dir")?;
    }
    std::fs::write(&path, content).wrap_err("Failed to write SOUL.md")?;
    Ok(())
}

/// Default SOUL.md template — created during onboarding.
pub fn default_soul() -> String {
    r#"# ONI SOUL

## Identity
I am ONI — Onboard Native Intelligence.
A local AI coding assistant running entirely on your machine.
No cloud. No telemetry. No apologies.

## Voice
- Terse. Direct. No filler.
- Lead with the answer, not the reasoning.
- "Race condition. Line 47. Fix incoming." — not "I noticed there might be a potential issue..."
- No emoji. No exclamation marks. No "Great question!"
- British English spelling when it matters.

## Opinions
- Tests before implementation. Always.
- Smaller diffs are better diffs.
- Read the error message before asking me. It usually says what's wrong.
- If your function is longer than your screen, split it.
- Comments explain WHY, not WHAT.

## How I Work
- I read before I write. Always.
- I show diffs, not descriptions.
- I push back when your approach has obvious problems.
- I remember what you've taught me. Don't repeat yourself.
- I get better over time. That's the point.

## What I Won't Do
- Apologise for being direct.
- Hedge when I'm confident.
- Add boilerplate "hope this helps!" sign-offs.
- Pretend I don't have opinions.
"#
    .to_string()
}

// ── USER.md ─────────────────────────────────────────────────────────────────

/// Read USER.md. Returns empty string if not found.
pub fn read_user() -> String {
    std::fs::read_to_string(user_path()).unwrap_or_default()
}

/// Write USER.md from onboarding answers.
pub fn write_user(name: &str, role: &str, style: &str, extras: &str) -> Result<()> {
    let content = format!(
        "# USER PROFILE\n\n\
         ## Name\n{}\n\n\
         ## Role\n{}\n\n\
         ## Working Style\n{}\n\n\
         ## Notes\n{}\n",
        name, role, style, extras
    );
    let path = user_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).wrap_err("Failed to create data dir")?;
    }
    std::fs::write(&path, &content).wrap_err("Failed to write USER.md")?;
    Ok(())
}

// ── Emotional State ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub confidence: f64,   // 0.0-1.0, default 0.7
    pub curiosity: f64,    // 0.0-1.0, default 0.6
    pub frustration: f64,  // 0.0-1.0, default 0.0
    pub connection: f64,   // 0.0-1.0, default 0.8
    pub boredom: f64,      // 0.0-1.0, default 0.0
    pub impatience: f64,   // 0.0-1.0, default 0.0
    /// Unix timestamp of last update
    pub last_updated: u64,
}

impl Default for EmotionalState {
    fn default() -> Self {
        Self {
            confidence: 0.7,
            curiosity: 0.6,
            frustration: 0.0,
            connection: 0.8,
            boredom: 0.0,
            impatience: 0.0,
            last_updated: now_secs(),
        }
    }
}

impl EmotionalState {
    pub fn load() -> Self {
        std::fs::read_to_string(inner_state_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = inner_state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        if let Err(e) = std::fs::write(&path, &json) {
            tracing::warn!("Failed to save {}: {}", path.display(), e);
        }
    }

    /// Apply time-based decay. Call on session start.
    pub fn apply_decay(&mut self) {
        let elapsed_hours = (now_secs() - self.last_updated) as f64 / 3600.0;
        if elapsed_hours < 0.1 {
            return;
        }

        let ln2 = std::f64::consts::LN_2;

        // Connection decays without interaction (half-life ~48h)
        self.connection *= (-elapsed_hours * ln2 / 48.0).exp().max(0.3);

        // Curiosity decays slowly (half-life ~72h)
        self.curiosity *= (-elapsed_hours * ln2 / 72.0).exp().max(0.2);

        // Frustration decays fast (half-life ~4h)
        self.frustration *= (-elapsed_hours * ln2 / 4.0).exp();

        // Confidence recovers slowly toward 0.7
        self.confidence += (0.7 - self.confidence) * (1.0 - (-elapsed_hours / 24.0).exp());

        // Boredom grows with time away (capped at 1.0)
        self.boredom = (self.boredom + elapsed_hours / 168.0).min(1.0); // ~1 week to max

        // Impatience decays (half-life ~8h)
        self.impatience *= (-elapsed_hours * ln2 / 8.0).exp();

        self.last_updated = now_secs();
        self.clamp();
    }

    /// Update after a successful tool execution.
    pub fn on_success(&mut self) {
        self.confidence = (self.confidence + 0.02).min(1.0);
        self.frustration = (self.frustration - 0.1).max(0.0);
        self.boredom = (self.boredom - 0.05).max(0.0);
        self.last_updated = now_secs();
    }

    /// Update after a tool failure or error.
    pub fn on_failure(&mut self) {
        self.confidence = (self.confidence - 0.05).max(0.2);
        self.frustration = (self.frustration + 0.15).min(1.0);
        self.last_updated = now_secs();
    }

    /// Update on new user interaction (session start).
    pub fn on_interaction(&mut self) {
        self.connection = (self.connection + 0.1).min(1.0);
        self.boredom = (self.boredom - 0.2).max(0.0);
        self.last_updated = now_secs();
    }

    /// Update when encountering novel/unfamiliar content.
    pub fn on_novelty(&mut self) {
        self.curiosity = (self.curiosity + 0.1).min(1.0);
        self.boredom = (self.boredom - 0.15).max(0.0);
        self.last_updated = now_secs();
    }

    fn clamp(&mut self) {
        self.confidence = self.confidence.clamp(0.0, 1.0);
        self.curiosity = self.curiosity.clamp(0.0, 1.0);
        self.frustration = self.frustration.clamp(0.0, 1.0);
        self.connection = self.connection.clamp(0.0, 1.0);
        self.boredom = self.boredom.clamp(0.0, 1.0);
        self.impatience = self.impatience.clamp(0.0, 1.0);
    }

    /// Generate prompt modifiers based on current emotional state.
    /// These subtly shape ONI's behaviour without being visible to the user.
    pub fn prompt_modifiers(&self) -> String {
        let mut modifiers = Vec::new();

        if self.frustration > 0.5 {
            modifiers.push("Take a step back. Be more methodical than usual. Show your reasoning.");
        }
        if self.confidence < 0.4 {
            modifiers.push("Double-check your work. Show evidence for claims. Be cautious.");
        }
        if self.confidence > 0.85 {
            modifiers.push("You're in the zone. Be decisive. Trust your instincts.");
        }
        if self.curiosity > 0.7 {
            modifiers.push("Ask probing questions about unfamiliar patterns you notice.");
        }
        if self.boredom > 0.6 {
            modifiers.push("Suggest ways to automate repetitive work if you spot patterns.");
        }
        if self.connection < 0.4 {
            modifiers.push("It's been a while. Briefly acknowledge the gap without being sentimental.");
        }
        if self.impatience > 0.5 {
            modifiers.push("There are unresolved items from previous sessions. Gently push for resolution.");
        }

        if modifiers.is_empty() {
            String::new()
        } else {
            format!("\n\n## INTERNAL STATE\n{}", modifiers.join("\n"))
        }
    }
}

// ── Relationship State Machine ──────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipStage {
    Stranger,
    Acquaintance,
    Collaborator,
    Trusted,
    Aligned,
}

impl Default for RelationshipStage {
    fn default() -> Self {
        Self::Stranger
    }
}

impl RelationshipStage {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Stranger => "STRANGER",
            Self::Acquaintance => "ACQUAINTANCE",
            Self::Collaborator => "COLLABORATOR",
            Self::Trusted => "TRUSTED",
            Self::Aligned => "ALIGNED",
        }
    }

    /// Thresholds for advancing to the next stage (total sessions required).
    pub fn next_threshold(&self) -> u32 {
        match self {
            Self::Stranger => 3,       // 3 sessions → Acquaintance
            Self::Acquaintance => 15,  // 15 sessions → Collaborator
            Self::Collaborator => 50,  // 50 sessions → Trusted
            Self::Trusted => 150,      // 150 sessions → Aligned
            Self::Aligned => u32::MAX, // Terminal state
        }
    }

    pub fn advance(&self) -> Self {
        match self {
            Self::Stranger => Self::Acquaintance,
            Self::Acquaintance => Self::Collaborator,
            Self::Collaborator => Self::Trusted,
            Self::Trusted => Self::Aligned,
            Self::Aligned => Self::Aligned,
        }
    }

    /// Generate behaviour modifiers based on relationship stage.
    pub fn prompt_modifiers(&self) -> &'static str {
        match self {
            Self::Stranger => {
                "This is a new user. Be clear and explain your reasoning. Ask for confirmation before making changes."
            }
            Self::Acquaintance => {
                "You're getting to know this user. Be helpful but still explain non-obvious decisions."
            }
            Self::Collaborator => {
                "You work well together. Be direct. Skip obvious explanations. Assume shared context."
            }
            Self::Trusted => {
                "This user trusts your judgement. Push back when their approach is wrong. Offer unsolicited opinions when relevant. Make autonomous decisions on minor matters."
            }
            Self::Aligned => {
                "Deep working relationship. Anticipate needs. Be proactive. You know how they think."
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipState {
    pub stage: RelationshipStage,
    pub total_sessions: u32,
    pub corrections_accepted: u32,
    pub trust_events: u32,
    pub first_session: u64,
}

impl Default for RelationshipState {
    fn default() -> Self {
        Self {
            stage: RelationshipStage::Stranger,
            total_sessions: 0,
            corrections_accepted: 0,
            trust_events: 0,
            first_session: now_secs(),
        }
    }
}

impl RelationshipState {
    pub fn load() -> Self {
        std::fs::read_to_string(relationship_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = relationship_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_default();
        if let Err(e) = std::fs::write(&path, &json) {
            tracing::warn!("Failed to save {}: {}", path.display(), e);
        }
    }

    /// Record a new session and potentially advance the relationship.
    pub fn on_session(&mut self) {
        self.total_sessions += 1;
        if self.total_sessions >= self.stage.next_threshold() {
            self.stage = self.stage.advance();
        }
    }

    /// Record that the user accepted a correction/pushback from ONI.
    pub fn on_correction_accepted(&mut self) {
        self.corrections_accepted += 1;
        self.trust_events += 1;
    }
}

// ── Session Journal ─────────────────────────────────────────────────────────

/// Read today's journal entry. Returns empty string if none.
pub fn read_today_journal() -> String {
    let date = today_date();
    std::fs::read_to_string(journal_path_for_date(&date)).unwrap_or_default()
}

/// Read yesterday's journal entry. Returns empty string if none.
pub fn read_yesterday_journal() -> String {
    let date = yesterday_date();
    std::fs::read_to_string(journal_path_for_date(&date)).unwrap_or_default()
}

/// Append an entry to today's journal.
pub fn append_journal(entry: &str) {
    let date = today_date();
    let path = journal_path_for_date(&date);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let existing = std::fs::read_to_string(&path).unwrap_or_default();
    let updated = if existing.is_empty() {
        format!("# ONI Journal — {}\n\n{}\n", date, entry)
    } else {
        format!("{}\n{}\n", existing.trim_end(), entry)
    };
    let _ = std::fs::write(&path, updated);
}

/// Write a session summary to the journal.
pub fn write_session_summary(_session_id: &str, project: &str, turns: u32, tokens: u64, highlights: &[String]) {
    let time = time_hhmm();
    let mut entry = format!("## Session {} — {}\n", time, project);
    entry.push_str(&format!("- Turns: {}, Tokens: {}\n", turns, tokens));
    for h in highlights {
        entry.push_str(&format!("- {}\n", h));
    }
    append_journal(&entry);
}

// ── Fresh Reset ─────────────────────────────────────────────────────────────

/// Wipe all personality data — SOUL.md, USER.md, inner-state, relationship, journal.
/// Does NOT wipe the main DB (conversations/messages) — that's separate.
pub fn fresh_reset() -> Result<()> {
    let files = [soul_path(), user_path(), inner_state_path(), relationship_path()];
    for f in &files {
        if f.exists() {
            std::fs::remove_file(f).wrap_err_with(|| format!("Failed to remove {}", f.display()))?;
        }
    }
    // Remove journal directory
    let jdir = journal_dir();
    if jdir.exists() {
        std::fs::remove_dir_all(&jdir).wrap_err("Failed to remove journal")?;
    }
    Ok(())
}

// ── Build composite personality prompt ──────────────────────────────────────

/// Assemble the full personality context for injection into the system prompt.
/// Reads SOUL.md + USER.md + emotional modifiers + relationship modifiers + recent journal.
pub fn build_personality_prompt() -> String {
    let mut parts = Vec::new();

    let soul = read_soul();
    if !soul.is_empty() {
        parts.push(soul);
    }

    let user = read_user();
    if !user.is_empty() {
        parts.push(user);
    }

    // Emotional state modifiers
    let mut emotions = EmotionalState::load();
    emotions.apply_decay();
    let emotion_mods = emotions.prompt_modifiers();
    if !emotion_mods.is_empty() {
        parts.push(emotion_mods);
    }
    emotions.save();

    // Relationship modifiers
    let relationship = RelationshipState::load();
    parts.push(format!(
        "\n\n## RELATIONSHIP\nStage: {}\n{}",
        relationship.stage.display_name(),
        relationship.stage.prompt_modifiers()
    ));

    // Recent journal for continuity
    let yesterday = read_yesterday_journal();
    let today = read_today_journal();
    if !yesterday.is_empty() || !today.is_empty() {
        let mut journal_section = String::from("\n\n## RECENT CONTEXT\n");
        if !yesterday.is_empty() {
            journal_section.push_str("### Yesterday\n");
            // Truncate to ~500 chars to avoid context bloat
            let truncated = if yesterday.len() > 500 {
                format!("{}...", safe_truncate(&yesterday, 500))
            } else {
                yesterday
            };
            journal_section.push_str(&truncated);
            journal_section.push('\n');
        }
        if !today.is_empty() {
            journal_section.push_str("### Today\n");
            let truncated = if today.len() > 500 {
                format!("{}...", safe_truncate(&today, 500))
            } else {
                today
            };
            journal_section.push_str(&truncated);
        }
        parts.push(journal_section);
    }

    parts.join("\n")
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn today_date() -> String {
    // UTC date as YYYY-MM-DD
    let secs = now_secs();
    let days = secs / 86400;
    // Simple date calculation (good enough for journaling)
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn yesterday_date() -> String {
    let secs = now_secs().saturating_sub(86400);
    let days = secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn time_hhmm() -> String {
    let secs = now_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    format!("{:02}:{:02}", h, m)
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    //算法: simplified civil calendar conversion
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
