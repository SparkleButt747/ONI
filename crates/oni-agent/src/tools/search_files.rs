use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;
use std::process::Command;

pub struct SearchFilesTool;

impl Tool for SearchFilesTool {
    fn name(&self) -> &str {
        "search_files"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::ReadFs]
    }

    fn description(&self) -> &str {
        "Search for a pattern in files using regex. Returns matching lines with file paths and line numbers."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "search_files",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regex pattern to search for"
                        },
                        "path": {
                            "type": "string",
                            "description": "Directory to search in (default: current dir)"
                        },
                        "file_pattern": {
                            "type": "string",
                            "description": "Glob pattern to filter files (e.g. '*.rs', '*.py')"
                        }
                    },
                    "required": ["pattern"]
                }
            }
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let pattern = args
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'pattern' argument"))?;

        let search_path = args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let mut cmd = Command::new("grep");
        cmd.arg("-rn").arg("--color=never");

        if let Some(file_pat) = args.get("file_pattern").and_then(|v| v.as_str()) {
            cmd.arg(format!("--include={}", file_pat));
        }

        cmd.arg("--").arg(pattern).arg(search_path);

        let output = match cmd.output() {
            Ok(o) => o,
            Err(e) => return Ok(format!("Error running grep: {}", e)),
        };

        // grep exits 1 when no matches — that's not an error
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.is_empty() {
            return Ok(format!("No matches found for pattern '{}'", pattern));
        }

        let lines: Vec<&str> = stdout.lines().collect();
        let total = lines.len();
        let truncated = total > 50;
        let shown: Vec<&str> = lines.into_iter().take(50).collect();
        let mut result = shown.join("\n");

        if truncated {
            result.push_str(&format!(
                "\n\n[Truncated: showing 50 of {} matches]",
                total
            ));
        }

        Ok(result)
    }
}
