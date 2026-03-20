use oni_llm::{ChatMessage, ToolCall};

pub struct Conversation {
    system_prompt: String,
    messages: Vec<ChatMessage>,
}

impl Conversation {
    pub fn new(system_prompt: String) -> Self {
        Self {
            system_prompt,
            messages: Vec::new(),
        }
    }

    /// Replace the system prompt (used to inject per-turn context).
    pub fn update_system(&mut self, prompt: String) {
        self.system_prompt = prompt;
    }

    pub fn add_user(&mut self, content: &str) {
        self.messages.push(ChatMessage::user(content));
    }

    pub fn add_assistant(&mut self, content: &str) {
        self.messages.push(ChatMessage::assistant(content));
    }

    /// Add an assistant message that includes tool calls
    pub fn add_assistant_with_tool_calls(&mut self, content: &str, tool_calls: Vec<ToolCall>) {
        self.messages
            .push(ChatMessage::assistant_with_tool_calls(content, tool_calls));
    }

    pub fn add_tool_result(&mut self, content: &str) {
        self.messages.push(ChatMessage::tool(content));
    }

    /// Build the full message list for an Ollama API call
    pub fn to_messages(&self) -> Vec<ChatMessage> {
        let mut msgs = vec![ChatMessage::system(&self.system_prompt)];
        msgs.extend(self.messages.clone());
        msgs
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Estimate total token usage: ~4 chars per token.
    pub fn estimated_tokens(&self) -> u64 {
        let sys_tokens = self.system_prompt.len() as u64 / 4;
        let msg_tokens: u64 = self
            .messages
            .iter()
            .map(|m| {
                let content_tokens = m.content.len() as u64 / 4;
                let tool_tokens = m
                    .tool_calls
                    .as_ref()
                    .map(|tc| serde_json::to_string(tc).unwrap_or_default().len() as u64 / 4)
                    .unwrap_or(0);
                content_tokens + tool_tokens
            })
            .sum();
        sys_tokens + msg_tokens
    }

    /// Compact conversation history when exceeding a token budget.
    /// Keeps the system prompt, the last `keep_recent` messages intact,
    /// and replaces everything before them with a compressed summary.
    pub fn compact(&mut self, summary: &str, keep_recent: usize) {
        if self.messages.len() <= keep_recent {
            return;
        }
        let cut = self.messages.len() - keep_recent;
        let recent: Vec<ChatMessage> = self.messages[cut..].to_vec();
        self.messages.clear();
        // Insert the compaction digest at index 0 as a user message so it
        // doesn't violate Ollama's single-system-message-at-position-0 rule.
        self.messages
            .push(ChatMessage::user(&format!("[Context summary: {}]", summary)));
        self.messages.extend(recent);
    }
}
