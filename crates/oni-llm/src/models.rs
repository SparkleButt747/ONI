use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call type — always "function" for OpenAI-compatible APIs.
    #[serde(rename = "type", default = "default_tool_call_type")]
    pub type_: String,
    pub function: ToolCallFunction,
    /// Tool call ID — required by llama-server v8420+.
    #[serde(default)]
    pub id: Option<String>,
}

fn default_tool_call_type() -> String {
    "function".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    #[serde(deserialize_with = "deserialize_arguments")]
    pub arguments: serde_json::Value,
}

/// Deserialize arguments: OpenAI-compatible APIs may return arguments as either
/// a JSON string or a parsed object. Normalize to always be a parsed object.
fn deserialize_arguments<'de, D>(deserializer: D) -> std::result::Result<serde_json::Value, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match &value {
        serde_json::Value::String(s) => {
            Ok(serde_json::from_str(s).unwrap_or(value))
        }
        _ => Ok(value),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub id: Option<String>,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<UsageStats>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageStats {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: Option<u64>,
}

impl ChatResponse {
    /// Backward-compat accessor: get the first choice's message.
    pub fn message(&self) -> &ResponseMessage {
        &self.choices[0].message
    }

    pub fn prompt_tokens(&self) -> u64 {
        self.usage.as_ref().map_or(0, |u| u.prompt_tokens)
    }

    pub fn completion_tokens(&self) -> u64 {
        self.usage.as_ref().map_or(0, |u| u.completion_tokens)
    }
}

/// Response message — separate from ChatMessage because the server returns
/// tool_calls at the message level in responses.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMessage {
    pub role: String,
    #[serde(default, deserialize_with = "deserialize_nullable_string")]
    pub content: String,
    #[serde(default)]
    pub tool_calls: Option<Vec<ToolCall>>,
}

fn deserialize_nullable_string<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<String> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Clone, Serialize)]
pub struct EmbedRequest {
    pub model: String,
    pub input: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbedResponse {
    pub data: Vec<EmbeddingObject>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingObject {
    pub embedding: Vec<f32>,
    pub index: Option<usize>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            tool_calls: None,
        }
    }

    /// Create an assistant message that contains tool calls (for replaying in history)
    pub fn assistant_with_tool_calls(content: &str, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            tool_calls: Some(tool_calls),
        }
    }

    /// Create a tool result message
    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            tool_calls: None,
        }
    }
}

impl ResponseMessage {
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls
            .as_ref()
            .map_or(false, |tc| !tc.is_empty())
    }
}
