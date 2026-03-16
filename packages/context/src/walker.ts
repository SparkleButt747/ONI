import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import ignore, { type Ignore } from "ignore";

const ALWAYS_SKIP = new Set([
  "node_modules",
  ".git",
  "dist",
  "build",
  ".oni",
  "__pycache__",
  ".next",
  ".turbo",
  ".cache",
  "coverage",
  ".nyc_output",
  ".vscode",
  ".idea",
]);

const INDEXED_EXTENSIONS = new Set([
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".mjs",
  ".cjs",
  ".py",
  ".json",
  ".md",
  ".yaml",
  ".yml",
  ".toml",
  ".css",
  ".html",
  ".sql",
  ".sh",
  ".rs",
  ".go",
]);

function loadIgnoreFile(projectDir: string, filename: string): string | null {
  try {
    return readFileSync(join(projectDir, filename), "utf-8");
  } catch {
    return null;
  }
}

export function walkProject(projectDir: string): string[] {
  const ig: Ignore = ignore.default();

  const gitignore = loadIgnoreFile(projectDir, ".gitignore");
  if (gitignore) ig.add(gitignore);

  const oniignore = loadIgnoreFile(projectDir, ".oniignore");
  if (oniignore) ig.add(oniignore);

  const files: string[] = [];

  function walk(dir: string): void {
    let entries: string[];
    try {
      entries = readdirSync(dir);
    } catch {
      return;
    }

    for (const entry of entries) {
      if (entry.startsWith(".") && entry !== ".gitignore" && entry !== ".oniignore") {
        // Skip hidden files/dirs (except ignore files themselves)
        if (ALWAYS_SKIP.has(entry)) continue;
        // Still skip other dotfiles
        continue;
      }

      if (ALWAYS_SKIP.has(entry)) continue;

      const fullPath = join(dir, entry);
      const relPath = relative(projectDir, fullPath);

      // Check gitignore
      if (ig.ignores(relPath)) continue;

      let stat;
      try {
        stat = statSync(fullPath);
      } catch {
        continue;
      }

      if (stat.isDirectory()) {
        walk(fullPath);
      } else if (stat.isFile()) {
        const ext = "." + entry.split(".").pop();
        if (INDEXED_EXTENSIONS.has(ext)) {
          files.push(relPath);
        }
      }
    }
  }

  walk(projectDir);
  return files.sort();
}

export function detectLang(filePath: string): string {
  const ext = filePath.split(".").pop() ?? "";
  const langMap: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    py: "python",
    rs: "rust",
    go: "go",
    json: "json",
    md: "markdown",
    yaml: "yaml",
    yml: "yaml",
    toml: "toml",
    css: "css",
    html: "html",
    sql: "sql",
    sh: "shell",
  };
  return langMap[ext] ?? "unknown";
}
