use super::Tool;
use oni_core::error::Result;
use oni_core::types::ToolCapability;
use std::fs;

pub struct ListDirTool;

impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn required_capabilities(&self) -> &[ToolCapability] {
        &[ToolCapability::ReadFs]
    }

    fn description(&self) -> &str {
        "List files and directories at the given path"
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "list_directory",
                "description": self.description(),
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to list"
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

        match fs::read_dir(path) {
            Ok(entries) => {
                let mut items: Vec<String> = Vec::new();
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let file_type = entry.file_type().ok();
                    let prefix = match file_type {
                        Some(ft) if ft.is_dir() => "d ",
                        Some(ft) if ft.is_symlink() => "l ",
                        _ => "f ",
                    };
                    items.push(format!("{}{}", prefix, name));
                }
                items.sort();
                Ok(items.join("\n"))
            }
            Err(e) => Ok(format!("Error listing '{}': {}", path, e)),
        }
    }
}
