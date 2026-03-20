use oni_core::config::OniConfig;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Expand `~` at the start of a path to the user's home directory.
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

/// Extract the port number from a URL like "http://localhost:8081".
fn extract_port(url: &str) -> u16 {
    url.rsplit(':')
        .next()
        .and_then(|s| s.trim_end_matches('/').parse().ok())
        .unwrap_or(8080)
}

/// Find the `llama-server` binary on PATH.
fn find_llama_server() -> Result<PathBuf> {
    which::which("llama-server").map_err(|_| {
        "llama-server not found on PATH. Install llama.cpp or add it to your PATH.".into()
    })
}

/// Check if a server is healthy by hitting GET /health.
async fn check_health(url: &str) -> bool {
    reqwest::Client::new()
        .get(format!("{}/health", url))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Poll /health every 500ms until healthy or timeout.
async fn wait_for_health(url: &str, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if check_health(url).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

/// Ensure the specified tier servers are running.
/// If `needed_tiers` is None, starts all configured tiers.
/// If Some, only starts the listed tiers.
pub async fn ensure_servers_running(
    config: &OniConfig,
    needed_tiers: Option<&[&str]>,
) -> Result<()> {
    if !config.server.auto_start {
        return Ok(());
    }

    if config.server.tiers.is_empty() {
        return Ok(());
    }

    let llama_server = find_llama_server()?;
    let models_dir = expand_tilde(&config.server.models_dir);

    let mut started_any = false;

    for (tier_name, tier_url) in &config.server.tier_urls {
        // Skip tiers not in the needed list
        if let Some(needed) = needed_tiers {
            if !needed.iter().any(|n| n.eq_ignore_ascii_case(tier_name)) {
                continue;
            }
        }
        // Only manage tiers that have a GGUF config
        let tier_config = match config.server.tiers.get(tier_name) {
            Some(tc) => tc,
            None => continue,
        };

        // Check if already running
        if check_health(tier_url).await {
            continue;
        }

        if !started_any {
            eprintln!("ONI — starting model servers...");
            started_any = true;
        }

        // Resolve GGUF path
        let gguf_path = models_dir.join(&tier_config.gguf);
        if !gguf_path.exists() {
            eprintln!(
                "  {:<9} → SKIP (model not found: {})",
                tier_name,
                gguf_path.display()
            );
            continue;
        }

        let port = extract_port(tier_url);

        // Open log file for this tier
        let log_path = format!("/tmp/oni-{}.log", tier_name);
        let log_file = std::fs::File::create(&log_path).map_err(|e| {
            format!("Failed to create log file {}: {}", log_path, e)
        })?;
        let log_stderr = log_file.try_clone().map_err(|e| {
            format!("Failed to clone log file handle: {}", e)
        })?;

        let mut cmd = std::process::Command::new(&llama_server);
        cmd.arg("--model")
            .arg(&gguf_path)
            .arg("--port")
            .arg(port.to_string())
            .arg("--ctx-size")
            .arg(tier_config.ctx_size.to_string())
            .arg("--n-gpu-layers")
            .arg(tier_config.gpu_layers.to_string())
            .arg("--threads")
            .arg(tier_config.threads.to_string())
            .arg("--threads-batch")
            .arg(tier_config.threads_batch.to_string())
            .arg("--parallel")
            .arg(tier_config.parallel.to_string());

        if tier_config.flash_attn {
            cmd.arg("-fa").arg("on");
        }
        if let Some(ref k) = tier_config.cache_type_k {
            cmd.arg("--cache-type-k").arg(k);
        }
        if let Some(ref v) = tier_config.cache_type_v {
            cmd.arg("--cache-type-v").arg(v);
        }
        for arg in &tier_config.extra_args {
            cmd.arg(arg);
        }

        cmd.stdout(log_file)
            .stderr(log_stderr)
            .stdin(Stdio::null());

        // Detach from parent process group so the server survives ONI exit
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| {
                    libc::setsid();
                    Ok(())
                });
            }
        }

        let child = cmd.spawn().map_err(|e| {
            format!(
                "Failed to spawn llama-server for tier '{}': {}",
                tier_name, e
            )
        })?;

        eprintln!(
            "  {:<9} → {} (starting PID {})",
            tier_name,
            tier_url,
            child.id()
        );

        // Wait for health (120s timeout for large models)
        if wait_for_health(tier_url, Duration::from_secs(120)).await {
            eprintln!("  {:<9} → ready", tier_name);
        } else {
            eprintln!(
                "  {:<9} → TIMEOUT (check {})",
                tier_name, log_path
            );
        }
    }

    Ok(())
}
