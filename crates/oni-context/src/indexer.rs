use crate::walker::{walk_project, DiscoveredFile};
use oni_core::error::{Result, WrapErr};
use regex::Regex;
use rusqlite::Connection;
use std::path::Path;

/// A symbol extracted from a source file.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: String, // "fn", "struct", "class", "def", etc.
    pub line: usize,
}

/// Extract top-level symbols from source text based on language.
pub fn extract_symbols(content: &str, language: &str) -> Vec<Symbol> {
    let patterns: &[(&str, &str)] = match language {
        "rust" => &[
            (r"(?m)^\s*(?:pub\s+)?fn\s+(\w+)", "fn"),
            (r"(?m)^\s*(?:pub\s+)?struct\s+(\w+)", "struct"),
            (r"(?m)^\s*(?:pub\s+)?enum\s+(\w+)", "enum"),
            (r"(?m)^\s*(?:pub\s+)?trait\s+(\w+)", "trait"),
            (r"(?m)^\s*(?:pub\s+)?impl(?:\s+\w+\s+for)?\s+(\w+)", "impl"),
            (r"(?m)^\s*(?:pub\s+)?type\s+(\w+)", "type"),
        ],
        "python" => &[
            (r"(?m)^def\s+(\w+)", "def"),
            (r"(?m)^class\s+(\w+)", "class"),
            (r"(?m)^    def\s+(\w+)", "method"),
        ],
        "typescript" | "javascript" => &[
            (r"(?m)^(?:export\s+)?(?:async\s+)?function\s+(\w+)", "function"),
            (r"(?m)^(?:export\s+)?class\s+(\w+)", "class"),
            (r"(?m)^(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(", "arrow"),
            (r"(?m)^(?:export\s+)?(?:const|let|var)\s+(\w+)", "const"),
            (r"(?m)^(?:export\s+)?(?:interface|type)\s+(\w+)", "type"),
        ],
        "go" => &[
            (r"(?m)^func\s+(?:\(\w+\s+\*?\w+\)\s+)?(\w+)", "func"),
            (r"(?m)^type\s+(\w+)\s+struct", "struct"),
            (r"(?m)^type\s+(\w+)\s+interface", "interface"),
            (r"(?m)^type\s+(\w+)", "type"),
        ],
        "java" | "csharp" | "kotlin" => &[
            (r"(?m)(?:public|private|protected|static|\s)+(?:class|interface|enum)\s+(\w+)", "class"),
            (r"(?m)(?:public|private|protected|static|\s)+\w+\s+(\w+)\s*\(", "method"),
        ],
        _ => &[],
    };

    let mut symbols = Vec::new();
    for (pattern, kind) in patterns {
        let Ok(re) = Regex::new(pattern) else { continue };
        for cap in re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                // Find line number by counting newlines up to the match start
                let line = content[..cap.get(0).unwrap().start()]
                    .chars()
                    .filter(|&c| c == '\n')
                    .count()
                    + 1;
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: kind.to_string(),
                    line,
                });
            }
        }
    }

    // Deduplicate by (name, kind)
    symbols.sort_by(|a, b| a.line.cmp(&b.line));
    symbols.dedup_by(|a, b| a.name == b.name && a.kind == b.kind);
    symbols
}

/// Initialise the FTS5 tables in the given SQLite connection.
pub fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS files (
            id      INTEGER PRIMARY KEY,
            path    TEXT NOT NULL UNIQUE,
            lang    TEXT NOT NULL,
            content TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS files_fts USING fts5(
            path,
            lang,
            content,
            content='files',
            content_rowid='id',
            tokenize='porter unicode61'
        );

        CREATE TABLE IF NOT EXISTS symbols (
            id      INTEGER PRIMARY KEY,
            file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
            name    TEXT NOT NULL,
            kind    TEXT NOT NULL,
            line    INTEGER NOT NULL
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS symbols_fts USING fts5(
            name,
            kind,
            content='symbols',
            content_rowid='id',
            tokenize='porter unicode61'
        );
        ",
    )
    .wrap_err("Failed to initialise context schema")
}

/// Index a single file into the database.
pub fn index_file(conn: &Connection, file: &DiscoveredFile, content: &str) -> Result<()> {
    // Upsert into files table
    conn.execute(
        "INSERT INTO files (path, lang, content)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET lang=excluded.lang, content=excluded.content",
        rusqlite::params![
            file.path.to_string_lossy().as_ref(),
            &file.language,
            content
        ],
    )
    .wrap_err("Failed to insert file record")?;

    let file_id: i64 = conn.query_row(
        "SELECT id FROM files WHERE path = ?1",
        [file.path.to_string_lossy().as_ref()],
        |row| row.get(0),
    )?;

    // Rebuild FTS entry for this file
    conn.execute(
        "INSERT OR REPLACE INTO files_fts(rowid, path, lang, content)
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            file_id,
            file.path.to_string_lossy().as_ref(),
            &file.language,
            content
        ],
    )
    .wrap_err("Failed to insert FTS record")?;

    // Replace symbols for this file
    conn.execute("DELETE FROM symbols WHERE file_id = ?1", [file_id])?;

    let symbols = extract_symbols(content, &file.language);
    for sym in &symbols {
        conn.execute(
            "INSERT INTO symbols (file_id, name, kind, line) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![file_id, &sym.name, &sym.kind, sym.line as i64],
        )
        .wrap_err("Failed to insert symbol")?;

        let sym_id: i64 = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO symbols_fts(rowid, name, kind) VALUES (?1, ?2, ?3)",
            rusqlite::params![sym_id, &sym.name, &sym.kind],
        )
        .wrap_err("Failed to insert symbol FTS record")?;
    }

    Ok(())
}

/// Walk a project directory and index all discovered files.
pub fn index_project(conn: &Connection, root: &Path) -> Result<usize> {
    init_schema(conn)?;

    let files = walk_project(root);
    let count = files.len();

    for file in &files {
        let content = match std::fs::read_to_string(&file.path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Skipping {:?}: {}", file.path, e);
                continue;
            }
        };
        if let Err(e) = index_file(conn, file, &content) {
            tracing::warn!("Failed to index {:?}: {}", file.path, e);
        }
    }

    tracing::info!("Indexed {} files from {:?}", count, root);
    Ok(count)
}

/// Incrementally re-index a single file. Called by the file watcher when
/// a file changes on disk. Deletes old entries and re-inserts.
pub fn index_single_file(conn: &Connection, path: &Path) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .wrap_err_with(|| format!("Failed to read {:?}", path))?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let language = match ext {
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "go" => "go",
        "java" => "java",
        _ => "unknown",
    };

    let path_str = path.to_string_lossy().to_string();

    // Delete old entries for this file
    let _ = conn.execute("DELETE FROM files WHERE path = ?1", [&path_str]);
    let _ = conn.execute(
        "DELETE FROM symbols WHERE file_id NOT IN (SELECT id FROM files)",
        [],
    );

    // Re-insert
    let file = DiscoveredFile {
        path: path.to_path_buf(),
        language: language.to_string(),
    };

    index_file(conn, &file, &content)?;
    Ok(())
}
