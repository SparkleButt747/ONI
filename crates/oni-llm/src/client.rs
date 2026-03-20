use crate::models::*;
use oni_core::error::{err, Result, WrapErr};
use reqwest::Client;
use std::time::Duration;

pub struct LlmClient {
    http: Client,
    base_url: String,
}

impl LlmClient {
    pub fn new(base_url: &str, timeout_secs: u64) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/health", self.base_url);
        let resp = self
            .http
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .wrap_err("llama-server not running. Start with: oni-servers.sh start")?;
        Ok(resp.status().is_success())
    }

    /// Ping a specific URL's /health endpoint.
    pub async fn health_check_url(&self, url: &str) -> Result<bool> {
        let endpoint = format!("{}/health", url.trim_end_matches('/'));
        let resp = self
            .http
            .get(&endpoint)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .wrap_err("llama-server not reachable")?;
        Ok(resp.status().is_success())
    }

    pub async fn chat(&self, request: &ChatRequest) -> Result<ChatResponse> {
        self.chat_at_url(&self.base_url, request).await
    }

    /// Send a chat request to a specific base URL (for per-tier routing).
    pub async fn chat_at_url(&self, base_url: &str, request: &ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/v1/chat/completions", base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await
            .wrap_err("Failed to connect to llama-server")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(err!("llama-server API error ({}): {}", status, body));
        }

        resp.json()
            .await
            .wrap_err("Failed to parse llama-server response")
    }

    pub async fn embed(&self, request: &EmbedRequest) -> Result<EmbedResponse> {
        self.embed_at_url(&self.base_url, request).await
    }

    /// Send an embed request to a specific base URL (for per-tier routing).
    pub async fn embed_at_url(&self, base_url: &str, request: &EmbedRequest) -> Result<EmbedResponse> {
        let url = format!("{}/v1/embeddings", base_url.trim_end_matches('/'));
        let resp = self
            .http
            .post(&url)
            .json(request)
            .send()
            .await
            .wrap_err("Failed to connect to llama-server for embeddings")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(err!("llama-server embed error ({}): {}", status, body));
        }

        resp.json()
            .await
            .wrap_err("Failed to parse embed response")
    }

    pub async fn has_model(&self, _model_name: &str) -> Result<bool> {
        // llama-server loads one model per instance. If /health is up, the model is available.
        self.health_check().await
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new("http://localhost:8082", 300)
    }
}
