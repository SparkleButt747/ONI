use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;
use std::path::Path;

pub struct WriteFileTool;

fn is_safe_path(path: &str) -> bool {
    let p = std::path::Path::new(path);
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return false;
        }
    }
    if p.is_absolute() {
        let cwd = match std::env::current_dir() {
            Ok(c) => c,
            Err(_) => return false,
        };
        if !p.starts_with(&cwd) {
            return false;
        }
    }
    true
}

fn compute_diff(old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let added = new_lines.len() as i64 - old_lines.len() as i64;
    let mut diff = format!(
        "[{} -> {} lines, net {:+}]\n",
        old_lines.len(),
        new_lines.len(),
        added
    );

    for line in new_lines.iter().take(5) {
        if !old_lines.contains(line) {
            diff.push_str(&format!("+ {}\n", line));
        }
    }
    for line in old_lines.iter().take(5) {
        if !new_lines.contains(line) {
            diff.push_str(&format!("- {}\n", line));
        }
    }

    diff
}

impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::WriteFs]
    }

    fn description(&self) -> &str {
        "Write content to a file at the given path. Creates parent directories if needed."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute or relative path to the file"
                        },
                        "content": {
                            "type": "string",
                            "description": "Content to write to the file"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'path' argument"))?;
        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'content' argument"))?;

        // CWD constraint — reject writes outside project directory
        if !is_safe_path(path) {
            return Ok(format!(
                "BLOCKED: Cannot write outside project directory: {}",
                path
            ));
        }

        let file_path = Path::new(path);

        // If file exists, compute diff before overwriting
        let diff_output = if file_path.exists() {
            match std::fs::read_to_string(file_path) {
                Ok(existing) => {
                    let diff = compute_diff(&existing, content);
                    Some(format!("Diff: {}", diff))
                }
                Err(_) => None,
            }
        } else {
            None
        };

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| oni_core::error::err!("Failed to create directories: {}", e))?;
        }

        match std::fs::write(path, content) {
            Ok(()) => {
                let mut result = format!("Written {} bytes to {}", content.len(), path);
                if let Some(diff) = diff_output {
                    result.push('\n');
                    result.push_str(&diff);
                }
                Ok(result)
            }
            Err(e) => Ok(format!("Error writing file '{}': {}", path, e)),
        }
    }
}

// TODO: Apply is_safe_path check to edit_file tool once it is created.
