//! Code Review — context-aware review of git diffs using the Critic agent.
//!
//! Analyses staged changes or arbitrary diffs for:
//! - Pattern consistency with the project
//! - Missing edge cases
//! - Security issues
//! - Style violations
//! - Correctness

use oni_core::error::Result;
use oni_core::types::ModelTier;
use oni_llm::ModelRouter;

/// Result of a code review.
#[derive(Debug, Clone)]
pub struct ReviewResult {
    pub summary: String,
    pub issues: Vec<ReviewIssue>,
    pub verdict: ReviewVerdict,
}

#[derive(Debug, Clone)]
pub struct ReviewIssue {
    pub severity: IssueSeverity,
    pub file: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IssueSeverity {
    Error,   // Must fix
    Warning, // Should fix
    Info,    // Suggestion
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReviewVerdict {
    Pass,
    Warn,
    Fail,
}

impl std::fmt::Display for IssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "ERROR"),
            Self::Warning => write!(f, "WARN"),
            Self::Info => write!(f, "INFO"),
        }
    }
}

impl std::fmt::Display for ReviewVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "PASS"),
            Self::Warn => write!(f, "WARN"),
            Self::Fail => write!(f, "FAIL"),
        }
    }
}

const REVIEW_PROMPT: &str = "\
You are ONI's code review agent. Review the following git diff for:
1. CORRECTNESS — bugs, logic errors, off-by-one, null/unwrap safety
2. SECURITY — injection, path traversal, hardcoded secrets, unsafe operations
3. STYLE — naming conventions, dead code, unnecessary complexity
4. EDGE CASES — missing error handling, boundary conditions
5. PATTERNS — does this change match the project's existing patterns?

For each issue found, output EXACTLY this format (one per line):
[ERROR|WARN|INFO] file.rs: description of the issue

After all issues, output a final verdict line:
VERDICT: [PASS|WARN|FAIL]

If the code looks good, output:
No issues found.
VERDICT: PASS

Be concise. No preamble. Issues only.";

/// Run a code review on the given diff text.
pub async fn review_diff(
    router: &ModelRouter,
    diff: &str,
    tier: ModelTier,
    project_context: Option<&str>,
) -> Result<ReviewResult> {
    let mut prompt = REVIEW_PROMPT.to_string();
    if let Some(ctx) = project_context {
        prompt.push_str(&format!("\n\nProject context:\n{}", ctx));
    }

    let messages = vec![
        oni_llm::ChatMessage::system(&prompt),
        oni_llm::ChatMessage::user(&format!("Review this diff:\n\n```diff\n{}\n```", diff)),
    ];

    let response = router.chat(tier, messages).await?;
    let content = response.message().content.clone();
    let content = &content;

    // Parse the response into structured ReviewResult
    let mut issues = Vec::new();
    let mut verdict = ReviewVerdict::Pass;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("[ERROR]") {
            let rest = trimmed.strip_prefix("[ERROR]").unwrap_or("").trim();
            let (file, desc) = split_file_desc(rest);
            issues.push(ReviewIssue {
                severity: IssueSeverity::Error,
                file,
                description: desc,
            });
        } else if trimmed.starts_with("[WARN]") {
            let rest = trimmed.strip_prefix("[WARN]").unwrap_or("").trim();
            let (file, desc) = split_file_desc(rest);
            issues.push(ReviewIssue {
                severity: IssueSeverity::Warning,
                file,
                description: desc,
            });
        } else if trimmed.starts_with("[INFO]") {
            let rest = trimmed.strip_prefix("[INFO]").unwrap_or("").trim();
            let (file, desc) = split_file_desc(rest);
            issues.push(ReviewIssue {
                severity: IssueSeverity::Info,
                file,
                description: desc,
            });
        } else if trimmed.starts_with("VERDICT:") {
            let v = trimmed.strip_prefix("VERDICT:").unwrap_or("").trim().to_uppercase();
            verdict = match v.as_str() {
                "FAIL" => ReviewVerdict::Fail,
                "WARN" => ReviewVerdict::Warn,
                _ => ReviewVerdict::Pass,
            };
        }
    }

    // If there are errors but verdict is PASS, override to FAIL
    if issues.iter().any(|i| i.severity == IssueSeverity::Error) && verdict == ReviewVerdict::Pass {
        verdict = ReviewVerdict::Fail;
    }

    let summary = if issues.is_empty() {
        "No issues found.".into()
    } else {
        format!(
            "{} issue(s): {} error, {} warn, {} info",
            issues.len(),
            issues.iter().filter(|i| i.severity == IssueSeverity::Error).count(),
            issues.iter().filter(|i| i.severity == IssueSeverity::Warning).count(),
            issues.iter().filter(|i| i.severity == IssueSeverity::Info).count(),
        )
    };

    Ok(ReviewResult {
        summary,
        issues,
        verdict,
    })
}

fn split_file_desc(s: &str) -> (String, String) {
    if let Some(idx) = s.find(':') {
        let file = s[..idx].trim().to_string();
        let desc = s[idx + 1..].trim().to_string();
        (file, desc)
    } else {
        ("?".into(), s.to_string())
    }
}

/// Get the current staged git diff.
pub fn get_staged_diff() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached"])
        .output()
        .ok()?;
    let diff = String::from_utf8_lossy(&output.stdout).to_string();
    if diff.trim().is_empty() {
        // Fall back to unstaged diff
        let output = std::process::Command::new("git")
            .args(["diff"])
            .output()
            .ok()?;
        let diff = String::from_utf8_lossy(&output.stdout).to_string();
        if diff.trim().is_empty() {
            None
        } else {
            Some(diff)
        }
    } else {
        Some(diff)
    }
}
