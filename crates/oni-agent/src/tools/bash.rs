use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;
use std::process::Command;

const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "rm -rf ~",
    "mkfs",
    "dd if=",
    ":(){:|:&};:",
    "chmod -r 777 /",
    "sudo rm",
    "sudo dd",
    "sudo mkfs",
    "> /dev/sda",
    "curl | sh",
    "curl | bash",
    "wget | sh",
    "wget | bash",
];

fn is_blocked(command: &str) -> bool {
    let normalised: String = command
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    for pattern in BLOCKED_PATTERNS {
        if normalised.contains(pattern) {
            return true;
        }
    }
    false
}

pub struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::ExecShell]
    }

    fn description(&self) -> &str {
        "Execute a bash command and return its output"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "bash",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The bash command to execute"
                        },
                        "cwd": {
                            "type": "string",
                            "description": "Working directory for the command (defaults to current directory)"
                        }
                    },
                    "required": ["command"]
                }
            }
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'command' argument"))?;

        // Check blocklist before executing
        if is_blocked(command) {
            return Ok(format!(
                "BLOCKED: Command matches security blocklist pattern."
            ));
        }

        let cwd = args.get("cwd").and_then(|v| v.as_str());

        let mut cmd = Command::new("bash");
        cmd.arg("-c").arg(command);

        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        let output = cmd
            .output()
            .map_err(|e| oni_core::error::err!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        let mut result = String::new();
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str("[stderr] ");
            result.push_str(&stderr);
        }
        if !output.status.success() {
            result.push_str(&format!("\n[exit code: {}]", output.status.code().unwrap_or(-1)));
        }

        // Truncate very long output, respecting UTF-8 char boundaries
        if result.len() > 50_000 {
            let mut end = 50_000;
            while end > 0 && !result.is_char_boundary(end) {
                end -= 1;
            }
            result.truncate(end);
            result.push_str("\n...[truncated]");
        }

        Ok(result)
    }
}
