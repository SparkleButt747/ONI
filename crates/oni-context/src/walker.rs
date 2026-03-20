use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

const MAX_FILE_SIZE: u64 = 512 * 1024; // 512KB

const ALWAYS_SKIP: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    "__pycache__",
    ".DS_Store",
    ".oni",
    ".next",
    ".turbo",
    ".cache",
    "coverage",
    ".nyc_output",
    ".vscode",
    ".idea",
];

const INDEXED_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "cjs", "py", "rs", "go", "java", "c", "cpp", "h", "hpp",
    "json", "md", "yaml", "yml", "toml", "css", "html", "sql", "sh", "rb", "cs", "swift", "kt",
    "scala", "php",
];

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub path: PathBuf,
    pub language: String,
}

pub fn walk_project(root: &Path) -> Vec<DiscoveredFile> {
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(true)
        .ignore(true)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(false)
        .follow_links(false);

    // Load .oniignore if present
    let oniignore = root.join(".oniignore");
    if oniignore.exists() {
        builder.add_ignore(&oniignore);
    }

    let mut files = Vec::new();

    for entry in builder.build().flatten() {
        let path = entry.path();

        // Skip entries whose name is in the always-skip list
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if ALWAYS_SKIP.contains(&name) {
                continue;
            }
        }

        // Only process regular files
        let Ok(metadata) = entry.metadata() else { continue };
        if !metadata.is_file() {
            continue;
        }

        // Skip large files
        if metadata.len() > MAX_FILE_SIZE {
            continue;
        }

        // Filter by extension
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else { continue };
        if !INDEXED_EXTENSIONS.contains(&ext) {
            continue;
        }

        let language = detect_lang(ext);
        files.push(DiscoveredFile { path: path.to_path_buf(), language });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

pub fn detect_lang(ext: &str) -> String {
    match ext {
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "rs" => "rust",
        "go" => "go",
        "java" => "java",
        "c" | "h" => "c",
        "cpp" | "hpp" => "cpp",
        "rb" => "ruby",
        "cs" => "csharp",
        "swift" => "swift",
        "kt" => "kotlin",
        "scala" => "scala",
        "php" => "php",
        "sh" => "shell",
        "json" => "json",
        "md" => "markdown",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "css" => "css",
        "html" => "html",
        "sql" => "sql",
        _ => "unknown",
    }
    .to_string()
}
