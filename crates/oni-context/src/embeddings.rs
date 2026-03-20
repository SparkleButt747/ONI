use oni_core::error::{Result, WrapErr};
use oni_llm::{EmbedRequest, LlmClient};

const EMBED_MODEL: &str = "nomic-embed-text";

/// Embed a single text string using Ollama's nomic-embed-text model.
/// Returns the first embedding vector from the response.
pub async fn embed(client: &LlmClient, text: &str) -> Result<Vec<f32>> {
    let request = EmbedRequest {
        model: EMBED_MODEL.to_string(),
        input: text.to_string(),
    };

    let response = client.embed(&request).await.wrap_err("Embedding request failed")?;

    response
        .data
        .into_iter()
        .next()
        .map(|e| e.embedding)
        .ok_or_else(|| oni_core::error::err!("llama-server returned empty embeddings"))
}

/// Embed multiple texts in sequence. Returns one vector per input.
pub async fn embed_batch(client: &LlmClient, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    let mut results = Vec::with_capacity(texts.len());
    for text in texts {
        results.push(embed(client, text).await?);
    }
    Ok(results)
}

/// Cosine similarity between two embedding vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Embedding dimension mismatch");
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
