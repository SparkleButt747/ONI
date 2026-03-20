//! Reflection Engine — runs between sessions to identify patterns and propose
//! personality mutations. Uses the Critic agent to review accumulated signals.
//!
//! Flow:
//! 1. Read journal entries from last N days
//! 2. Read preference signals (accept/reject counts per tool)
//! 3. Identify patterns worthy of a personality update
//! 4. Generate a reflection summary for the journal
//! 5. Propose SOUL.md mutations (require user approval)

use oni_core::personality;
use std::path::PathBuf;

/// A proposed mutation to ONI's personality or behaviour.
#[derive(Debug, Clone)]
pub struct PersonalityMutation {
    pub category: String,     // "voice", "opinion", "behaviour"
    pub description: String,  // Human-readable description
    pub soul_addition: String, // Text to append to SOUL.md if approved
}

/// Result of a reflection pass.
#[derive(Debug, Clone)]
pub struct ReflectionResult {
    pub summary: String,
    pub mutations: Vec<PersonalityMutation>,
}

/// Run a reflection pass. Analyses recent signals and journal to identify patterns.
/// Returns proposed mutations. This is a synchronous, non-LLM analysis —
/// it uses heuristics on the accumulated data, not another LLM call.
pub fn reflect(db_path: &PathBuf) -> ReflectionResult {
    let mut mutations = Vec::new();
    let mut observations = Vec::new();

    // Read preference signals from DB
    if let Ok(db) = oni_db::Database::open(db_path) {
        // Count accept/reject per tool
        let tool_stats = count_tool_signals(&db);
        for (tool, accepts, rejects) in &tool_stats {
            let total = accepts + rejects;
            if total < 5 {
                continue; // Not enough data
            }
            let accept_rate = *accepts as f64 / total as f64;

            if accept_rate > 0.9 && total >= 10 {
                observations.push(format!(
                    "Tool '{}' has been accepted {}/{} times — high trust",
                    tool, accepts, total
                ));
                mutations.push(PersonalityMutation {
                    category: "behaviour".into(),
                    description: format!("Auto-approve '{}' tool calls (consistently accepted)", tool),
                    soul_addition: format!(
                        "\n- Always auto-approve {} — user trusts this tool.",
                        tool
                    ),
                });
            }

            if accept_rate < 0.3 && total >= 5 {
                observations.push(format!(
                    "Tool '{}' has been rejected {}/{} times — low trust",
                    tool, rejects, total
                ));
                mutations.push(PersonalityMutation {
                    category: "behaviour".into(),
                    description: format!(
                        "Always confirm before using '{}' (frequently rejected)",
                        tool
                    ),
                    soul_addition: format!(
                        "\n- Always ask before using {} — user has rejected it frequently.",
                        tool
                    ),
                });
            }
        }

        // Check for verbose response rejections (if we track that in future)
        // For now, check learned_rules for patterns
        let rules = get_active_rules(&db);
        for rule in &rules {
            observations.push(format!("Active rule: {}", rule));
        }
    }

    // Read recent journal for context
    let today = personality::read_today_journal();
    if !today.is_empty() {
        observations.push("Today's journal has entries.".into());
    }

    // Check relationship stage
    let rel = personality::RelationshipState::load();
    observations.push(format!(
        "Relationship: {} ({} sessions)",
        rel.stage.display_name(),
        rel.total_sessions
    ));

    // Check emotional state
    let emotions = personality::EmotionalState::load();
    if emotions.frustration > 0.4 {
        observations.push(format!(
            "Frustration elevated: {:.1}",
            emotions.frustration
        ));
    }
    if emotions.confidence < 0.4 {
        observations.push(format!(
            "Confidence low: {:.1}",
            emotions.confidence
        ));
        mutations.push(PersonalityMutation {
            category: "voice".into(),
            description: "Show more reasoning when confidence is low".into(),
            soul_addition: "\n- When uncertain, show your reasoning. Don't guess.".into(),
        });
    }

    // Build summary
    let summary = if observations.is_empty() {
        "No significant patterns detected.".into()
    } else {
        format!(
            "## Reflection\n{}\n\nProposed mutations: {}",
            observations
                .iter()
                .map(|o| format!("- {}", o))
                .collect::<Vec<_>>()
                .join("\n"),
            mutations.len()
        )
    };

    // Write reflection to journal
    if !observations.is_empty() {
        personality::append_journal(&summary);
    }

    ReflectionResult { summary, mutations }
}

/// Apply an approved mutation to SOUL.md.
pub fn apply_mutation(mutation: &PersonalityMutation) {
    let mut soul = personality::read_soul();
    if soul.is_empty() {
        soul = personality::default_soul();
    }
    soul.push_str(&mutation.soul_addition);
    if let Err(e) = personality::write_soul(&soul) {
        tracing::warn!("Failed to apply personality mutation: {}", e);
    }
}

// ── DB helpers ──────────────────────────────────────────────────────────────

fn count_tool_signals(db: &oni_db::Database) -> Vec<(String, u32, u32)> {
    let mut results = Vec::new();
    let sql = "SELECT tool_name, \
               SUM(CASE WHEN signal_type='accept' THEN 1 ELSE 0 END) as accepts, \
               SUM(CASE WHEN signal_type='reject' THEN 1 ELSE 0 END) as rejects \
               FROM preference_signals \
               GROUP BY tool_name";
    if let Ok(mut stmt) = db.conn().prepare(sql) {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, u32>(1)?,
                row.get::<_, u32>(2)?,
            ))
        }) {
            for row in rows.flatten() {
                results.push(row);
            }
        }
    }
    results
}

fn get_active_rules(db: &oni_db::Database) -> Vec<String> {
    let mut rules = Vec::new();
    let sql = "SELECT description FROM learned_rules WHERE active = 1";
    if let Ok(mut stmt) = db.conn().prepare(sql) {
        if let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(0)) {
            for row in rows.flatten() {
                rules.push(row);
            }
        }
    }
    rules
}
