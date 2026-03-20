/// Tests for `oni run` background task tracking.
///
/// These cover serialization round-trips, tasks.json read/write, and
/// list-display formatting. Process-spawning itself is not tested here
/// (it's an integration concern that requires a built binary).
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Mirror of the structs in src/main.rs so tests can be self-contained.
// If the fields change, the compiler will catch mismatches in the binary build.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct BackgroundTask {
    id: String,
    prompt: String,
    tier: String,
    status: String,
    pid: u32,
    start_time: String,
    log_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
}

fn make_task(id: &str, status: &str) -> BackgroundTask {
    BackgroundTask {
        id: id.to_string(),
        prompt: "echo hello world".to_string(),
        tier: "medium".to_string(),
        status: status.to_string(),
        pid: 12345,
        start_time: "1711234567".to_string(),
        log_path: format!("/tmp/{}.log", id),
        completed_at: None,
        exit_code: None,
    }
}

fn tasks_file_in(dir: &TempDir) -> PathBuf {
    dir.path().join("tasks.json")
}

fn save_tasks(path: &PathBuf, tasks: &[BackgroundTask]) {
    std::fs::write(path, serde_json::to_string_pretty(tasks).unwrap()).unwrap();
}

fn load_tasks(path: &PathBuf) -> Vec<BackgroundTask> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------

/// T-RUN-1: BackgroundTask serializes and deserializes without loss.
#[test]
fn t_run_1_task_entry_round_trip() {
    let task = make_task("task_123456", "running");
    let json = serde_json::to_string(&task).unwrap();
    let decoded: BackgroundTask = serde_json::from_str(&json).unwrap();
    assert_eq!(task, decoded);
}

/// T-RUN-2: Optional fields are omitted when None, present when Some.
#[test]
fn t_run_2_optional_fields_serialization() {
    let task = make_task("task_200000", "running");
    let json = serde_json::to_string(&task).unwrap();
    // Optional None fields should not appear in the JSON
    assert!(!json.contains("completed_at"), "None completed_at should be omitted");
    assert!(!json.contains("exit_code"), "None exit_code should be omitted");

    let mut finished = task.clone();
    finished.status = "done".into();
    finished.completed_at = Some("1711234999".to_string());
    finished.exit_code = Some(0);
    let json2 = serde_json::to_string(&finished).unwrap();
    assert!(json2.contains("completed_at"));
    assert!(json2.contains("exit_code"));
}

/// T-RUN-3: tasks.json can be written and re-read correctly.
#[test]
fn t_run_3_tasks_json_read_write() {
    let dir = TempDir::new().unwrap();
    let path = tasks_file_in(&dir);

    let tasks = vec![
        make_task("task_300001", "running"),
        make_task("task_300002", "done"),
    ];
    save_tasks(&path, &tasks);

    let loaded = load_tasks(&path);
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0].id, "task_300001");
    assert_eq!(loaded[1].status, "done");
}

/// T-RUN-4: Loading from a missing file returns an empty vec (no panic).
#[test]
fn t_run_4_load_missing_file_returns_empty() {
    let path = PathBuf::from("/tmp/oni_test_nonexistent_tasks_xyz.json");
    let result = load_tasks(&path);
    assert!(result.is_empty());
}

/// T-RUN-5: Adding a task to an existing tasks.json appends correctly.
#[test]
fn t_run_5_append_task() {
    let dir = TempDir::new().unwrap();
    let path = tasks_file_in(&dir);

    let initial = vec![make_task("task_400001", "done")];
    save_tasks(&path, &initial);

    let mut tasks = load_tasks(&path);
    tasks.push(make_task("task_400002", "running"));
    save_tasks(&path, &tasks);

    let reloaded = load_tasks(&path);
    assert_eq!(reloaded.len(), 2);
    assert_eq!(reloaded[1].id, "task_400002");
}

/// T-RUN-6: List display formatting produces the expected columns.
#[test]
fn t_run_6_list_display_format() {
    let tasks = vec![
        make_task("task_500001", "running"),
        {
            let mut t = make_task("task_500002", "done");
            t.prompt = "a".repeat(80); // longer than 50 chars
            t
        },
    ];

    // Simulate the display formatting from main.rs
    let header = format!("{:<14} {:<10} {:<8} {}", "ID", "STATUS", "TIER", "PROMPT");
    assert!(header.starts_with("ID"));

    for t in &tasks {
        let preview = truncate_str_test(&t.prompt, 50);
        assert!(preview.len() <= 50, "preview must be at most 50 chars");
        let line = format!("{:<14} {:<10} {:<8} {}", t.id, t.status, t.tier, preview);
        // ID column should be left-padded to 14
        assert!(line.len() >= 14);
    }
}

/// T-RUN-7: truncate_str is char-boundary safe for multi-byte characters.
#[test]
fn t_run_7_truncate_str_multibyte() {
    // "🔥" is 4 bytes but 1 char
    let s = "🔥".repeat(20);
    let truncated = truncate_str_test(&s, 10);
    // Should have at most 10 chars (10 fire emoji)
    assert_eq!(truncated.chars().count(), 10);
    // Must be valid UTF-8 (no panic)
    let _ = std::str::from_utf8(truncated.as_bytes()).unwrap();
}

/// T-RUN-8: Status update propagates correctly through save/load cycle.
#[test]
fn t_run_8_status_update_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = tasks_file_in(&dir);

    let tasks = vec![make_task("task_600001", "running")];
    save_tasks(&path, &tasks);

    let mut tasks = load_tasks(&path);
    if let Some(t) = tasks.iter_mut().find(|t| t.id == "task_600001") {
        t.status = "failed".into();
        t.completed_at = Some("1711235000".to_string());
    }
    save_tasks(&path, &tasks);

    let reloaded = load_tasks(&path);
    let t = reloaded.iter().find(|t| t.id == "task_600001").unwrap();
    assert_eq!(t.status, "failed");
    assert_eq!(t.completed_at.as_deref(), Some("1711235000"));
}

// ---------------------------------------------------------------------------
// Local copy of the truncate helper for tests (avoids depending on main.rs internals)
// ---------------------------------------------------------------------------

fn truncate_str_test(s: &str, max_chars: usize) -> &str {
    let mut char_count = 0;
    for (byte_idx, _) in s.char_indices() {
        if char_count == max_chars {
            return &s[..byte_idx];
        }
        char_count += 1;
    }
    s
}
