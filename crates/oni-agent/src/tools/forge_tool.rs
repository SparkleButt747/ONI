//! forge_tool — Dynamic tool generation. FENRIR generates one-off bash scripts
//! at runtime when no existing tool fits the need. Scripts are ephemeral.
//!
//! Named after the forging metaphor — ONI forges new tools as needed.
//! Safety: scripts are validated (syntax check) and gated by autonomy level.

use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;
use std::io::Read;
use std::process::Command;

const BLOCKED_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    ":(){ :|:& };:",
    "mkfs.",
    "dd if=",
    "sudo rm",
    "> /dev/sda",
    "chmod -R 777 /",
    "curl | sh",
    "curl | bash",
    "wget | sh",
    "wget | bash",
];

pub struct ForgeTool;

impl Tool for ForgeTool {
    fn name(&self) -> &str {
        "forge_tool"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::ExecShell]
    }

    fn description(&self) -> &str {
        "Generate and execute a one-off bash script for a specific task. \
         Provide a 'description' of what the script should do, and a 'script' \
         containing the bash code. The script will be syntax-checked before execution."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "forge_tool",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "What this script should do"
                        },
                        "script": {
                            "type": "string",
                            "description": "The bash script to execute"
                        }
                    },
                    "required": ["description", "script"]
                }
            }
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let description = args
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'description' argument"))?;
        let script = args
            .get("script")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'script' argument"))?;

        // Safety: check for dangerous patterns (same blocklist as bash tool)
        let script_lower = script.to_lowercase();
        for pattern in BLOCKED_PATTERNS {
            if script_lower.contains(pattern) {
                return Ok(format!(
                    "BLOCKED: Forged script contains dangerous pattern: {}",
                    pattern
                ));
            }
        }

        // Syntax check: use bash -n to validate without executing
        let syntax_check = Command::new("bash")
            .args(["-n", "-c", script])
            .output()
            .map_err(|e| oni_core::error::err!("Syntax check failed: {}", e))?;

        if !syntax_check.status.success() {
            let stderr = String::from_utf8_lossy(&syntax_check.stderr);
            return Ok(format!(
                "Script syntax error:\n{}\n\nFix the script and try again.",
                stderr.trim()
            ));
        }

        // Execute the script with a 30-second timeout
        let mut child = Command::new("bash")
            .args(["-c", script])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| oni_core::error::err!("Execution failed: {}", e))?;

        let timeout = std::time::Duration::from_secs(30);
        let start = std::time::Instant::now();
        let exit_status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        return Ok("Error: script execution timed out after 30 seconds.".into());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                Err(e) => return Ok(format!("Error waiting for script: {}", e)),
            }
        };

        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_end(&mut stdout_bytes);
        }
        if let Some(mut err) = child.stderr.take() {
            let _ = err.read_to_end(&mut stderr_bytes);
        }
        let stdout = String::from_utf8_lossy(&stdout_bytes);
        let stderr = String::from_utf8_lossy(&stderr_bytes);

        let mut result = format!("[forge_tool: {}]\n", description);
        if !stdout.is_empty() {
            result.push_str(&stdout);
        }
        if !stderr.is_empty() {
            result.push_str(&format!("[stderr] {}", stderr));
        }
        if !exit_status.success() {
            result.push_str(&format!("\n[exit code: {}]", exit_status.code().unwrap_or(-1)));
        }

        // Truncate long output (consistent with bash tool's 50KB limit)
        if result.len() > 50_000 {
            result.truncate(50_000);
            result.push_str("\n...[truncated]");
        }

        Ok(result)
    }
}
