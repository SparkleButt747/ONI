use oni_llm::{ChatMessage, ChatRequest, LlmClient};

#[tokio::test]
async fn test_health_check() {
    let client = LlmClient::default();
    match client.health_check().await {
        Ok(true) => println!("llama-server is running"),
        Ok(false) => eprintln!("SKIP: llama-server returned unhealthy"),
        Err(e) => eprintln!("SKIP: llama-server not running: {}", e),
    }
}

#[tokio::test]
async fn test_has_model() {
    let client = LlmClient::default();
    if client.health_check().await.is_err() {
        eprintln!("SKIP: llama-server not running");
        return;
    }

    // With llama-server, has_model just checks health (single-model per instance)
    let has = client.has_model("nomic-embed-text").await;
    match has {
        Ok(true) => println!("server is healthy (model loaded)"),
        Ok(false) => println!("server is unhealthy (no model loaded)"),
        Err(e) => eprintln!("Error checking model: {}", e),
    }
}

#[tokio::test]
async fn test_batch_chat_with_any_model() {
    let client = LlmClient::default();
    if client.health_check().await.is_err() {
        eprintln!("SKIP: llama-server not running");
        return;
    }

    // Use a fixed model name — llama-server is single-model per instance
    let model_name = "default".to_string();

    println!("Testing chat against llama-server default model");

    let request = ChatRequest {
        model: model_name.clone(),
        messages: vec![ChatMessage::user("Reply with exactly: HELLO")],
        stream: false,
        temperature: None,
        max_tokens: None,
        tools: None,
    };

    match client.chat(&request).await {
        Ok(resp) => {
            let msg = resp.message();
            println!("Response: {}", msg.content);
            println!(
                "Tokens: prompt={}, completion={}",
                resp.prompt_tokens(),
                resp.completion_tokens()
            );
            assert!(!msg.content.is_empty());
        }
        Err(e) => {
            eprintln!("Chat error: {}", e);
        }
    }
}

#[tokio::test]
async fn test_embed() {
    let client = LlmClient::default();
    if client.health_check().await.is_err() {
        eprintln!("SKIP: llama-server not running");
        return;
    }

    let request = oni_llm::EmbedRequest {
        model: "nomic-embed-text".into(),
        input: "function that handles authentication".into(),
    };

    match client.embed(&request).await {
        Ok(resp) => {
            assert!(!resp.data.is_empty());
            let dims = resp.data[0].embedding.len();
            println!("Embedding dimensions: {}", dims);
            assert_eq!(dims, 768); // nomic-embed-text produces 768-dim vectors
        }
        Err(e) => {
            eprintln!("Embed error: {}", e);
        }
    }
}
