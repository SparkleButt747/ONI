//! Callback System — unprompted references to past interactions.
//!
//! "Last time you hit this error it was a missing semicolon in the macro expansion."
//!
//! Reads journal entries and searches for relevance to the current query.
//! Returns a callback string to inject into the system prompt when a relevant
//! past episode is found.

use oni_core::personality;
use std::path::PathBuf;

/// Check if there's a relevant past episode for the current query.
/// Returns Some(callback_text) if a match is found, None otherwise.
///
/// This is intentionally probabilistic — we don't want callbacks every turn.
/// Triggered when:
/// 1. Current query has keyword overlap with a journal entry
/// 2. A random check passes (avoid being annoying)
pub fn find_callback(query: &str, db_path: &PathBuf) -> Option<String> {
    // Only trigger callbacks ~20% of the time to avoid being annoying
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    if nanos % 5 != 0 {
        return None; // ~80% skip rate
    }

    let query_lower = query.to_lowercase();
    let query_words: Vec<&str> = query_lower
        .split_whitespace()
        .filter(|w| w.len() > 3) // Skip short words
        .collect();

    if query_words.is_empty() {
        return None;
    }

    // Search journal entries (today + last 7 days)
    let mut best_match: Option<(usize, String)> = None;

    for days_ago in 0..7 {
        let date = date_n_days_ago(days_ago);
        let path = personality::journal_path_for_date(&date);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Score each line by keyword overlap
        for line in content.lines() {
            if line.starts_with('#') || line.trim().is_empty() || line.starts_with("- Turns:") {
                continue; // Skip headers and metadata
            }
            let line_lower = line.to_lowercase();
            let overlap: usize = query_words
                .iter()
                .filter(|w| line_lower.contains(*w))
                .count();

            if overlap >= 2 {
                // Need at least 2 keyword matches
                match &best_match {
                    Some((best_score, _)) if overlap <= *best_score => {}
                    _ => {
                        best_match = Some((overlap, line.trim().to_string()));
                    }
                }
            }
        }
    }

    // Also check DB for similar tool events
    if let Ok(db) = oni_db::Database::open(db_path) {
        if let Some(callback) = search_tool_history(&db, &query_words) {
            return Some(callback);
        }
    }

    best_match.map(|(_, line)| {
        format!("I recall a related episode: {}", line)
    })
}

/// Search tool event history for patterns matching the current query.
fn search_tool_history(db: &oni_db::Database, query_words: &[&str]) -> Option<String> {
    // Look for similar bash commands or file operations in tool_events
    let sql = "SELECT tool_name, args_json, result_json FROM tool_events \
               ORDER BY timestamp DESC LIMIT 100";
    let mut stmt = db.conn().prepare(sql).ok()?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0).unwrap_or_default(),
                row.get::<_, String>(1).unwrap_or_default(),
                row.get::<_, String>(2).unwrap_or_default(),
            ))
        })
        .ok()?;

    for row in rows.flatten() {
        let (tool, args, result) = row;
        let combined = format!("{} {} {}", tool, args, result).to_lowercase();

        let overlap: usize = query_words
            .iter()
            .filter(|w| combined.contains(*w))
            .count();

        if overlap >= 2 {
            // Found a relevant past tool call
            if result.contains("error") || result.contains("Error") {
                return Some(format!(
                    "Last time this came up, {} returned an error. Worth checking if the same issue applies.",
                    tool
                ));
            }
        }
    }

    None
}

/// Get date string N days ago.
fn date_n_days_ago(n: u64) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(n * 86400);
    let days = secs / 86400;
    let (y, m, d) = days_to_ymd(days);
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
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
