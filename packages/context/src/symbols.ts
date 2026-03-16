export interface ExtractedSymbol {
  name: string;
  kind: "function" | "class" | "export" | "interface" | "type" | "method";
  startLine: number;
  endLine: number;
  signature: string;
}

interface PatternDef {
  pattern: RegExp;
  kind: ExtractedSymbol["kind"];
  nameGroup: number;
  sigGroup?: number;
}

const TS_PATTERNS: PatternDef[] = [
  {
    pattern: /^(?:export\s+)?(?:async\s+)?function\s+(\w+)\s*(<[^>]*>)?\s*\([^)]*\)/gm,
    kind: "function",
    nameGroup: 1,
  },
  {
    pattern: /^(?:export\s+)?class\s+(\w+)/gm,
    kind: "class",
    nameGroup: 1,
  },
  {
    pattern: /^(?:export\s+)?interface\s+(\w+)/gm,
    kind: "interface",
    nameGroup: 1,
  },
  {
    pattern: /^(?:export\s+)?type\s+(\w+)\s*(?:<[^>]*>)?\s*=/gm,
    kind: "type",
    nameGroup: 1,
  },
  {
    pattern: /^export\s+(?:const|let|var)\s+(\w+)/gm,
    kind: "export",
    nameGroup: 1,
  },
];

const PY_PATTERNS: PatternDef[] = [
  {
    pattern: /^\s*(?:async\s+)?def\s+(\w+)\s*\([^)]*\)/gm,
    kind: "function",
    nameGroup: 1,
  },
  {
    pattern: /^\s*class\s+(\w+)/gm,
    kind: "class",
    nameGroup: 1,
  },
];

function getPatternsForLang(lang: string): PatternDef[] {
  switch (lang) {
    case "typescript":
    case "javascript":
      return TS_PATTERNS;
    case "python":
      return PY_PATTERNS;
    default:
      return [];
  }
}

export function extractSymbols(content: string, lang: string): ExtractedSymbol[] {
  const patterns = getPatternsForLang(lang);
  if (patterns.length === 0) return [];

  const lines = content.split("\n");
  const symbols: ExtractedSymbol[] = [];

  for (const def of patterns) {
    // Reset regex state
    const regex = new RegExp(def.pattern.source, def.pattern.flags);
    let match: RegExpExecArray | null;

    while ((match = regex.exec(content)) !== null) {
      const name = match[def.nameGroup];
      if (!name) continue;

      // Calculate line number from match index
      const beforeMatch = content.slice(0, match.index);
      const startLine = beforeMatch.split("\n").length;

      // Estimate end line: scan forward for matching brace or use heuristic
      const endLine = estimateEndLine(lines, startLine - 1, lang);

      const signature = match[0].trim();

      symbols.push({
        name,
        kind: def.kind,
        startLine,
        endLine,
        signature: signature.length > 200 ? signature.slice(0, 200) + "..." : signature,
      });
    }
  }

  return symbols.sort((a, b) => a.startLine - b.startLine);
}

function estimateEndLine(lines: string[], startIdx: number, lang: string): number {
  // For Python: scan until indentation returns to same level or decreases
  if (lang === "python") {
    const startIndent = lines[startIdx]?.search(/\S/) ?? 0;
    for (let i = startIdx + 1; i < lines.length; i++) {
      const line = lines[i];
      if (line.trim() === "") continue;
      const indent = line.search(/\S/);
      if (indent <= startIndent && i > startIdx + 1) {
        return i; // line before this is the end
      }
    }
    return lines.length;
  }

  // For JS/TS: track brace depth
  let depth = 0;
  let foundOpen = false;
  for (let i = startIdx; i < lines.length; i++) {
    const line = lines[i];
    for (const ch of line) {
      if (ch === "{") {
        depth++;
        foundOpen = true;
      } else if (ch === "}") {
        depth--;
        if (foundOpen && depth === 0) {
          return i + 1;
        }
      }
    }
  }

  // Fallback: single line or small block
  return Math.min(startIdx + 5, lines.length);
}
