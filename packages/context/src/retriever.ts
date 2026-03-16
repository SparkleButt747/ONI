import { join } from "node:path";
import { createIndexDb, searchChunks } from "./index-db.js";

export interface ContextChunk {
  path: string;
  content: string;
  score: number;
  tokenEstimate: number;
}

export interface ContextPack {
  chunks: ContextChunk[];
  totalTokens: number;
  retrievalMs: number;
}

const CHARS_PER_TOKEN = 4;
const DEFAULT_TOKEN_BUDGET = 8000;
const DEFAULT_MAX_RESULTS = 20;

function estimateTokens(text: string): number {
  return Math.ceil(text.length / CHARS_PER_TOKEN);
}

/**
 * Sanitise a query for FTS5 MATCH syntax.
 * Strips characters that FTS5 treats as syntax (quotes, parens, etc.)
 * and joins remaining tokens with implicit AND.
 */
function sanitiseQuery(raw: string): string {
  // Remove FTS5 special chars, keep alphanumeric + spaces
  const cleaned = raw.replace(/[^\w\s]/g, " ").trim();
  if (!cleaned) return "";
  // Split into tokens and rejoin — FTS5 implicit AND
  return cleaned
    .split(/\s+/)
    .filter((t) => t.length > 1)
    .join(" ");
}

export function queryContext(
  projectDir: string,
  query: string,
  tokenBudget: number = DEFAULT_TOKEN_BUDGET,
): ContextPack {
  const start = performance.now();
  const dbPath = join(projectDir, ".oni", "index.db");

  let db;
  try {
    db = createIndexDb(dbPath);
  } catch {
    return { chunks: [], totalTokens: 0, retrievalMs: 0 };
  }

  try {
    const sanitised = sanitiseQuery(query);
    if (!sanitised) {
      return { chunks: [], totalTokens: 0, retrievalMs: Math.round(performance.now() - start) };
    }

    const rawResults = searchChunks(db, sanitised, DEFAULT_MAX_RESULTS);
    const chunks: ContextChunk[] = [];
    let totalTokens = 0;

    for (const result of rawResults) {
      const tokenEstimate = estimateTokens(result.content);
      if (totalTokens + tokenEstimate > tokenBudget) {
        // Try truncating the content to fit remaining budget
        const remainingTokens = tokenBudget - totalTokens;
        if (remainingTokens > 100) {
          const truncatedContent = result.content.slice(0, remainingTokens * CHARS_PER_TOKEN);
          chunks.push({
            path: result.path,
            content: truncatedContent,
            score: Math.abs(result.rank),
            tokenEstimate: remainingTokens,
          });
          totalTokens += remainingTokens;
        }
        break;
      }

      chunks.push({
        path: result.path,
        content: result.content,
        score: Math.abs(result.rank),
        tokenEstimate,
      });
      totalTokens += tokenEstimate;
    }

    return {
      chunks,
      totalTokens,
      retrievalMs: Math.round(performance.now() - start),
    };
  } finally {
    db.close();
  }
}
