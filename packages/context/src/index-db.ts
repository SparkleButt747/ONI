import Database from "better-sqlite3";
import { mkdirSync } from "node:fs";
import { dirname } from "node:path";

export interface FileRow {
  id: number;
  path: string;
  lang: string;
  hash: string;
  last_indexed: string;
  token_count: number;
}

export interface SymbolRow {
  id: number;
  name: string;
  kind: string;
  file_id: number;
  start_line: number;
  end_line: number;
  signature: string;
}

export interface ChunkMatch {
  path: string;
  content: string;
  rank: number;
}

const SCHEMA = `
  CREATE TABLE IF NOT EXISTS files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT UNIQUE NOT NULL,
    lang TEXT NOT NULL DEFAULT '',
    hash TEXT NOT NULL,
    last_indexed TEXT NOT NULL DEFAULT (datetime('now')),
    token_count INTEGER NOT NULL DEFAULT 0
  );

  CREATE TABLE IF NOT EXISTS symbols (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    start_line INTEGER NOT NULL,
    end_line INTEGER NOT NULL,
    signature TEXT NOT NULL DEFAULT ''
  );

  CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_id);
  CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);

  CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(
    path,
    content,
    tokenize='porter unicode61'
  );
`;

export function createIndexDb(dbPath: string): Database.Database {
  mkdirSync(dirname(dbPath), { recursive: true });
  const db = new Database(dbPath);
  db.pragma("journal_mode = WAL");
  db.pragma("foreign_keys = ON");
  db.exec(SCHEMA);
  return db;
}

export function upsertFile(
  db: Database.Database,
  path: string,
  lang: string,
  hash: string,
  tokenCount: number,
): number {
  const stmt = db.prepare(`
    INSERT INTO files (path, lang, hash, token_count, last_indexed)
    VALUES (?, ?, ?, ?, datetime('now'))
    ON CONFLICT(path) DO UPDATE SET
      lang = excluded.lang,
      hash = excluded.hash,
      token_count = excluded.token_count,
      last_indexed = datetime('now')
  `);
  const result = stmt.run(path, lang, hash, tokenCount);

  // Return the file id
  if (result.changes > 0 && result.lastInsertRowid) {
    return Number(result.lastInsertRowid);
  }
  const row = db.prepare("SELECT id FROM files WHERE path = ?").get(path) as { id: number };
  return row.id;
}

export function getFileHash(db: Database.Database, path: string): string | undefined {
  const row = db.prepare("SELECT hash FROM files WHERE path = ?").get(path) as
    | { hash: string }
    | undefined;
  return row?.hash;
}

export function insertSymbol(
  db: Database.Database,
  name: string,
  kind: string,
  fileId: number,
  startLine: number,
  endLine: number,
  signature: string,
): void {
  db.prepare(
    "INSERT INTO symbols (name, kind, file_id, start_line, end_line, signature) VALUES (?, ?, ?, ?, ?, ?)",
  ).run(name, kind, fileId, startLine, endLine, signature);
}

export function clearFileSymbols(db: Database.Database, fileId: number): void {
  db.prepare("DELETE FROM symbols WHERE file_id = ?").run(fileId);
}

export function upsertChunk(db: Database.Database, path: string, content: string): void {
  // Delete old entry first, then insert fresh
  db.prepare("DELETE FROM chunks_fts WHERE path = ?").run(path);
  db.prepare("INSERT INTO chunks_fts (path, content) VALUES (?, ?)").run(path, content);
}

export function searchChunks(db: Database.Database, query: string, limit: number): ChunkMatch[] {
  const stmt = db.prepare(`
    SELECT path, content, rank
    FROM chunks_fts
    WHERE chunks_fts MATCH ?
    ORDER BY rank
    LIMIT ?
  `);
  return stmt.all(query, limit) as ChunkMatch[];
}

export function getIndexStats(db: Database.Database): {
  fileCount: number;
  symbolCount: number;
  totalTokens: number;
} {
  const files = db.prepare("SELECT COUNT(*) as c FROM files").get() as { c: number };
  const symbols = db.prepare("SELECT COUNT(*) as c FROM symbols").get() as { c: number };
  const tokens = db.prepare("SELECT COALESCE(SUM(token_count), 0) as t FROM files").get() as {
    t: number;
  };
  return {
    fileCount: files.c,
    symbolCount: symbols.c,
    totalTokens: tokens.t,
  };
}

export function removeStaleFiles(db: Database.Database, activePaths: Set<string>): number {
  const allFiles = db.prepare("SELECT id, path FROM files").all() as { id: number; path: string }[];
  let removed = 0;
  const deleteFile = db.prepare("DELETE FROM files WHERE id = ?");
  const deleteChunk = db.prepare("DELETE FROM chunks_fts WHERE path = ?");

  for (const file of allFiles) {
    if (!activePaths.has(file.path)) {
      deleteFile.run(file.id);
      deleteChunk.run(file.path);
      removed++;
    }
  }
  return removed;
}
