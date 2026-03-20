use crate::client::LlmClient;
use crate::models::*;
use oni_core::config::{ModelConfig, ReasoningConfig};
use oni_core::error::Result;
use oni_core::types::ModelTier;
use std::collections::HashMap;

pub struct ModelRouter {
    client: LlmClient,
    models: ModelConfig,
    /// Per-tier base URLs (e.g. Heavy -> "http://localhost:8081").
    tier_urls: HashMap<ModelTier, String>,
    /// Global reasoning config — provides temperature/max_tokens
    /// fallbacks when per-tier overrides are absent.
    reasoning: ReasoningConfig,
}

impl ModelRouter {
    pub fn new(client: LlmClient, models: ModelConfig) -> Self {
        Self {
            client,
            models,
            tier_urls: HashMap::new(),
            reasoning: ReasoningConfig::default(),
        }
    }

    pub fn new_with_tier_urls(
        client: LlmClient,
        models: ModelConfig,
        tier_urls: HashMap<ModelTier, String>,
    ) -> Self {
        Self {
            client,
            models,
            tier_urls,
            reasoning: ReasoningConfig::default(),
        }
    }

    pub fn with_reasoning(mut self, reasoning: ReasoningConfig) -> Self {
        self.reasoning = reasoning;
        self
    }

    pub fn model_name(&self, tier: ModelTier) -> &str {
        self.models.model_for_tier(tier)
    }

    /// Resolve the base URL for a given tier.
    fn url_for_tier(&self, tier: ModelTier) -> &str {
        self.tier_urls
            .get(&tier)
            .map(|s| s.as_str())
            .unwrap_or_else(|| self.client.base_url())
    }

    /// Resolve temperature for a tier (per-tier > global > hardcoded default).
    fn temperature_for_tier(&self, tier: ModelTier) -> f32 {
        let (default_temp, _) = Self::tier_defaults(tier);
        let tier_cfg = self.models.tier_reasoning(tier);
        tier_cfg
            .temperature
            .or(self.reasoning.temperature)
            .unwrap_or(default_temp)
    }

    /// Resolve max_tokens for a tier (per-tier > global > hardcoded default).
    fn max_tokens_for_tier(&self, tier: ModelTier) -> u32 {
        let (_, default_ctx) = Self::tier_defaults(tier);
        let tier_cfg = self.models.tier_reasoning(tier);
        tier_cfg
            .num_ctx
            .or(self.reasoning.num_ctx)
            .unwrap_or(default_ctx)
    }

    /// Hardcoded fallback defaults per tier.
    fn tier_defaults(tier: ModelTier) -> (f32, u32) {
        match tier {
            ModelTier::Heavy => (0.3, 32768),
            ModelTier::Medium => (0.2, 32768),
            ModelTier::General => (0.3, 16384),
            ModelTier::Fast => (0.1, 8192),
            ModelTier::Embed => (0.0, 0),
        }
    }

    pub async fn chat(
        &self,
        tier: ModelTier,
        messages: Vec<ChatMessage>,
    ) -> Result<ChatResponse> {
        let url = self.url_for_tier(tier);
        let (temperature, max_tokens) = if tier == ModelTier::Embed {
            (None, None)
        } else {
            (Some(self.temperature_for_tier(tier)), Some(self.max_tokens_for_tier(tier)))
        };
        let request = ChatRequest {
            model: self.model_name(tier).to_string(),
            messages,
            stream: false,
            temperature,
            max_tokens,
            tools: None,
        };
        self.client.chat_at_url(url, &request).await
    }

    /// Chat with tool definitions — enables native tool calling
    pub async fn chat_with_tools(
        &self,
        tier: ModelTier,
        messages: Vec<ChatMessage>,
        tools: Vec<serde_json::Value>,
    ) -> Result<ChatResponse> {
        let url = self.url_for_tier(tier);
        let (temperature, max_tokens) = if tier == ModelTier::Embed {
            (None, None)
        } else {
            (Some(self.temperature_for_tier(tier)), Some(self.max_tokens_for_tier(tier)))
        };
        let request = ChatRequest {
            model: self.model_name(tier).to_string(),
            messages,
            stream: false,
            temperature,
            max_tokens,
            tools: if tools.is_empty() { None } else { Some(tools) },
        };
        self.client.chat_at_url(url, &request).await
    }

    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let url = self.url_for_tier(ModelTier::Embed);
        let request = EmbedRequest {
            model: self.model_name(ModelTier::Embed).to_string(),
            input: text.to_string(),
        };
        let resp = self.client.embed_at_url(url, &request).await?;
        Ok(resp.data.into_iter().next().map(|o| o.embedding).unwrap_or_default())
    }

    /// Check health of each tier by pinging its /health endpoint.
    pub async fn check_all_models(&self) -> HashMap<ModelTier, bool> {
        let mut results = HashMap::new();
        for tier in [
            ModelTier::Heavy,
            ModelTier::Medium,
            ModelTier::General,
            ModelTier::Fast,
            ModelTier::Embed,
        ] {
            let url = self.url_for_tier(tier);
            let available = self
                .client
                .health_check_url(url)
                .await
                .unwrap_or(false);
            results.insert(tier, available);
        }
        results
    }

    pub fn client(&self) -> &LlmClient {
        &self.client
    }
}
