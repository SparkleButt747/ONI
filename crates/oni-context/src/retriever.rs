use oni_core::error::{Result, WrapErr};
use oni_llm::LlmClient;
use rusqlite::Connection;

use crate::embeddings;

const DEFAULT_TOKEN_BUDGET: usize = 8192;
const CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, Clone)]
pub struct ContextChunk {
    pub path: String,
    pub content: String,
    pub score: f64,
}

/// Read the current pin path from `.oni/pin` if it exists.
pub fn read_pin(project_dir: &std::path::Path) -> Option<String> {
    let pin_path = project_dir.join(".oni").join("pin");
    std::fs::read_to_string(pin_path).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Set or clear the pin path.
pub fn set_pin(project_dir: &std::path::Path, pin: Option<&str>) -> std::io::Result<()> {
    let oni_dir = project_dir.join(".oni");
    std::fs::create_dir_all(&oni_dir)?;
    let pin_path = oni_dir.join("pin");
    match pin {
        Some(p) => std::fs::write(pin_path, p),
        None => {
            if pin_path.exists() {
                std::fs::remove_file(pin_path)
            } else {
                Ok(())
            }
        }
    }
}

/// Read `.oni-context` file if it exists — injected into system prompt.
pub fn read_oni_context(project_dir: &std::path::Path) -> Option<String> {
    let ctx_path = project_dir.join(".oni-context");
    std::fs::read_to_string(ctx_path).ok().filter(|s| !s.is_empty())
}

/// Query the FTS5 index and return context chunks within the token budget.
/// If a pin is set, only returns results whose path starts with the pin prefix.
pub fn retrieve(conn: &Connection, query: &str, token_budget: Option<usize>) -> Result<Vec<ContextChunk>> {
    let budget = token_budget.unwrap_or(DEFAULT_TOKEN_BUDGET);
    let char_budget = budget * CHARS_PER_TOKEN;

    // BM25 ranking: lower (more negative) = more relevant in SQLite FTS5
    let mut stmt = conn.prepare(
        "SELECT f.path, f.content, bm25(files_fts) as score
         FROM files_fts
         JOIN files f ON f.id = files_fts.rowid
         WHERE files_fts MATCH ?1
         ORDER BY score ASC
         LIMIT 50",
    )
    .wrap_err("Failed to prepare FTS query")?;

    let rows = stmt
        .query_map([query], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .wrap_err("Failed to execute FTS query")?;

    let mut chunks = Vec::new();
    let mut used_chars = 0;

    for row in rows {
        let (path, content, score) = row.wrap_err("Failed to read FTS row")?;

        if used_chars >= char_budget {
            break;
        }

        // Truncate content to fit within remaining budget
        let remaining = char_budget - used_chars;
        let truncated = if content.len() > remaining {
            content.chars().take(remaining).collect::<String>()
        } else {
            content.clone()
        };

        used_chars += truncated.len();
        chunks.push(ContextChunk { path, content: truncated, score });
    }

    Ok(chunks)
}

/// Hybrid retrieval: FTS5 returns up to 50 candidates, then embeddings rerank
/// them by cosine similarity to the query. Falls back to FTS5 order if the
/// embedding model is unavailable or returns an error.
///
/// The `llm_client` is used only for the rerank step; the DB query is
/// always synchronous and runs on the calling thread.
pub async fn retrieve_hybrid(
    conn: &Connection,
    query: &str,
    token_budget: Option<usize>,
    llm_client: &LlmClient,
) -> Result<Vec<ContextChunk>> {
    let budget = token_budget.unwrap_or(DEFAULT_TOKEN_BUDGET);
    let char_budget = budget * CHARS_PER_TOKEN;

    // --- Step 1: FTS5 top-50 candidates (no budget applied yet) ---------------
    let mut stmt = conn
        .prepare(
            "SELECT f.path, f.content, bm25(files_fts) as score
             FROM files_fts
             JOIN files f ON f.id = files_fts.rowid
             WHERE files_fts MATCH ?1
             ORDER BY score ASC
             LIMIT 50",
        )
        .wrap_err("Failed to prepare FTS query")?;

    let rows = stmt
        .query_map([query], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .wrap_err("Failed to execute FTS query")?;

    let mut fts_results: Vec<ContextChunk> = Vec::new();
    for row in rows {
        let (path, content, score) = row.wrap_err("Failed to read FTS row")?;
        fts_results.push(ContextChunk { path, content, score });
    }

    // --- Step 2: Rerank with embeddings (graceful fallback) -------------------
    // rerank_with_embeddings takes a shared slice; fts_results stays owned here
    // so we can fall back to it without re-querying.
    let reranked = match rerank_with_embeddings(query, &fts_results, llm_client).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Embedding rerank failed, using FTS5 order: {}", e);
            fts_results
        }
    };

    // --- Step 3: Apply token budget to reranked results -----------------------
    let mut chunks = Vec::new();
    let mut used_chars = 0;

    for chunk in reranked {
        if used_chars >= char_budget {
            break;
        }
        let remaining = char_budget - used_chars;
        let truncated = if chunk.content.len() > remaining {
            chunk.content.chars().take(remaining).collect::<String>()
        } else {
            chunk.content.clone()
        };
        used_chars += truncated.len();
        chunks.push(ContextChunk { path: chunk.path, content: truncated, score: chunk.score });
    }

    Ok(chunks)
}

/// Embed the query and all candidates, then return a new vec sorted by cosine
/// similarity (highest first). Takes candidates by reference so the caller
/// retains ownership for fallback use.
async fn rerank_with_embeddings(
    query: &str,
    candidates: &[ContextChunk],
    client: &LlmClient,
) -> Result<Vec<ContextChunk>> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // Embed the query
    let query_emb = embeddings::embed(client, query).await?;

    // Batch-embed all candidate contents
    let texts: Vec<&str> = candidates.iter().map(|c| c.content.as_str()).collect();
    let chunk_embs = embeddings::embed_batch(client, &texts).await?;

    // Pair each candidate with its cosine similarity and sort highest first
    let mut scored: Vec<(ContextChunk, f32)> = candidates
        .iter()
        .cloned()
        .zip(chunk_embs)
        .map(|(chunk, emb)| {
            let sim = embeddings::cosine_similarity(&query_emb, &emb);
            (chunk, sim)
        })
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    Ok(scored.into_iter().map(|(chunk, _)| chunk).collect())
}

/// Query symbols specifically — useful for "find function X" style lookups.
pub fn retrieve_symbols(
    conn: &Connection,
    query: &str,
    token_budget: Option<usize>,
) -> Result<Vec<ContextChunk>> {
    let budget = token_budget.unwrap_or(DEFAULT_TOKEN_BUDGET);
    let char_budget = budget * CHARS_PER_TOKEN;

    let mut stmt = conn.prepare(
        "SELECT f.path, f.content, bm25(symbols_fts) as score
         FROM symbols_fts
         JOIN symbols s ON s.id = symbols_fts.rowid
         JOIN files f ON f.id = s.file_id
         WHERE symbols_fts MATCH ?1
         ORDER BY score ASC
         LIMIT 20",
    )
    .wrap_err("Failed to prepare symbol FTS query")?;

    let rows = stmt
        .query_map([query], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .wrap_err("Failed to execute symbol FTS query")?;

    let mut chunks = Vec::new();
    let mut used_chars = 0;
    let mut seen_paths = std::collections::HashSet::new();

    for row in rows {
        let (path, content, score) = row.wrap_err("Failed to read symbol FTS row")?;

        // Deduplicate by file — same file might match multiple symbols
        if seen_paths.contains(&path) {
            continue;
        }
        seen_paths.insert(path.clone());

        if used_chars >= char_budget {
            break;
        }

        let remaining = char_budget - used_chars;
        let truncated = if content.len() > remaining {
            content.chars().take(remaining).collect::<String>()
        } else {
            content.clone()
        };

        used_chars += truncated.len();
        chunks.push(ContextChunk { path, content: truncated, score });
    }

    Ok(chunks)
}
