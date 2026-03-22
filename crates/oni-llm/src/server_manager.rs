use oni_core::config::{ModelConfig, ServerConfig};
use oni_core::types::ModelTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

const STATE_FILE: &str = "/tmp/oni-servers.json";
const DEFAULT_MEMORY_HEADROOM: u64 = 4 * 1024 * 1024 * 1024; // 4 GB
const DEFAULT_MEMORY_MULTIPLIER: f64 = 1.3;

// ─── helpers ────────────────────────────────────────────────────────────────

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

/// Check if a process with the given PID is alive.
fn is_pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

// ─── core types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ServerInstance {
    pub pid: u32,
    pub port: u16,
    pub tier: ModelTier,
    pub model_name: String,
    pub gguf_path: PathBuf,
    pub estimated_mem: u64,
    pub last_used: Instant,
    pub started_at: u64, // unix timestamp
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedState {
    servers: HashMap<String, PersistedInstance>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedInstance {
    pid: u32,
    port: u16,
    tier: String,
    model_name: String,
    gguf_path: String,
    estimated_mem: u64,
    started_at: u64,
}

pub struct EvictionPlan {
    pub tiers_to_evict: Vec<ModelTier>,
    pub will_free: u64,
}

impl EvictionPlan {
    /// Select tiers to evict using LRU strategy.
    /// instances: (tier, estimated_mem, seconds_since_last_use)
    /// needed: bytes of memory to free
    /// target: tier being loaded (never evict this or Embed)
    pub fn select(
        instances: &[(ModelTier, u64, u64)],
        needed: u64,
        target: ModelTier,
    ) -> Self {
        let mut candidates: Vec<_> = instances
            .iter()
            .filter(|(tier, _, _)| *tier != target && *tier != ModelTier::Embed)
            .collect();
        // Sort by age descending (oldest/least-recently-used first)
        candidates.sort_by_key(|(_, _, age)| std::cmp::Reverse(*age));

        let mut to_evict = Vec::new();
        let mut freed: u64 = 0;
        for (tier, mem, _) in candidates {
            if freed >= needed {
                break;
            }
            to_evict.push(*tier);
            freed += mem;
        }

        EvictionPlan {
            tiers_to_evict: to_evict,
            will_free: freed,
        }
    }
}

// ─── ServerManager ──────────────────────────────────────────────────────────

pub struct ServerManager {
    server_config: ServerConfig,
    models_config: ModelConfig,
    instances: Arc<RwLock<HashMap<ModelTier, ServerInstance>>>,
    memory_headroom: u64,
    memory_multiplier: f64,
}

impl ServerManager {
    pub fn new(
        server_config: ServerConfig,
        models_config: ModelConfig,
        memory_headroom: u64,
        memory_multiplier: f64,
    ) -> Self {
        let headroom = if memory_headroom == 0 {
            DEFAULT_MEMORY_HEADROOM
        } else {
            memory_headroom
        };
        let multiplier = if memory_multiplier <= 0.0 {
            DEFAULT_MEMORY_MULTIPLIER
        } else {
            memory_multiplier
        };
        Self {
            server_config,
            models_config,
            instances: Arc::new(RwLock::new(HashMap::new())),
            memory_headroom: headroom,
            memory_multiplier: multiplier,
        }
    }

    /// Read /tmp/oni-servers.json and re-adopt any PIDs that are still alive and healthy.
    pub async fn restore_state(&self) {
        let Ok(raw) = std::fs::read_to_string(STATE_FILE) else {
            return;
        };
        let Ok(state) = serde_json::from_str::<PersistedState>(&raw) else {
            return;
        };

        let mut instances = self.instances.write().await;
        for (tier_key, persisted) in state.servers {
            let Some(tier) = ModelTier::from_key(&tier_key) else {
                continue;
            };
            if !is_pid_alive(persisted.pid) {
                continue;
            }
            let url = self.url_for_tier(tier);
            if !check_health(&url).await {
                continue;
            }
            instances.insert(
                tier,
                ServerInstance {
                    pid: persisted.pid,
                    port: persisted.port,
                    tier,
                    model_name: persisted.model_name,
                    gguf_path: PathBuf::from(persisted.gguf_path),
                    estimated_mem: persisted.estimated_mem,
                    last_used: Instant::now(),
                    started_at: persisted.started_at,
                },
            );
            eprintln!("  {} → adopted (PID {})", tier.display_name(), persisted.pid);
        }
    }

    /// Ensure a tier's server is loaded and healthy. This is the main brain.
    pub async fn ensure_loaded(&self, tier: ModelTier) -> Result<()> {
        let url = self.url_for_tier(tier);

        // Fast path: already registered and healthy.
        // Read snapshot first, then release lock before async health check.
        let existing_pid = {
            let instances = self.instances.read().await;
            instances.get(&tier).map(|inst| inst.pid)
        };

        if let Some(pid) = existing_pid {
            if is_pid_alive(pid) && check_health(&url).await {
                // Still alive — update last_used under write lock
                let mut instances = self.instances.write().await;
                if let Some(inst) = instances.get_mut(&tier) {
                    inst.last_used = Instant::now();
                }
                return Ok(());
            }
            // Stale entry — remove it
            self.instances.write().await.remove(&tier);
        }

        // Resolve GGUF path
        let models_dir = expand_tilde(&self.server_config.models_dir);
        let tier_key = tier.key();
        let tier_config = self
            .server_config
            .tiers
            .get(tier_key)
            .ok_or_else(|| format!("No server config for tier '{}'", tier_key))?;
        let gguf_path = models_dir.join(&tier_config.gguf);
        if !gguf_path.exists() {
            return Err(format!(
                "Model file not found for tier '{}': {}",
                tier_key,
                gguf_path.display()
            )
            .into());
        }

        // Estimate memory requirement
        let file_size = std::fs::metadata(&gguf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let estimated_mem =
            crate::memory::estimate_model_memory(file_size, self.memory_multiplier);

        // Memory check
        let mem = crate::memory::system_memory();
        let needed = estimated_mem + self.memory_headroom;
        if mem.available < needed {
            let shortfall = needed - mem.available;

            // Build eviction plan from current instances
            let snapshot: Vec<(ModelTier, u64, u64)> = {
                let instances = self.instances.read().await;
                instances
                    .values()
                    .map(|inst| {
                        let age_secs = inst.last_used.elapsed().as_secs();
                        (inst.tier, inst.estimated_mem, age_secs)
                    })
                    .collect()
            };

            let plan = EvictionPlan::select(&snapshot, shortfall, tier);
            for evict_tier in &plan.tiers_to_evict {
                eprintln!(
                    "  {} → evicting to free memory",
                    evict_tier.display_name()
                );
                let _ = self.stop_server(*evict_tier).await;
            }

            if !plan.tiers_to_evict.is_empty() {
                // Wait for OS to reclaim memory, then verify
                tokio::time::sleep(Duration::from_secs(2)).await;
                let rechecked = crate::memory::system_memory();
                if rechecked.available < estimated_mem {
                    // Still tight — wait a bit longer
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
        }

        // Spawn the server
        eprintln!(
            "  {} → spawning ({})",
            tier.display_name(),
            gguf_path.display()
        );
        let instance = self.spawn_server(tier).await?;
        let pid = instance.pid;

        // Wait for health
        if !wait_for_health(&url, Duration::from_secs(120)).await {
            return Err(format!(
                "Tier '{}' (PID {}) did not become healthy within 120s — check /tmp/oni-{}.log",
                tier_key, pid, tier_key
            )
            .into());
        }
        eprintln!("  {} → ready", tier.display_name());

        {
            let mut instances = self.instances.write().await;
            instances.insert(tier, instance);
        }
        self.save_state().await;
        Ok(())
    }

    /// Send SIGTERM, wait up to 5 s, then SIGKILL.
    pub async fn stop_server(&self, tier: ModelTier) -> Result<()> {
        let pid = {
            let mut instances = self.instances.write().await;
            match instances.remove(&tier) {
                Some(inst) => inst.pid,
                None => return Ok(()),
            }
        };

        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if !is_pid_alive(pid) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        if is_pid_alive(pid) {
            unsafe {
                libc::kill(pid as i32, libc::SIGKILL);
            }
        }

        self.save_state().await;
        Ok(())
    }

    /// Evict all tiers except target and Embed, then restart target if unhealthy.
    pub async fn recover(&self, tier: ModelTier) -> Result<()> {
        let tiers_to_stop: Vec<ModelTier> = {
            let instances = self.instances.read().await;
            instances
                .keys()
                .filter(|&&t| t != tier && t != ModelTier::Embed)
                .copied()
                .collect()
        };

        for t in tiers_to_stop {
            let _ = self.stop_server(t).await;
        }

        let url = self.url_for_tier(tier);
        // Snapshot PID under lock, then release before async health check
        let target_pid = {
            let instances = self.instances.read().await;
            instances.get(&tier).map(|inst| inst.pid)
        };
        let healthy = match target_pid {
            Some(pid) => is_pid_alive(pid) && check_health(&url).await,
            None => false,
        };

        if !healthy {
            {
                let mut instances = self.instances.write().await;
                instances.remove(&tier);
            }
            self.ensure_loaded(tier).await?;
        }

        Ok(())
    }

    /// List tiers that currently have a live, registered server.
    pub async fn loaded_tiers(&self) -> Vec<ModelTier> {
        self.instances.read().await.keys().copied().collect()
    }

    /// Stop all running servers.
    pub async fn stop_all(&self) {
        let tiers: Vec<ModelTier> = {
            let instances = self.instances.read().await;
            instances.keys().copied().collect()
        };
        for tier in tiers {
            let _ = self.stop_server(tier).await;
        }
    }

    /// Resolve the base URL for a tier from the server config.
    pub fn url_for_tier(&self, tier: ModelTier) -> String {
        self.server_config
            .tier_urls
            .get(tier.key())
            .cloned()
            .unwrap_or_else(|| format!("http://localhost:8080"))
    }

    /// Persist current instance state to /tmp/oni-servers.json.
    pub async fn save_state(&self) {
        let instances = self.instances.read().await;

        let servers: HashMap<String, PersistedInstance> = instances
            .values()
            .map(|inst| {
                let persisted = PersistedInstance {
                    pid: inst.pid,
                    port: inst.port,
                    tier: inst.tier.key().to_string(),
                    model_name: inst.model_name.clone(),
                    gguf_path: inst.gguf_path.to_string_lossy().into_owned(),
                    estimated_mem: inst.estimated_mem,
                    started_at: inst.started_at,
                };
                (inst.tier.key().to_string(), persisted)
            })
            .collect();

        let state = PersistedState { servers };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(STATE_FILE, json);
        }
    }

    /// Spawn a llama-server process for the given tier.
    async fn spawn_server(&self, tier: ModelTier) -> Result<ServerInstance> {
        let llama_server = find_llama_server()?;
        let models_dir = expand_tilde(&self.server_config.models_dir);
        let tier_key = tier.key();

        let tier_config = self
            .server_config
            .tiers
            .get(tier_key)
            .ok_or_else(|| format!("No server config for tier '{}'", tier_key))?;

        let gguf_path = models_dir.join(&tier_config.gguf);
        let file_size = std::fs::metadata(&gguf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let estimated_mem =
            crate::memory::estimate_model_memory(file_size, self.memory_multiplier);

        let url = self.url_for_tier(tier);
        let port = extract_port(&url);

        let log_path = format!("/tmp/oni-{}.log", tier_key);
        let log_file = std::fs::File::create(&log_path)
            .map_err(|e| format!("Failed to create log file {}: {}", log_path, e))?;
        let log_stderr = log_file
            .try_clone()
            .map_err(|e| format!("Failed to clone log file handle: {}", e))?;

        let model_name = self.models_config.model_for_tier(tier).to_string();

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
            cmd.arg("--flash-attn");
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

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn llama-server for tier '{}': {}", tier_key, e))?;

        let pid = child.id();
        eprintln!(
            "  {:<9} → {} (starting PID {})",
            tier.display_name(),
            url,
            pid
        );

        let started_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(ServerInstance {
            pid,
            port,
            tier,
            model_name,
            gguf_path,
            estimated_mem,
            last_used: Instant::now(),
            started_at,
        })
    }
}

// ─── backward-compat wrappers ────────────────────────────────────────────────

/// Backward-compatible entry point used by main.rs.
/// Constructs a `ServerManager` from the full `OniConfig` and starts needed tiers.
pub async fn ensure_servers_running(
    config: &oni_core::config::OniConfig,
    needed_tiers: Option<&[&str]>,
) -> Result<()> {
    if !config.server.auto_start {
        return Ok(());
    }
    if config.server.tiers.is_empty() {
        return Ok(());
    }
    let manager = ServerManager::new(
        config.server.clone(),
        config.models.clone(),
        config.server.memory_headroom,
        config.server.memory_multiplier,
    );
    manager.restore_state().await;
    ensure_servers_running_via(&manager, needed_tiers).await
}

/// Thin wrapper for callers that already hold a `ServerManager`.
pub async fn ensure_servers_running_via(
    manager: &ServerManager,
    needed_tiers: Option<&[&str]>,
) -> Result<()> {
    let tiers: Vec<ModelTier> = match needed_tiers {
        Some(names) => names
            .iter()
            .filter_map(|n| ModelTier::from_key(n))
            .collect(),
        None => vec![
            ModelTier::Heavy,
            ModelTier::Medium,
            ModelTier::General,
            ModelTier::Fast,
            ModelTier::Embed,
        ],
    };
    for tier in tiers {
        if let Err(e) = manager.ensure_loaded(tier).await {
            eprintln!("  {} → {}", tier.display_name(), e);
        }
    }
    Ok(())
}
