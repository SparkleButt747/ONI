//! DroidShield-lite — automatic linting after file writes.
//! Detects language from extension and runs the appropriate linter.
//! Non-blocking — results returned as a string for display.

fn find_manifest(file_path: &str) -> Option<String> {
    let mut dir = std::path::Path::new(file_path).parent()?;
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.exists() {
            return Some(manifest.to_string_lossy().to_string());
        }
        dir = dir.parent()?;
    }
}

fn format_output(output: std::process::Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}{}", stdout, stderr);
    let trimmed = combined.trim();

    if trimmed.is_empty() || output.status.success() {
        None // Clean — no issues
    } else {
        // Truncate to avoid flooding the chat
        let truncated = if trimmed.len() > 500 {
            format!("{}...\n(truncated)", &trimmed[..500])
        } else {
            trimmed.to_string()
        };
        Some(truncated)
    }
}

/// Run the appropriate linter on a file. Returns lint output or None if no linter available.
pub fn lint_file(path: &str) -> Option<String> {
    let ext = path.rsplit('.').next().unwrap_or("");

    // Check if the linter is available
    let cmd = match ext {
        "rs" => "cargo",
        "py" => "ruff",
        "js" | "jsx" | "ts" | "tsx" => "npx",
        "go" => "go",
        _ => return None,
    };
    let check = std::process::Command::new("which").arg(cmd).output().ok()?;
    if !check.status.success() {
        return None; // Linter not installed
    }

    match ext {
        "rs" => {
            let mut cmd_builder =
                std::process::Command::new("cargo");
            cmd_builder.args(["clippy", "--message-format=short", "--quiet"]);
            if let Some(manifest) = find_manifest(path) {
                cmd_builder.arg("--manifest-path").arg(manifest);
            }
            format_output(cmd_builder.output().ok()?)
        }
        "py" => format_output(
            std::process::Command::new("ruff")
                .args(["check", path, "--output-format=concise"])
                .output()
                .ok()?,
        ),
        "js" | "jsx" | "ts" | "tsx" => format_output(
            std::process::Command::new("npx")
                .args(["eslint", "--format=compact", path])
                .output()
                .ok()?,
        ),
        "go" => format_output(
            std::process::Command::new("go")
                .args(["vet", path])
                .output()
                .ok()?,
        ),
        _ => None,
    }
}

/// Detect language name from file extension (for display).
pub fn language_for_ext(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "Rust",
        "py" => "Python",
        "js" => "JavaScript",
        "jsx" => "React JSX",
        "ts" => "TypeScript",
        "tsx" => "React TSX",
        "go" => "Go",
        _ => "Unknown",
    }
}
