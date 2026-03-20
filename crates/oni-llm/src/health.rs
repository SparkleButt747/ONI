use crate::client::LlmClient;
use oni_core::types::ModelTier;
use std::collections::HashMap;

pub struct HealthReport {
    pub server_running: bool,
    pub models: HashMap<String, bool>,
}

pub async fn check_health(
    client: &LlmClient,
    tier_urls: &HashMap<ModelTier, String>,
) -> HealthReport {
    let mut any_running = false;
    let mut results = HashMap::new();

    for (tier, url) in tier_urls {
        let up = client
            .health_check_url(url)
            .await
            .unwrap_or(false);
        if up {
            any_running = true;
        }
        results.insert(format!("{}: {}", tier, url), up);
    }

    HealthReport {
        server_running: any_running,
        models: results,
    }
}
