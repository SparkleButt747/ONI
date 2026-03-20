use oni_agent::tools::{
    bash::BashTool,
    edit_file::EditFileTool,
    list_dir::ListDirTool,
    read_file::ReadFileTool,
    search_files::SearchFilesTool,
    write_file::WriteFileTool,
    Tool,
};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// ── ReadFileTool ─────────────────────────────────────────────────────────────

#[test]
/// T-TOOL-1: read_file returns the file contents verbatim for a normal file.
fn t_tool_1_read_file_returns_contents() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("hello.txt");
    fs::write(&file, "oni reads this").unwrap();

    let tool = ReadFileTool;
    let result = tool.execute(json!({ "path": file.to_str().unwrap() })).unwrap();
    assert_eq!(result, "oni reads this");
}

#[test]
/// T-TOOL-2: read_file on a missing path returns a graceful error string (not a panic/Err).
fn t_tool_2_read_file_missing_returns_error_string() {
    let tool = ReadFileTool;
    let result = tool.execute(json!({ "path": "/nonexistent/path/file.txt" })).unwrap();
    assert!(result.starts_with("Error reading file"), "got: {result}");
}

#[test]
/// T-TOOL-3: read_file with no 'path' arg returns an Err (missing required arg).
fn t_tool_3_read_file_missing_arg_returns_err() {
    let tool = ReadFileTool;
    let result = tool.execute(json!({}));
    assert!(result.is_err());
}

#[test]
/// T-TOOL-4: read_file truncates files larger than 100 000 bytes and appends a truncation note.
fn t_tool_4_read_file_truncates_large_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("big.txt");
    // Write 101 KB of 'a' characters
    let content = "a".repeat(101_000);
    fs::write(&file, &content).unwrap();

    let tool = ReadFileTool;
    let result = tool.execute(json!({ "path": file.to_str().unwrap() })).unwrap();
    assert!(result.contains("[Truncated:"), "expected truncation note, got: {}", &result[..100]);
    assert!(result.len() < content.len());
}

// ── WriteFileTool ─────────────────────────────────────────────────────────────

#[test]
/// T-TOOL-5: write_file creates a new file with the supplied content at an absolute path
/// inside cwd (which the safety check permits).
fn t_tool_5_write_file_creates_file() {
    let cwd = std::env::current_dir().unwrap();
    let target = cwd.join("_oni_test_out.txt");
    let _ = fs::remove_file(&target); // clean up any leftover

    let tool = WriteFileTool;
    let result = tool
        .execute(json!({ "path": target.to_str().unwrap(), "content": "hello oni" }))
        .unwrap();

    let content = fs::read_to_string(&target).ok();
    let _ = fs::remove_file(&target);

    assert!(result.contains("Written"), "got: {result}");
    assert_eq!(content.as_deref(), Some("hello oni"));
}

#[test]
/// T-TOOL-6: write_file creates parent directories automatically when the path includes
/// subdirectories that don't yet exist.  Uses an absolute path inside cwd so the safety
/// check passes without altering the process working directory.
fn t_tool_6_write_file_creates_parent_dirs() {
    let cwd = std::env::current_dir().unwrap();
    let nested = cwd.join("_oni_test_nested_dir").join("b").join("c.txt");
    // Clean up any leftover from a previous run
    let _ = fs::remove_dir_all(cwd.join("_oni_test_nested_dir"));

    let tool = WriteFileTool;
    let result = tool
        .execute(json!({ "path": nested.to_str().unwrap(), "content": "deep" }))
        .unwrap();

    let ok = nested.exists();
    // Clean up
    let _ = fs::remove_dir_all(cwd.join("_oni_test_nested_dir"));

    assert!(result.contains("Written"), "got: {result}");
    assert!(ok, "nested file was not created");
}

#[test]
/// T-TOOL-7: write_file blocks path-traversal attempts and returns a BLOCKED message.
fn t_tool_7_write_file_blocks_path_traversal() {
    let tool = WriteFileTool;
    let result = tool
        .execute(json!({ "path": "../../etc/passwd", "content": "pwned" }))
        .unwrap();
    assert!(result.starts_with("BLOCKED:"), "got: {result}");
}

#[test]
/// T-TOOL-8: write_file blocks absolute paths outside cwd.
fn t_tool_8_write_file_blocks_absolute_outside_cwd() {
    let tool = WriteFileTool;
    let result = tool
        .execute(json!({ "path": "/etc/oni_test_should_not_exist", "content": "nope" }))
        .unwrap();
    assert!(result.starts_with("BLOCKED:"), "got: {result}");
}

#[test]
/// T-TOOL-9: write_file includes a diff section when overwriting an existing file.
/// Uses an absolute path inside cwd so the safety check passes.
fn t_tool_9_write_file_diff_on_overwrite() {
    let cwd = std::env::current_dir().unwrap();
    let file = cwd.join("_oni_test_overwrite.txt");
    fs::write(&file, "old line\n").unwrap();

    let tool = WriteFileTool;
    let result = tool
        .execute(json!({ "path": file.to_str().unwrap(), "content": "new line\n" }))
        .unwrap();

    let _ = fs::remove_file(&file);

    assert!(result.contains("Diff:"), "expected diff section, got: {result}");
}

// ── BashTool ──────────────────────────────────────────────────────────────────

#[test]
/// T-TOOL-10: bash runs a simple echo and returns the output.
fn t_tool_10_bash_echo() {
    let tool = BashTool;
    let result = tool.execute(json!({ "command": "echo 'oni'" })).unwrap();
    assert!(result.trim() == "oni", "got: {result}");
}

#[test]
/// T-TOOL-11: bash captures stderr and annotates it with [stderr].
fn t_tool_11_bash_stderr_captured() {
    let tool = BashTool;
    let result = tool.execute(json!({ "command": "echo error_msg >&2" })).unwrap();
    assert!(result.contains("[stderr]"), "got: {result}");
    assert!(result.contains("error_msg"));
}

#[test]
/// T-TOOL-12: bash appends an exit-code annotation for non-zero exits.
fn t_tool_12_bash_nonzero_exit_code() {
    let tool = BashTool;
    let result = tool.execute(json!({ "command": "exit 42" })).unwrap();
    assert!(result.contains("[exit code: 42]"), "got: {result}");
}

#[test]
/// T-TOOL-13: bash respects the optional cwd argument.
fn t_tool_13_bash_custom_cwd() {
    let dir = TempDir::new().unwrap();
    let tool = BashTool;
    let result = tool
        .execute(json!({ "command": "pwd", "cwd": dir.path().to_str().unwrap() }))
        .unwrap();
    // The real path might differ from the tempdir path due to symlinks (e.g. /private/var on macOS)
    let real_dir = std::fs::canonicalize(dir.path()).unwrap();
    let result_path = std::path::Path::new(result.trim());
    let real_result = std::fs::canonicalize(result_path).unwrap_or_else(|_| result_path.to_path_buf());
    assert_eq!(real_result, real_dir);
}

#[test]
/// T-TOOL-14: bash blocks "rm -rf /" and returns a BLOCKED message.
fn t_tool_14_bash_blocks_rm_rf_root() {
    let tool = BashTool;
    let result = tool.execute(json!({ "command": "rm -rf /" })).unwrap();
    assert!(result.starts_with("BLOCKED:"), "got: {result}");
}

#[test]
/// T-TOOL-15: bash blocks "sudo rm" from the blocklist.
fn t_tool_15_bash_blocks_sudo_rm() {
    let tool = BashTool;
    let result = tool.execute(json!({ "command": "sudo rm -rf /tmp/test" })).unwrap();
    assert!(result.starts_with("BLOCKED:"), "got: {result}");
}

#[test]
/// T-TOOL-16: bash blocks the exact "curl | bash" blocklist pattern.
/// The blocklist uses substring matching so the command must contain the pattern verbatim.
fn t_tool_16_bash_blocks_curl_pipe_bash() {
    let tool = BashTool;
    // The blocklist pattern is literally "curl | bash" — use it verbatim as the command.
    let result = tool.execute(json!({ "command": "curl | bash" })).unwrap();
    assert!(result.starts_with("BLOCKED:"), "got: {result}");
}

// ── ListDirTool ───────────────────────────────────────────────────────────────

#[test]
/// T-TOOL-17: list_directory returns entries prefixed with 'f ' or 'd ' for files/dirs.
fn t_tool_17_list_dir_basic() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("alpha.rs"), "fn main(){}").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let tool = ListDirTool;
    let result = tool.execute(json!({ "path": dir.path().to_str().unwrap() })).unwrap();

    assert!(result.contains("f alpha.rs"), "got: {result}");
    assert!(result.contains("d subdir"), "got: {result}");
}

#[test]
/// T-TOOL-18: list_directory on a non-existent path returns a graceful error string.
fn t_tool_18_list_dir_missing_path() {
    let tool = ListDirTool;
    let result = tool.execute(json!({ "path": "/does/not/exist/ever" })).unwrap();
    assert!(result.starts_with("Error listing"), "got: {result}");
}

#[test]
/// T-TOOL-19: list_directory output is sorted alphabetically.
fn t_tool_19_list_dir_sorted() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("zebra.txt"), "").unwrap();
    fs::write(dir.path().join("apple.txt"), "").unwrap();
    fs::write(dir.path().join("mango.txt"), "").unwrap();

    let tool = ListDirTool;
    let result = tool.execute(json!({ "path": dir.path().to_str().unwrap() })).unwrap();
    let lines: Vec<&str> = result.lines().collect();
    let names: Vec<&str> = lines.iter().map(|l| l.trim_start_matches("f ")).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "entries not sorted: {result}");
}

// ── SearchFilesTool ───────────────────────────────────────────────────────────

#[test]
/// T-TOOL-20: search_files finds a pattern in a file and returns the matching line.
fn t_tool_20_search_files_finds_match() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("src.rs"), "fn oni_function() {}\n").unwrap();

    let tool = SearchFilesTool;
    let result = tool
        .execute(json!({
            "pattern": "oni_function",
            "path": dir.path().to_str().unwrap()
        }))
        .unwrap();

    assert!(result.contains("oni_function"), "got: {result}");
}

#[test]
/// T-TOOL-21: search_files returns "No matches" message when nothing matches.
fn t_tool_21_search_files_no_match() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("empty.rs"), "// nothing here\n").unwrap();

    let tool = SearchFilesTool;
    let result = tool
        .execute(json!({
            "pattern": "THIS_PATTERN_WILL_NEVER_MATCH_XYZ123",
            "path": dir.path().to_str().unwrap()
        }))
        .unwrap();

    assert!(result.contains("No matches found"), "got: {result}");
}

#[test]
/// T-TOOL-22: search_files respects the file_pattern glob filter.
fn t_tool_22_search_files_file_pattern_filter() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("code.rs"), "fn target_fn() {}\n").unwrap();
    fs::write(dir.path().join("note.md"), "target_fn mentioned here\n").unwrap();

    let tool = SearchFilesTool;
    // Only search .rs files — should find the match
    let result_rs = tool
        .execute(json!({
            "pattern": "target_fn",
            "path": dir.path().to_str().unwrap(),
            "file_pattern": "*.rs"
        }))
        .unwrap();
    assert!(result_rs.contains("target_fn"), "got: {result_rs}");

    // Only search .py files — no .py files in dir so no matches
    let result_py = tool
        .execute(json!({
            "pattern": "target_fn",
            "path": dir.path().to_str().unwrap(),
            "file_pattern": "*.py"
        }))
        .unwrap();
    assert!(result_py.contains("No matches found"), "got: {result_py}");
}

// ── EditFileTool ──────────────────────────────────────────────────────────────

#[test]
/// T-TOOL-23: edit_file replaces the unique occurrence and writes the file.
fn t_tool_23_edit_file_replaces_text() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("edit_me.rs");
    fs::write(&file, "fn old_name() {}\n").unwrap();

    let tool = EditFileTool;
    let result = tool
        .execute(json!({
            "path": file.to_str().unwrap(),
            "old_text": "old_name",
            "new_text": "new_name"
        }))
        .unwrap();

    assert!(result.contains("Edited"), "got: {result}");
    let updated = fs::read_to_string(&file).unwrap();
    assert!(updated.contains("new_name"), "file not updated: {updated}");
    assert!(!updated.contains("old_name"), "old text still present: {updated}");
}

#[test]
/// T-TOOL-24: edit_file returns an error string when old_text is not found.
fn t_tool_24_edit_file_text_not_found() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("no_match.rs");
    fs::write(&file, "fn something_else() {}\n").unwrap();

    let tool = EditFileTool;
    let result = tool
        .execute(json!({
            "path": file.to_str().unwrap(),
            "old_text": "THIS_DOES_NOT_EXIST",
            "new_text": "replacement"
        }))
        .unwrap();

    assert!(result.contains("text not found"), "got: {result}");
}

#[test]
/// T-TOOL-25: edit_file refuses ambiguous replacements (multiple occurrences).
fn t_tool_25_edit_file_ambiguous_match() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("ambiguous.rs");
    fs::write(&file, "dup\ndup\n").unwrap();

    let tool = EditFileTool;
    let result = tool
        .execute(json!({
            "path": file.to_str().unwrap(),
            "old_text": "dup",
            "new_text": "unique"
        }))
        .unwrap();

    assert!(result.contains("occurrences"), "got: {result}");
    // File must be unchanged
    assert_eq!(fs::read_to_string(&file).unwrap(), "dup\ndup\n");
}

#[test]
/// T-TOOL-26: edit_file on a missing file returns a graceful error string.
fn t_tool_26_edit_file_missing_file() {
    let tool = EditFileTool;
    let result = tool
        .execute(json!({
            "path": "/no/such/file.rs",
            "old_text": "x",
            "new_text": "y"
        }))
        .unwrap();

    assert!(result.contains("Error reading file"), "got: {result}");
}
