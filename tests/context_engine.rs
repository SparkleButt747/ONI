use oni_context::{
    indexer::{extract_symbols, index_project},
    retriever::{retrieve, retrieve_hybrid, retrieve_symbols},
};
use oni_llm::LlmClient;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

// ── Symbol extraction ─────────────────────────────────────────────────────────

#[test]
/// T-CTX-1: extract_symbols finds pub fn, struct, and enum items in Rust source.
fn t_ctx_1_extract_symbols_rust() {
    let src = r#"
pub fn alpha() {}
pub struct Beta {}
pub enum Gamma { A, B }
fn private_fn() {}
"#;
    let symbols = extract_symbols(src, "rust");
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"alpha"), "missing alpha: {names:?}");
    assert!(names.contains(&"Beta"), "missing Beta: {names:?}");
    assert!(names.contains(&"Gamma"), "missing Gamma: {names:?}");
    assert!(names.contains(&"private_fn"), "missing private_fn: {names:?}");
}

#[test]
/// T-CTX-2: extract_symbols assigns correct kinds to Rust items.
fn t_ctx_2_extract_symbols_rust_kinds() {
    let src = "pub fn my_fn() {}\npub struct MyStruct {}\npub trait MyTrait {}\n";
    let symbols = extract_symbols(src, "rust");
    let fn_sym = symbols.iter().find(|s| s.name == "my_fn").unwrap();
    let struct_sym = symbols.iter().find(|s| s.name == "MyStruct").unwrap();
    let trait_sym = symbols.iter().find(|s| s.name == "MyTrait").unwrap();
    assert_eq!(fn_sym.kind, "fn");
    assert_eq!(struct_sym.kind, "struct");
    assert_eq!(trait_sym.kind, "trait");
}

#[test]
/// T-CTX-3: extract_symbols finds def and class items in Python source.
fn t_ctx_3_extract_symbols_python() {
    let src = "def compute():\n    pass\n\nclass Engine:\n    def run(self):\n        pass\n";
    let symbols = extract_symbols(src, "python");
    let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"compute"), "missing compute: {names:?}");
    assert!(names.contains(&"Engine"), "missing Engine: {names:?}");
}

#[test]
/// T-CTX-4: extract_symbols returns an empty vec for an unknown language.
fn t_ctx_4_extract_symbols_unknown_lang() {
    let symbols = extract_symbols("some text", "brainfuck");
    assert!(symbols.is_empty());
}

#[test]
/// T-CTX-5: extract_symbols records correct 1-based line numbers.
fn t_ctx_5_extract_symbols_line_numbers() {
    let src = "// comment\npub fn first() {}\n\npub fn second() {}\n";
    let symbols = extract_symbols(src, "rust");
    let first = symbols.iter().find(|s| s.name == "first").unwrap();
    let second = symbols.iter().find(|s| s.name == "second").unwrap();
    assert_eq!(first.line, 2, "first should be on line 2");
    assert!(second.line > first.line, "second should come after first");
}

// ── Indexing ──────────────────────────────────────────────────────────────────

fn make_in_memory_conn() -> Connection {
    Connection::open_in_memory().unwrap()
}

#[test]
/// T-CTX-6: index_project returns the correct file count for a directory with known files.
fn t_ctx_6_index_project_file_count() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("lib.rs"), "pub fn hello() {}").unwrap();
    fs::write(dir.path().join("utils.py"), "def helper(): pass").unwrap();
    // non-indexed extension — should not be counted
    fs::write(dir.path().join("ignored.xyz"), "data").unwrap();

    let conn = make_in_memory_conn();
    let count = index_project(&conn, dir.path()).unwrap();
    assert_eq!(count, 2, "expected 2 indexed files, got {count}");
}

#[test]
/// T-CTX-7: after index_project the files table contains the indexed paths.
fn t_ctx_7_index_project_stores_paths() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    let path_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))
        .unwrap();
    assert_eq!(path_count, 1);
}

#[test]
/// T-CTX-8: after index_project the symbols table is populated for Rust sources.
fn t_ctx_8_index_project_populates_symbols() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("agent.rs"),
        "pub fn run() {}\npub struct Agent {}\n",
    )
    .unwrap();

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    let sym_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM symbols", [], |r| r.get(0))
        .unwrap();
    assert!(sym_count >= 2, "expected at least 2 symbols, got {sym_count}");
}

#[test]
/// T-CTX-9: index_project on an empty directory succeeds with a count of zero.
fn t_ctx_9_index_empty_dir() {
    let dir = TempDir::new().unwrap();
    let conn = make_in_memory_conn();
    let count = index_project(&conn, dir.path()).unwrap();
    assert_eq!(count, 0);
}

// ── Retrieval ─────────────────────────────────────────────────────────────────

#[test]
/// T-CTX-10: retrieve returns matching chunks when the query term appears in indexed content.
fn t_ctx_10_retrieve_returns_results() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("context.rs"),
        "pub fn indexer_entry_point() -> Vec<String> { vec![] }",
    )
    .unwrap();

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    let chunks = retrieve(&conn, "indexer_entry_point", None).unwrap();
    assert!(!chunks.is_empty(), "expected at least one chunk back");
    assert!(chunks[0].content.contains("indexer_entry_point"));
}

#[test]
/// T-CTX-11: retrieve returns an empty vec (not an error) when no files match the query.
fn t_ctx_11_retrieve_no_match_returns_empty() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("code.rs"), "fn something() {}").unwrap();

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    let chunks = retrieve(&conn, "ZZZZ_THIS_WONT_MATCH_ANYTHING", None).unwrap();
    assert!(chunks.is_empty());
}

#[test]
/// T-CTX-12: retrieve_symbols returns results scoped to files containing the named symbol.
fn t_ctx_12_retrieve_symbols_finds_symbol() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("lib.rs"),
        "pub struct Orchestrator {}\npub fn dispatch() {}",
    )
    .unwrap();

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    let chunks = retrieve_symbols(&conn, "Orchestrator", None).unwrap();
    assert!(!chunks.is_empty(), "expected symbol hit");
}

#[test]
/// T-CTX-13: retrieve respects the token_budget — total chars returned stay within 4x the budget.
fn t_ctx_13_retrieve_respects_token_budget() {
    let dir = TempDir::new().unwrap();
    // Write a file large enough that the budget would constrain it
    let content = "token ".repeat(5_000); // 30 000 chars
    fs::write(dir.path().join("big.rs"), format!("fn placeholder() {{ /* {} */ }}", content)).unwrap();

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    let budget = 100; // tiny budget = 400 chars
    let chunks = retrieve(&conn, "placeholder", Some(budget)).unwrap();
    let total_chars: usize = chunks.iter().map(|c| c.content.len()).sum();
    assert!(
        total_chars <= budget * 4,
        "total chars {total_chars} exceeded budget {} chars",
        budget * 4
    );
}

// ── Hybrid retrieval ──────────────────────────────────────────────────────────

/// Returns true if nomic-embed-text is available via a local Ollama instance.
async fn ollama_embed_available() -> bool {
    let client = LlmClient::new("http://localhost:11434", 5);
    match client.has_model("nomic-embed-text").await {
        Ok(present) => present,
        Err(_) => false,
    }
}

#[tokio::test]
/// T-CTX-14: retrieve_hybrid returns results when Ollama + nomic-embed-text are available;
/// skips gracefully when they are not.
async fn t_ctx_14_hybrid_retrieve_returns_results() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("agent.rs"),
        "pub fn orchestrator_run() -> Vec<String> { vec![] }",
    )
    .unwrap();

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    let client = LlmClient::new("http://localhost:11434", 30);
    let chunks = retrieve_hybrid(&conn, "orchestrator_run", None, &client).await.unwrap();

    if ollama_embed_available().await {
        assert!(!chunks.is_empty(), "expected at least one hybrid result");
        assert!(chunks[0].content.contains("orchestrator_run"));
    } else {
        // Fallback path — FTS5 order is used; result must still be valid (no error)
        let _ = chunks;
    }
}

#[tokio::test]
/// T-CTX-15: retrieve_hybrid falls back to FTS5 order when embeddings are unavailable
/// (Ollama unreachable). Must return results and not error out.
async fn t_ctx_15_hybrid_fallback_when_embeddings_unavailable() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join("lib.rs"),
        "pub fn fallback_function() -> bool { true }",
    )
    .unwrap();

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    // Point to a port that won't respond — forces the embed call to fail
    let bad_client = LlmClient::new("http://localhost:19999", 2);
    let chunks = retrieve_hybrid(&conn, "fallback_function", None, &bad_client).await.unwrap();

    // FTS5 matched, so we should still get results via the fallback path
    assert!(!chunks.is_empty(), "fallback should return FTS5 results");
    assert!(chunks[0].content.contains("fallback_function"));
}

#[tokio::test]
/// T-CTX-16: retrieve_hybrid preserves all FTS5 candidates — reranking only reorders,
/// never drops chunks (before the token budget is applied).
async fn t_ctx_16_hybrid_preserves_candidate_count() {
    let dir = TempDir::new().unwrap();
    // Write several files all containing the search term so FTS5 returns multiple hits
    for i in 0..5 {
        fs::write(
            dir.path().join(format!("file{i}.rs")),
            format!("pub fn widget_{i}_render() {{ /* widget body */ }}"),
        )
        .unwrap();
    }

    let conn = make_in_memory_conn();
    index_project(&conn, dir.path()).unwrap();

    // Use a large token budget so truncation doesn't drop anything
    let budget = Some(100_000);

    // Sync FTS5 baseline count
    let fts_chunks = retrieve(&conn, "widget", budget).unwrap();

    // Hybrid with unreachable Ollama — falls back to FTS5 order
    let bad_client = LlmClient::new("http://localhost:19999", 2);
    let hybrid_chunks =
        retrieve_hybrid(&conn, "widget", budget, &bad_client).await.unwrap();

    assert_eq!(
        fts_chunks.len(),
        hybrid_chunks.len(),
        "hybrid fallback must return the same number of chunks as plain FTS5"
    );
}
