/// T-EVAL: Validate that all YAML eval fixtures parse correctly.
/// This doesn't run the LLM — it just checks fixture integrity.

use std::path::PathBuf;

#[derive(Debug, serde::Deserialize)]
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

#[derive(Debug, serde::Deserialize)]
struct EvalMessage {
    role: String,
    content: String,
}

#[derive(Debug, serde::Deserialize)]
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

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("evals")
        .join("fixtures")
}

#[test]
fn t_eval_1_fixtures_dir_exists() {
    assert!(fixtures_dir().exists(), "evals/fixtures/ directory should exist");
}

#[test]
fn t_eval_2_at_least_one_fixture() {
    let count = std::fs::read_dir(fixtures_dir())
        .unwrap()
        .filter(|e| {
            e.as_ref()
                .ok()
                .map(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "yaml" || ext == "yml")
                        .unwrap_or(false)
                })
                .unwrap_or(false)
        })
        .count();
    assert!(count >= 1, "should have at least 1 eval fixture");
}

#[test]
fn t_eval_3_all_fixtures_parse() {
    let dir = fixtures_dir();
    let mut parsed = 0;
    let mut errors = Vec::new();

    for entry in std::fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map(|e| e == "yaml" || e == "yml").unwrap_or(false) {
            let text = std::fs::read_to_string(&path).unwrap();
            match serde_yaml::from_str::<EvalFixture>(&text) {
                Ok(fixture) => {
                    assert!(!fixture.name.is_empty(), "fixture name should not be empty");
                    assert!(!fixture.input.is_empty(), "fixture input should not be empty");
                    assert!(
                        !fixture.assertions.is_empty(),
                        "fixture '{}' should have at least one assertion",
                        fixture.name
                    );
                    parsed += 1;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", path.display(), e));
                }
            }
        }
    }

    assert!(errors.is_empty(), "Fixture parse errors:\n{}", errors.join("\n"));
    assert!(parsed >= 1, "should parse at least 1 fixture");
}

#[test]
fn t_eval_4_no_comfort_phrasing_fixture() {
    let text = std::fs::read_to_string(fixtures_dir().join("no_comfort_phrasing.yaml")).unwrap();
    let fixture: EvalFixture = serde_yaml::from_str(&text).unwrap();
    assert_eq!(fixture.name, "no_comfort_phrasing");
    assert!(fixture.assertions.len() >= 5, "should have at least 5 not_contains assertions");
}

#[test]
fn t_eval_5_planner_fixture() {
    let text = std::fs::read_to_string(fixtures_dir().join("planner_decomposes.yaml")).unwrap();
    let fixture: EvalFixture = serde_yaml::from_str(&text).unwrap();
    assert_eq!(fixture.tier, "heavy");
}
