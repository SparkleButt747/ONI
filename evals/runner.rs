//! ONI LLM Eval Framework
//!
//! Loads YAML fixtures from evals/fixtures/ and runs assertions against
//! actual LLM responses via Ollama. Not part of `cargo test` — run with:
//!   cargo run --bin oni-eval
//!
//! Each fixture defines:
//!   - name: identifier
//!   - input: conversation messages
//!   - assertions: checks on the response
//!   - tier: optional model tier (default: fast)

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct EvalFixture {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_tier")]
    tier: String,
    input: Vec<EvalMessage>,
    assertions: Vec<Assertion>,
}

fn default_tier() -> String {
    "fast".into()
}

#[derive(Debug, Deserialize)]
struct EvalMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum Assertion {
    #[serde(rename = "contains")]
    Contains { value: String },
    #[serde(rename = "not_contains")]
    NotContains { value: String },
    #[serde(rename = "contains_any")]
    ContainsAny { values: Vec<String> },
    #[serde(rename = "has_tool_call")]
    HasToolCall { tool: String },
    #[serde(rename = "no_tool_call")]
    NoToolCall { tool: String },
    #[serde(rename = "max_length")]
    MaxLength { chars: usize },
}

impl Assertion {
    fn check(&self, response: &str) -> Result<(), String> {
        match self {
            Assertion::Contains { value } => {
                if !response.contains(value) {
                    Err(format!("Expected response to contain '{}'", value))
                } else {
                    Ok(())
                }
            }
            Assertion::NotContains { value } => {
                if response.contains(value) {
                    Err(format!("Response should NOT contain '{}' but does", value))
                } else {
                    Ok(())
                }
            }
            Assertion::ContainsAny { values } => {
                let lower = response.to_lowercase();
                if values.iter().any(|v| lower.contains(&v.to_lowercase())) {
                    Ok(())
                } else {
                    Err(format!("Expected response to contain any of {:?}", values))
                }
            }
            Assertion::HasToolCall { tool } => {
                // Check for tool call patterns in response
                let has_native = response.contains(&format!("\"name\":\"{}\"", tool))
                    || response.contains(&format!("\"tool\":\"{}\"", tool));
                let has_xml = response.contains(&format!("<function={}", tool));
                if has_native || has_xml {
                    Ok(())
                } else {
                    Err(format!("Expected tool call to '{}'", tool))
                }
            }
            Assertion::NoToolCall { tool } => {
                let has = response.contains(&format!("\"name\":\"{}\"", tool))
                    || response.contains(&format!("<function={}", tool));
                if has {
                    Err(format!("Expected NO tool call to '{}'", tool))
                } else {
                    Ok(())
                }
            }
            Assertion::MaxLength { chars } => {
                if response.len() > *chars {
                    Err(format!(
                        "Response too long: {} chars (max {})",
                        response.len(),
                        chars
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn load_fixtures() -> Vec<EvalFixture> {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("evals").join("fixtures");
    let mut fixtures = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&fixtures_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
                match std::fs::read_to_string(&path) {
                    Ok(text) => match serde_yaml::from_str::<EvalFixture>(&text) {
                        Ok(fixture) => fixtures.push(fixture),
                        Err(e) => eprintln!("  SKIP {}: parse error: {}", path.display(), e),
                    },
                    Err(e) => eprintln!("  SKIP {}: read error: {}", path.display(), e),
                }
            }
        }
    }

    fixtures
}

/// Run all eval fixtures. Returns (passed, failed, total).
pub fn run_evals() -> (usize, usize, usize) {
    let fixtures = load_fixtures();
    let total = fixtures.len();
    let mut passed = 0;
    let mut failed = 0;

    println!("ONI EVAL FRAMEWORK");
    println!("==================");
    println!("Loaded {} fixtures\n", total);

    for fixture in &fixtures {
        print!("  {} ... ", fixture.name);

        // For now, just validate the fixture structure without running LLM
        // (actual LLM eval requires async runtime + Ollama)
        let mut fixture_ok = true;
        for assertion in &fixture.assertions {
            // Dry-run: just check assertion is well-formed
            match assertion {
                Assertion::Contains { value } if value.is_empty() => {
                    println!("INVALID (empty contains)");
                    fixture_ok = false;
                }
                _ => {}
            }
        }

        if fixture_ok {
            println!("VALID ({} assertions)", fixture.assertions.len());
            passed += 1;
        } else {
            failed += 1;
        }
    }

    println!("\n==================");
    println!("FIXTURES: {}/{} valid", passed, total);

    (passed, failed, total)
}
