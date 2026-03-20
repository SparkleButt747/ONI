use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq)]
pub enum SignalType {
    Accept,
    Reject,
    Edit,
    Rerun,
}

impl SignalType {
    fn as_str(&self) -> &'static str {
        match self {
            SignalType::Accept => "accept",
            SignalType::Reject => "reject",
            SignalType::Edit => "edit",
            SignalType::Rerun => "rerun",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LearnedRule {
    pub id: i64,
    pub description: String,
    pub context: String,
    pub confidence: f64,
    pub observations: i64,
    pub active: bool,
}

pub struct PreferenceEngine {
    conn: Mutex<rusqlite::Connection>,
}

impl PreferenceEngine {
    pub fn new(db_path: PathBuf) -> Self {
        let conn = rusqlite::Connection::open(&db_path)
            .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap());
        Self { conn: Mutex::new(conn) }
    }

    /// Record a preference signal for a tool execution.
    pub fn record_signal(&self, tool_name: &str, signal: SignalType, context: &str, session_id: Option<&str>) {
        let Ok(conn) = self.conn.lock() else { return };

        // Time-decay: signals older than 7 days already in DB get 0.5x weight via UPDATE — we
        // don't touch them here. New signals always start at weight 1.0.
        let res = conn.execute(
            "INSERT INTO preference_signals (session_id, tool_name, signal_type, context, weight) \
             VALUES (?1, ?2, ?3, ?4, 1.0)",
            rusqlite::params![session_id, tool_name, signal.as_str(), context],
        );
        if let Err(e) = res {
            tracing::warn!("PreferenceEngine: failed to insert signal: {}", e);
        }
    }

    /// Fetch all active rules (confidence >= 0.8) to inject into the system prompt.
    pub fn get_active_rules(&self) -> Vec<LearnedRule> {
        let Ok(conn) = self.conn.lock() else { return Vec::new() };

        let mut stmt = match conn.prepare(
            "SELECT id, description, context, confidence, observations, active \
             FROM learned_rules WHERE active = 1 ORDER BY confidence DESC",
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("PreferenceEngine: prepare failed: {}", e);
                return Vec::new();
            }
        };

        let rows = stmt.query_map([], |row| {
            Ok(LearnedRule {
                id: row.get(0)?,
                description: row.get(1)?,
                context: row.get(2)?,
                confidence: row.get(3)?,
                observations: row.get(4)?,
                active: row.get::<_, i32>(5)? != 0,
            })
        });

        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                tracing::warn!("PreferenceEngine: query failed: {}", e);
                Vec::new()
            }
        }
    }

    /// Fetch ALL rules (any confidence level) — used by the TUI preferences view.
    pub fn get_all_rules(&self) -> Vec<LearnedRule> {
        let Ok(conn) = self.conn.lock() else { return Vec::new() };

        let mut stmt = match conn.prepare(
            "SELECT id, description, context, confidence, observations, active \
             FROM learned_rules ORDER BY confidence DESC",
        ) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("PreferenceEngine: prepare failed: {}", e);
                return Vec::new();
            }
        };

        let rows = stmt.query_map([], |row| {
            Ok(LearnedRule {
                id: row.get(0)?,
                description: row.get(1)?,
                context: row.get(2)?,
                confidence: row.get(3)?,
                observations: row.get(4)?,
                active: row.get::<_, i32>(5)? != 0,
            })
        });

        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                tracing::warn!("PreferenceEngine: query failed: {}", e);
                Vec::new()
            }
        }
    }

    /// Recompute confidence for all existing rules based on accumulated signals.
    ///
    /// Confidence formula:
    ///   confidence = (accepts * 1.0 + reruns * 0.5) / total
    /// Signals older than 7 days contribute at 0.5x weight.
    /// A rule becomes active when confidence >= 0.8.
    pub fn update_rules(&self) {
        let Ok(conn) = self.conn.lock() else { return };

        // Fetch all rules
        let rule_ids: Vec<i64> = {
            let mut stmt = match conn.prepare("SELECT id FROM learned_rules") {
                Ok(s) => s,
                Err(_) => return,
            };
            stmt.query_map([], |r| r.get(0))
                .ok()
                .map(|iter| iter.filter_map(|r| r.ok()).collect())
                .unwrap_or_default()
        };

        for rule_id in rule_ids {
            // For now rules map to tools by context prefix "TOOL=<name>"; compute from signals
            let context: String = match conn.query_row(
                "SELECT context FROM learned_rules WHERE id = ?1",
                rusqlite::params![rule_id],
                |r| r.get(0),
            ) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Extract tool name from context string e.g. "TOOL=bash"
            let tool_name = context
                .split(',')
                .find_map(|part| part.strip_prefix("TOOL="))
                .unwrap_or("")
                .to_string();

            if tool_name.is_empty() {
                continue;
            }

            // Aggregate signals with time-decay applied
            struct Counts {
                positive: f64,
                total: f64,
            }

            let counts: Counts = {
                let mut stmt = match conn.prepare(
                    "SELECT signal_type, weight, \
                     CASE WHEN julianday('now') - julianday(timestamp) > 7 THEN 0.5 ELSE 1.0 END \
                     AS decay \
                     FROM preference_signals WHERE tool_name = ?1",
                ) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let mut positive = 0.0_f64;
                let mut total = 0.0_f64;

                if let Ok(rows) = stmt.query_map(rusqlite::params![tool_name], |row| {
                    let stype: String = row.get(0)?;
                    let weight: f64 = row.get(1)?;
                    let decay: f64 = row.get(2)?;
                    Ok((stype, weight * decay))
                }) {
                    for row in rows.filter_map(|r| r.ok()) {
                        let (stype, effective_weight) = row;
                        total += effective_weight;
                        match stype.as_str() {
                            "accept" => positive += effective_weight * 1.0,
                            "rerun" => positive += effective_weight * 0.5,
                            _ => {} // reject, edit contribute 0
                        }
                    }
                }

                Counts { positive, total }
            };

            if counts.total < 1.0 {
                continue;
            }

            let confidence = counts.positive / counts.total;
            let active: i32 = if confidence >= 0.8 { 1 } else { 0 };

            let _ = conn.execute(
                "UPDATE learned_rules SET confidence = ?1, active = ?2, \
                 last_updated = datetime('now') WHERE id = ?3",
                rusqlite::params![confidence, active, rule_id],
            );
        }
    }

    /// Crystallise new rules from high-frequency tool patterns.
    ///
    /// A rule is crystallised when a tool accumulates >= 10 signals AND
    /// weighted confidence >= 0.7, and no rule already exists for that tool.
    pub fn crystallise_rules(&self) {
        let Ok(conn) = self.conn.lock() else { return };

        // Find tools with >= 10 signals not already having a rule
        let candidates: Vec<(String, i64)> = {
            let mut stmt = match conn.prepare(
                "SELECT tool_name, COUNT(*) as cnt FROM preference_signals \
                 WHERE tool_name NOT IN (SELECT REPLACE(context, 'TOOL=', '') FROM learned_rules \
                     WHERE context LIKE 'TOOL=%') \
                 GROUP BY tool_name HAVING cnt >= 10",
            ) {
                Ok(s) => s,
                Err(_) => return,
            };
            stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
                .ok()
                .map(|iter| iter.filter_map(|r| r.ok()).collect())
                .unwrap_or_default()
        };

        for (tool_name, _count) in candidates {
            // Compute confidence for this tool
            let mut positive = 0.0_f64;
            let mut total = 0.0_f64;
            let mut observations: i64 = 0;

            {
                let mut stmt = match conn.prepare(
                    "SELECT signal_type, weight, \
                     CASE WHEN julianday('now') - julianday(timestamp) > 7 THEN 0.5 ELSE 1.0 END \
                     FROM preference_signals WHERE tool_name = ?1",
                ) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let results: Vec<(String, f64)> = stmt.query_map(rusqlite::params![tool_name], |row| {
                    let stype: String = row.get(0)?;
                    let weight: f64 = row.get(1)?;
                    let decay: f64 = row.get(2)?;
                    Ok((stype, weight * decay))
                }).ok()
                    .map(|rows| rows.filter_map(|r| r.ok()).collect())
                    .unwrap_or_default();

                for (stype, ew) in results {
                    total += ew;
                    observations += 1;
                    match stype.as_str() {
                        "accept" => positive += ew,
                        "rerun" => positive += ew * 0.5,
                        _ => {}
                    }
                }
            }

            if total < 1.0 {
                continue;
            }

            let confidence = positive / total;
            if confidence < 0.7 {
                continue;
            }

            let active: i32 = if confidence >= 0.8 { 1 } else { 0 };
            let description = format!("Use {} tool (inferred from usage patterns)", tool_name);
            let context = format!("TOOL={}", tool_name);

            let _ = conn.execute(
                "INSERT INTO learned_rules (description, context, confidence, observations, active) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![description, context, confidence, observations, active],
            );

            tracing::info!(
                "PreferenceEngine: crystallised rule for '{}' (conf={:.2}, obs={})",
                tool_name, confidence, observations
            );
        }
    }
}
