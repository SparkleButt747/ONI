use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;

pub struct ReadFileTool;

impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::ReadFs]
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Absolute or relative path to the file"
                        }
                    },
                    "required": ["path"]
                }
            }
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'path' argument"))?;

        match std::fs::read_to_string(path) {
            Ok(content) => {
                // Truncate very large files
                if content.len() > 100_000 {
                    let end = {
                        let mut e = 100_000_usize.min(content.len());
                        while e > 0 && !content.is_char_boundary(e) { e -= 1; }
                        e
                    };
                    Ok(format!(
                        "{}...\n\n[Truncated: file is {} bytes]",
                        &content[..end],
                        content.len()
                    ))
                } else {
                    Ok(content)
                }
            }
            Err(e) => Ok(format!("Error reading file '{}': {}", path, e)),
        }
    }
}
