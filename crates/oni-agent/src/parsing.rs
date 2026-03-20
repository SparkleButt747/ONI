/// Shared parsing utilities used by both `agent` and `orchestrator`.

pub(crate) struct TextToolCall {
    pub(crate) name: String,
    pub(crate) args: serde_json::Value,
}

/// Strip `<think>...</think>` blocks from model output.
/// Uses the fixed algorithm that searches from `result[start..]` to avoid
/// byte-index confusion when the same string is searched repeatedly.
pub(crate) fn strip_thinking(content: &str) -> String {
    let mut result = content.to_string();
    loop {
        let Some(start) = result.find("<think>") else { break };
        if let Some(relative_end) = result[start..].find("</think>") {
            let end = start + relative_end;
            result = format!("{}{}", &result[..start], &result[end + 8..]);
        } else {
            result = result[..start].to_string();
            break;
        }
    }
    result.trim().to_string()
}

/// Parse text-based tool calls from model output.
/// Handles multiple formats:
/// 1. `<function=tool_name><parameter=key>value</parameter></function>` (Qwen-style)
/// 2. ` ```json {"tool": "name", "args": {...}} ``` ` (markdown JSON)
/// 3. `{"name": "tool_name", "arguments": {...}}` (direct JSON)
pub(crate) fn extract_text_tool_call(content: &str) -> Option<TextToolCall> {
    // Format 1: <function=tool_name> XML-style
    if let Some(func_start) = content.find("<function=") {
        let after = &content[func_start + 10..];
        let name_end = after.find('>')?;
        let name = after[..name_end].trim().to_string();

        let mut args = serde_json::Map::new();

        let mut search = &content[func_start..];
        while let Some(param_start) = search.find("<parameter=") {
            let after_param = &search[param_start + 11..];
            let key_end = after_param.find('>')?;
            let key = after_param[..key_end].trim().to_string();

            let value_start = key_end + 1;
            let value_end = after_param.find("</parameter>")?;
            let value = after_param[value_start..value_end].trim().to_string();

            args.insert(key, serde_json::Value::String(value));
            search = &after_param[value_end + 12..];
        }

        if !args.is_empty() {
            return Some(TextToolCall {
                name,
                args: serde_json::Value::Object(args),
            });
        }
    }

    // Format 2: ```json {"tool": "name", "args": {...}} ```
    if let Some(json_start) = content.find("```json") {
        let json_content_start = json_start + 7;
        if let Some(json_end) = content[json_content_start..].find("```") {
            let json_str = content[json_content_start..json_content_start + json_end].trim();
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(name) = parsed.get("tool").and_then(|v| v.as_str()) {
                    let args = parsed.get("args").cloned().unwrap_or(serde_json::json!({}));
                    return Some(TextToolCall {
                        name: name.to_string(),
                        args,
                    });
                }
            }
        }
    }

    // Format 3: {"name": "tool_name", "arguments": {...}} direct JSON
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let Some(name) = parsed.get("name").and_then(|v| v.as_str()) {
                let args = parsed.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                return Some(TextToolCall {
                    name: name.to_string(),
                    args,
                });
            }
        }
    }

    None
}
