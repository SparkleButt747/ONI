import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import type Database from "better-sqlite3";
import {
  clearFileSymbols,
  createIndexDb,
  getFileHash,
  getIndexStats,
  insertSymbol,
  removeStaleFiles,
  upsertChunk,
  upsertFile,
} from "./index-db.js";
import { extractSymbols } from "./symbols.js";
import { detectLang, walkProject } from "./walker.js";

export interface IndexResult {
  filesIndexed: number;
  filesSkipped: number;
  filesRemoved: number;
  symbolsFound: number;
  totalFiles: number;
  totalSymbols: number;
  totalTokens: number;
  elapsedMs: number;
}

const MAX_FILE_SIZE = 512 * 1024; // 512KB — skip huge files
const CHARS_PER_TOKEN = 4;

function hashContent(content: string): string {
  return createHash("sha256").update(content).digest("hex");
}

function estimateTokens(content: string): number {
  return Math.ceil(content.length / CHARS_PER_TOKEN);
}

export function initIndex(projectDir: string): IndexResult {
  const dbPath = join(projectDir, ".oni", "index.db");
  const db = createIndexDb(dbPath);

  try {
    return runIndex(db, projectDir);
  } finally {
    db.close();
  }
}

export function updateIndex(projectDir: string): IndexResult {
  return initIndex(projectDir); // Same logic — upserts handle incremental
}

function runIndex(db: Database.Database, projectDir: string): IndexResult {
  const start = performance.now();
  const files = walkProject(projectDir);

  let filesIndexed = 0;
  let filesSkipped = 0;
  let symbolsFound = 0;

  const activePaths = new Set(files);

  const indexFile = db.transaction((relPath: string) => {
    const fullPath = join(projectDir, relPath);

    let content: string;
    try {
      const stat = readFileSync(fullPath);
      if (stat.length > MAX_FILE_SIZE) {
        filesSkipped++;
        return;
      }
      content = stat.toString("utf-8");
    } catch {
      filesSkipped++;
      return;
    }

    const hash = hashContent(content);
    const existingHash = getFileHash(db, relPath);

    if (existingHash === hash) {
      filesSkipped++;
      return;
    }

    const lang = detectLang(relPath);
    const tokenCount = estimateTokens(content);
    const fileId = upsertFile(db, relPath, lang, hash, tokenCount);

    // Clear old symbols and re-extract
    clearFileSymbols(db, fileId);
    const symbols = extractSymbols(content, lang);
    for (const sym of symbols) {
      insertSymbol(db, sym.name, sym.kind, fileId, sym.startLine, sym.endLine, sym.signature);
      symbolsFound++;
    }

    // Index content for FTS
    upsertChunk(db, relPath, content);

    filesIndexed++;
  });

  for (const relPath of files) {
    indexFile(relPath);
  }

  // Remove files that no longer exist
  const filesRemoved = removeStaleFiles(db, activePaths);

  const stats = getIndexStats(db);
  const elapsed = performance.now() - start;

  return {
    filesIndexed,
    filesSkipped,
    filesRemoved,
    symbolsFound,
    totalFiles: stats.fileCount,
    totalSymbols: stats.symbolCount,
    totalTokens: stats.totalTokens,
    elapsedMs: Math.round(elapsed),
  };
}
