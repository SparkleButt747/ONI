use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;

pub struct EditFileTool;

fn is_safe_path(path: &str) -> bool {
    let p = std::path::Path::new(path);
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return false;
        }
    }
    true
}

impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::WriteFs]
    }

    fn description(&self) -> &str {
        "Edit a file by replacing specific text. More precise than write_file for small changes."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to edit"
                        },
                        "old_text": {
                            "type": "string",
                            "description": "Exact text to find and replace"
                        },
                        "new_text": {
                            "type": "string",
                            "description": "Replacement text"
                        }
                    },
                    "required": ["path", "old_text", "new_text"]
                }
            }
        })
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'path' argument"))?;
        let old_text = args
            .get("old_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'old_text' argument"))?;
        let new_text = args
            .get("new_text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| oni_core::error::err!("Missing 'new_text' argument"))?;

        if !is_safe_path(path) {
            return Ok(format!("Error: path '{}' is outside the project directory.", path));
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => return Ok(format!("Error reading file '{}': {}", path, e)),
        };

        let count = content.matches(old_text).count();
        if count == 0 {
            return Ok(format!(
                "Error: text not found in '{}'. No changes made.",
                path
            ));
        }
        if count > 1 {
            return Ok(format!(
                "Error: found {} occurrences of the text in '{}' — ambiguous. Provide more context to make it unique.",
                count, path
            ));
        }

        let updated = content.replacen(old_text, new_text, 1);

        if let Err(e) = std::fs::write(path, &updated) {
            return Ok(format!("Error writing file '{}': {}", path, e));
        }

        // Build a mini diff showing context
        let old_lines: Vec<&str> = old_text.lines().collect();
        let new_lines: Vec<&str> = new_text.lines().collect();
        let mut diff = format!("Edited '{}'\n", path);
        for line in &old_lines {
            diff.push_str(&format!("- {}\n", line));
        }
        for line in &new_lines {
            diff.push_str(&format!("+ {}\n", line));
        }

        Ok(diff)
    }
}
