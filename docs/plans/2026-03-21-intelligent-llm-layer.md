# Intelligent LLM Server Manager — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Memory-aware server lifecycle manager that intelligently loads/unloads llama-server instances when switching tiers, with LRU eviction and error recovery instead of crashing.

**Architecture:** Replace the two ad-hoc server spawners (`src/server_manager.rs::ensure_servers_running()` and `app.rs::ensure_tier_server()`) with a single `ServerManager` struct in the root crate. It tracks running server PIDs, probes system memory before loading models, evicts LRU models when memory is tight, and retries on COMPUTE ERRORs. Conversation context is already preserved in the `Conversation` struct in memory — no special save/restore needed.

**Tech Stack:** Rust, `sysinfo` crate (memory probing), `serde_json` (state persistence), existing `reqwest`/`tokio` for health checks.

---

## Problem Statement

On M4 Max 128GB, switching tiers (e.g., HEAVY 35b → CODE 30b) tries to spawn a second llama-server without checking available memory. The new server either:
- Fails to load the model (OOM)
- Loads but gets 500 COMPUTE ERROR during inference

Current code treats ALL llama-server errors as CRITICAL FAILURE → full-screen crash with no recovery.

## Architecture

```
┌─────────────────────────────────────────────┐
│                 ServerManager                │
│  ┌────────────────────────────────────────┐  │
│  │ instances: HashMap<ModelTier, Instance> │  │
│  │ state_file: /tmp/oni-servers.json       │  │
│  └────────────────────────────────────────┘  │
│                                              │
│  ensure_loaded(tier)                         │
│    1. Already running? → mark_used, return   │
│    2. Enough memory? → spawn, return         │
│    3. Not enough → evict LRU, spawn          │
│                                              │
│  recover(tier)                               │
│    1. Evict all except target tier            │
│    2. Verify target health                   │
│    3. Restart target if unhealthy            │
│                                              │
│  stop_server(tier)                           │
│    1. SIGTERM → wait 5s → SIGKILL            │
│    2. Remove from instances                  │
│    3. Update state file                      │
└─────────────────────────────────────────────┘
```

**Flow on tier switch (`/tier heavy`):**
```
1. ServerManager::ensure_loaded(Heavy)
2. Health check Heavy port → not running
3. Read GGUF file size → estimate runtime memory (size × 1.3)
4. Query sysinfo → available_memory
5. available < needed + headroom?
   YES → find LRU tier(s) to evict (not Embed, not target)
       → stop_server(evicted_tier) for each
       → wait for memory to free
6. spawn_server(Heavy) → PID, health wait
7. Update state file
8. Agent proceeds with Heavy tier
```

**Flow on COMPUTE ERROR during chat:**
```
1. agent.run_turn() → Err("llama-server API error (500): ...COMPUTE ERROR...")
2. Detect COMPUTE ERROR pattern in error text
3. Call server_manager.recover(current_tier)
   → evict all other tiers
   → verify current tier is healthy (restart if not)
4. Retry the turn ONCE
5. If retry fails → show error as message (NOT critical_error)
```

## File Structure

| File | Action | Purpose |
|------|--------|---------|
| `src/memory.rs` | CREATE | System memory probing, model size estimation |
| `src/server_manager.rs` | REWRITE | `ServerManager` struct with lifecycle ops |
| `crates/oni-tui/src/app.rs` | MODIFY | Wire ServerManager, error recovery, status events |
| `src/main.rs` | MODIFY | Create ServerManager, pass to TUI |
| `Cargo.toml` | MODIFY | Add `sysinfo` dependency |
| `crates/oni-core/src/config.rs` | MODIFY | Add memory config fields |
| `tests/server_manager_tests.rs` | CREATE | Unit tests for memory + eviction logic |

---

### Task 1: Memory Probing Module

**Files:**
- Create: `src/memory.rs`
- Modify: `Cargo.toml` (add `sysinfo = "0.34"`)

- [ ] **Step 1: Add sysinfo dependency**

In root `Cargo.toml`, add to `[dependencies]`:
```toml
sysinfo = "0.34"
```

- [ ] **Step 2: Write failing tests for memory probing**

Create `tests/memory_tests.rs`:
```rust
use oni::memory::{system_memory, estimate_model_memory};
use std::path::PathBuf;

#[test]
fn test_system_memory_returns_nonzero() {
    let report = system_memory();
    assert!(report.total > 0, "total memory should be > 0");
    assert!(report.available > 0, "available memory should be > 0");
    assert!(report.available <= report.total, "available <= total");
}

#[test]
fn test_estimate_model_memory_from_file_size() {
    // 10 GB file × 1.3 multiplier = 13 GB estimated
    let est = estimate_model_memory(10 * 1024 * 1024 * 1024, 1.3);
    assert_eq!(est, 13 * 1024 * 1024 * 1024);
}

#[test]
fn test_estimate_model_memory_default_multiplier() {
    let est = estimate_model_memory(10 * 1024 * 1024 * 1024, 0.0);
    // Should use default 1.3 when multiplier is 0 or unset
    assert_eq!(est, 13 * 1024 * 1024 * 1024);
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test --test memory_tests`
Expected: FAIL — module doesn't exist yet.

- [ ] **Step 4: Implement memory module**

Create `src/memory.rs`:
```rust
use sysinfo::System;

/// Snapshot of system memory state.
pub struct MemoryReport {
    /// Total physical memory in bytes.
    pub total: u64,
    /// Available memory in bytes (free + reclaimable).
    pub available: u64,
}

/// Query current system memory.
pub fn system_memory() -> MemoryReport {
    let mut sys = System::new();
    sys.refresh_memory();
    MemoryReport {
        total: sys.total_memory(),
        available: sys.available_memory(),
    }
}

/// Estimate runtime memory for a model given its GGUF file size.
/// `multiplier` accounts for KV cache, activations, and Metal overhead.
/// If multiplier is <= 0, uses default 1.3.
pub fn estimate_model_memory(gguf_file_size: u64, multiplier: f64) -> u64 {
    let mult = if multiplier <= 0.0 { 1.3 } else { multiplier };
    (gguf_file_size as f64 * mult) as u64
}

/// Read the file size of a GGUF model file.
pub fn gguf_file_size(path: &std::path::Path) -> std::io::Result<u64> {
    std::fs::metadata(path).map(|m| m.len())
}
```

- [ ] **Step 5: Export from main crate**

Add `pub mod memory;` to `src/main.rs` (or `src/lib.rs` if it exists).

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test memory_tests`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/memory.rs tests/memory_tests.rs Cargo.toml
git commit -m "feat(server): add memory probing module with sysinfo"
```

---

### Task 2: ServerManager Core Struct

**Files:**
- Rewrite: `src/server_manager.rs`
- Modify: `src/main.rs` (re-export)

- [ ] **Step 1: Write failing tests for server instance tracking**

Create `tests/server_manager_tests.rs`:
```rust
use oni::server_manager::{ServerManager, ServerInstance, EvictionPlan};
use oni_core::types::ModelTier;
use std::time::Instant;

#[test]
fn test_eviction_selects_lru_first() {
    // Given: Heavy (used 10s ago), General (used 5s ago), Fast (used 1s ago)
    // When: need to evict for Heavy load
    // Then: General evicted first (oldest that isn't target), then Fast
    let instances = vec![
        (ModelTier::General, 60_000_000_000u64, 5), // 60GB, used 5s ago
        (ModelTier::Fast, 8_000_000_000u64, 1),     // 8GB, used 1s ago
    ];
    let plan = EvictionPlan::select(
        &instances,
        50_000_000_000, // need 50GB
        ModelTier::Heavy, // loading Heavy
    );
    assert_eq!(plan.tiers_to_evict.len(), 1);
    assert_eq!(plan.tiers_to_evict[0], ModelTier::General);
    assert!(plan.will_free >= 50_000_000_000);
}

#[test]
fn test_eviction_never_evicts_target_tier() {
    let instances = vec![
        (ModelTier::Heavy, 20_000_000_000u64, 10),
    ];
    let plan = EvictionPlan::select(
        &instances,
        20_000_000_000,
        ModelTier::Heavy, // target = Heavy, should not evict itself
    );
    assert!(plan.tiers_to_evict.is_empty());
}

#[test]
fn test_eviction_skips_embed_tier() {
    let instances = vec![
        (ModelTier::Embed, 300_000_000u64, 100),
        (ModelTier::Fast, 8_000_000_000u64, 1),
    ];
    let plan = EvictionPlan::select(
        &instances,
        8_000_000_000,
        ModelTier::Heavy,
    );
    // Should evict Fast, not Embed
    assert_eq!(plan.tiers_to_evict.len(), 1);
    assert_eq!(plan.tiers_to_evict[0], ModelTier::Fast);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test server_manager_tests`
Expected: FAIL — types don't exist yet.

- [ ] **Step 3: Implement ServerManager struct and EvictionPlan**

Rewrite `src/server_manager.rs` — keep `expand_tilde`, `extract_port`, `find_llama_server`, `check_health`, `wait_for_health` helper functions. Add:

```rust
use oni_core::config::{ServerConfig, ModelConfig, TierServerConfig};
use oni_core::types::ModelTier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;

const STATE_FILE: &str = "/tmp/oni-servers.json";
const DEFAULT_MEMORY_HEADROOM: u64 = 4 * 1024 * 1024 * 1024; // 4 GB
const DEFAULT_MEMORY_MULTIPLIER: f64 = 1.3;

/// A running llama-server instance.
#[derive(Debug, Clone)]
pub struct ServerInstance {
    pub pid: u32,
    pub port: u16,
    pub tier: ModelTier,
    pub model_name: String,
    pub gguf_path: PathBuf,
    pub estimated_mem: u64,    // estimated runtime memory in bytes
    pub last_used: Instant,
}

/// Serialisable state for persistence across ONI restarts.
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
    started_at: u64, // unix timestamp
}

/// Result of eviction planning — which tiers to evict and how much memory freed.
pub struct EvictionPlan {
    pub tiers_to_evict: Vec<ModelTier>,
    pub will_free: u64,
}

impl EvictionPlan {
    /// Select tiers to evict using LRU strategy.
    /// `instances`: (tier, estimated_mem, seconds_since_last_use)
    /// `needed`: bytes of memory needed
    /// `target`: tier being loaded (never evict this)
    pub fn select(
        instances: &[(ModelTier, u64, u64)],
        needed: u64,
        target: ModelTier,
    ) -> Self {
        // Sort by last_used ascending (oldest first = evict first)
        let mut candidates: Vec<_> = instances
            .iter()
            .filter(|(tier, _, _)| *tier != target && *tier != ModelTier::Embed)
            .collect();
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

/// Central server lifecycle manager.
pub struct ServerManager {
    server_config: ServerConfig,
    models_config: ModelConfig,
    instances: Arc<RwLock<HashMap<ModelTier, ServerInstance>>>,
    memory_headroom: u64,
    memory_multiplier: f64,
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test server_manager_tests`
Expected: PASS for eviction tests.

- [ ] **Step 5: Commit**

```bash
git add src/server_manager.rs tests/server_manager_tests.rs
git commit -m "feat(server): ServerManager struct with LRU eviction planning"
```

---

### Task 3: Server Lifecycle Operations

**Files:**
- Modify: `src/server_manager.rs`

Implement the core lifecycle methods on `ServerManager`.

- [ ] **Step 1: Implement `new()`, state loading, PID validation**

```rust
impl ServerManager {
    pub fn new(
        server_config: ServerConfig,
        models_config: ModelConfig,
        memory_headroom: u64,
        memory_multiplier: f64,
    ) -> Self {
        let headroom = if memory_headroom == 0 { DEFAULT_MEMORY_HEADROOM } else { memory_headroom };
        let multiplier = if memory_multiplier <= 0.0 { DEFAULT_MEMORY_MULTIPLIER } else { memory_multiplier };

        let mgr = Self {
            server_config,
            models_config,
            instances: Arc::new(RwLock::new(HashMap::new())),
            memory_headroom: headroom,
            memory_multiplier: multiplier,
        };
        // Load persisted state is done async, caller should call mgr.restore_state().await
        mgr
    }

    /// Restore state from disk, validate PIDs are alive + healthy.
    pub async fn restore_state(&self) {
        let state: PersistedState = match std::fs::read_to_string(STATE_FILE) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(st) => st,
                Err(_) => return,
            },
            Err(_) => return,
        };

        let mut instances = self.instances.write().await;
        for (_, ps) in state.servers {
            let tier = match ModelTier::from_key(&ps.tier) {
                Some(t) => t,
                None => continue,
            };
            // Check if PID is still alive
            if !is_pid_alive(ps.pid) {
                continue;
            }
            // Check if port responds to health
            let url = self.url_for_tier(tier);
            if !check_health(&url).await {
                continue;
            }
            instances.insert(tier, ServerInstance {
                pid: ps.pid,
                port: ps.port,
                tier,
                model_name: ps.model_name,
                gguf_path: PathBuf::from(ps.gguf_path),
                estimated_mem: ps.estimated_mem,
                last_used: Instant::now(),
            });
        }
    }
}

/// Check if a PID is alive via kill(pid, 0).
fn is_pid_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}
```

- [ ] **Step 2: Implement `stop_server()` — graceful shutdown**

```rust
impl ServerManager {
    /// Stop a running server. SIGTERM → 5s wait → SIGKILL.
    pub async fn stop_server(&self, tier: ModelTier) -> Result<(), String> {
        let mut instances = self.instances.write().await;
        let instance = match instances.remove(&tier) {
            Some(i) => i,
            None => return Ok(()), // not running
        };
        drop(instances); // release lock before blocking

        tracing::info!("Stopping {} server (PID {})", tier.display_name(), instance.pid);

        // SIGTERM
        unsafe { libc::kill(instance.pid as i32, libc::SIGTERM); }

        // Wait up to 5s for process to exit
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if !is_pid_alive(instance.pid) {
                self.save_state().await;
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }

        // SIGKILL if still alive
        tracing::warn!("SIGKILL {} server (PID {})", tier.display_name(), instance.pid);
        unsafe { libc::kill(instance.pid as i32, libc::SIGKILL); }
        tokio::time::sleep(Duration::from_millis(500)).await;

        self.save_state().await;
        Ok(())
    }
}
```

- [ ] **Step 3: Implement `spawn_server()` — start with PID tracking**

Adapt the existing spawn logic from `ensure_servers_running()`, but track PID in instances map and state file.

```rust
impl ServerManager {
    async fn spawn_server(&self, tier: ModelTier) -> Result<ServerInstance, String> {
        let tier_name = tier.key();
        let tier_url = self.url_for_tier(tier);
        let tier_cfg = self.server_config.tiers.get(tier_name)
            .ok_or_else(|| format!("No config for tier '{}'", tier_name))?
            .clone();

        let llama_server = find_llama_server()?;
        let models_dir = expand_tilde(&self.server_config.models_dir);
        let gguf_path = models_dir.join(&tier_cfg.gguf);

        if !gguf_path.exists() {
            return Err(format!("Model not found: {}", gguf_path.display()));
        }

        let gguf_size = std::fs::metadata(&gguf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let estimated_mem = crate::memory::estimate_model_memory(gguf_size, self.memory_multiplier);

        let port = extract_port(&tier_url);
        let log_path = format!("/tmp/oni-{}.log", tier_name);
        let log_file = std::fs::File::create(&log_path)
            .map_err(|e| format!("Log file: {}", e))?;
        let log_stderr = log_file.try_clone()
            .map_err(|e| format!("Log clone: {}", e))?;

        let mut cmd = std::process::Command::new(&llama_server);
        cmd.arg("--model").arg(&gguf_path)
            .arg("--port").arg(port.to_string())
            .arg("--ctx-size").arg(tier_cfg.ctx_size.to_string())
            .arg("--n-gpu-layers").arg(tier_cfg.gpu_layers.to_string())
            .arg("--threads").arg(tier_cfg.threads.to_string())
            .arg("--threads-batch").arg(tier_cfg.threads_batch.to_string())
            .arg("--parallel").arg(tier_cfg.parallel.to_string());

        if tier_cfg.flash_attn { cmd.arg("-fa").arg("on"); }
        if let Some(ref k) = tier_cfg.cache_type_k { cmd.arg("--cache-type-k").arg(k); }
        if let Some(ref v) = tier_cfg.cache_type_v { cmd.arg("--cache-type-v").arg(v); }
        for arg in &tier_cfg.extra_args { cmd.arg(arg); }

        cmd.stdout(log_file).stderr(log_stderr).stdin(std::process::Stdio::null());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                cmd.pre_exec(|| { libc::setsid(); Ok(()) });
            }
        }

        let child = cmd.spawn()
            .map_err(|e| format!("Failed to spawn llama-server: {}", e))?;
        let pid = child.id();

        tracing::info!("Spawned {} server (PID {}, port {})", tier.display_name(), pid, port);

        // Wait for health
        if !wait_for_health(&tier_url, Duration::from_secs(120)).await {
            // Kill the failed process
            unsafe { libc::kill(pid as i32, libc::SIGKILL); }
            return Err(format!("{} server failed to become healthy (check {})", tier.display_name(), log_path));
        }

        let model_name = self.models_config.model_for_tier(tier).to_string();
        let instance = ServerInstance {
            pid,
            port,
            tier,
            model_name,
            gguf_path,
            estimated_mem,
            last_used: Instant::now(),
        };

        let mut instances = self.instances.write().await;
        instances.insert(tier, instance.clone());
        drop(instances);
        self.save_state().await;

        Ok(instance)
    }
}
```

- [ ] **Step 4: Implement `ensure_loaded()` — the main brain**

```rust
impl ServerManager {
    /// Ensure a tier's server is running and ready. This is the main entry point.
    /// Returns Ok(()) if the server is ready, Err if it cannot be started.
    pub async fn ensure_loaded(&self, tier: ModelTier) -> Result<(), String> {
        // 1. Already running and healthy?
        {
            let mut instances = self.instances.write().await;
            if let Some(inst) = instances.get_mut(&tier) {
                if is_pid_alive(inst.pid) && check_health(&self.url_for_tier(tier)).await {
                    inst.last_used = Instant::now();
                    return Ok(());
                }
                // PID dead or unhealthy — remove stale entry
                instances.remove(&tier);
            }
        }

        // 2. Check if we have config for this tier
        let tier_name = tier.key();
        if !self.server_config.tiers.contains_key(tier_name) {
            return Err(format!("No server config for tier '{}'", tier_name));
        }

        // 3. Estimate memory needed
        let models_dir = expand_tilde(&self.server_config.models_dir);
        let gguf_path = models_dir.join(
            &self.server_config.tiers[tier_name].gguf
        );
        let gguf_size = std::fs::metadata(&gguf_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let needed = crate::memory::estimate_model_memory(gguf_size, self.memory_multiplier);

        // 4. Check available memory
        let mem = crate::memory::system_memory();
        let effective_available = mem.available.saturating_sub(self.memory_headroom);

        if effective_available < needed {
            // 5. Need to evict — build eviction plan
            let deficit = needed - effective_available;
            let instances = self.instances.read().await;
            let loaded: Vec<(ModelTier, u64, u64)> = instances.values()
                .map(|i| (i.tier, i.estimated_mem, i.last_used.elapsed().as_secs()))
                .collect();
            drop(instances);

            let plan = EvictionPlan::select(&loaded, deficit, tier);

            if plan.will_free < deficit {
                tracing::warn!(
                    "Cannot free enough memory: need {}GB, can free {}GB",
                    deficit / (1024*1024*1024),
                    plan.will_free / (1024*1024*1024)
                );
                // Try anyway — OS might reclaim buffers/caches
            }

            for evict_tier in &plan.tiers_to_evict {
                self.stop_server(*evict_tier).await?;
            }

            // Brief pause for OS to reclaim memory
            if !plan.tiers_to_evict.is_empty() {
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        // 6. Spawn the server
        self.spawn_server(tier).await?;
        Ok(())
    }

    /// Recover from a COMPUTE ERROR: evict non-essential models, verify health.
    pub async fn recover(&self, tier: ModelTier) -> Result<(), String> {
        tracing::warn!("Recovering {} from COMPUTE ERROR", tier.display_name());

        // Evict everything except the target tier and embed
        let instances = self.instances.read().await;
        let to_evict: Vec<ModelTier> = instances.keys()
            .filter(|t| **t != tier && **t != ModelTier::Embed)
            .copied()
            .collect();
        drop(instances);

        for t in to_evict {
            self.stop_server(t).await?;
        }

        // Wait for memory
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check if target is still healthy
        let url = self.url_for_tier(tier);
        if !check_health(&url).await {
            // Restart it
            self.instances.write().await.remove(&tier);
            self.spawn_server(tier).await?;
        }

        Ok(())
    }

    /// List currently loaded tiers.
    pub async fn loaded_tiers(&self) -> Vec<ModelTier> {
        self.instances.read().await.keys().copied().collect()
    }

    /// Stop all running servers.
    pub async fn stop_all(&self) {
        let tiers: Vec<ModelTier> = self.instances.read().await.keys().copied().collect();
        for tier in tiers {
            let _ = self.stop_server(tier).await;
        }
    }

    fn url_for_tier(&self, tier: ModelTier) -> String {
        self.server_config.tier_urls
            .get(tier.key())
            .cloned()
            .unwrap_or_else(|| format!("http://localhost:{}", 8080 + tier as u16))
    }

    async fn save_state(&self) {
        let instances = self.instances.read().await;
        let servers: HashMap<String, PersistedInstance> = instances.iter()
            .map(|(tier, inst)| {
                let key = tier.key().to_string();
                let pi = PersistedInstance {
                    pid: inst.pid,
                    port: inst.port,
                    tier: key.clone(),
                    model_name: inst.model_name.clone(),
                    gguf_path: inst.gguf_path.to_string_lossy().to_string(),
                    estimated_mem: inst.estimated_mem,
                    started_at: SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                };
                (key, pi)
            })
            .collect();
        let state = PersistedState { servers };
        if let Ok(json) = serde_json::to_string_pretty(&state) {
            let _ = std::fs::write(STATE_FILE, json);
        }
    }
}
```

- [ ] **Step 5: Add `key()` method to ModelTier if missing**

Check `oni_core::types::ModelTier` — if it lacks a `key()` method that returns `"heavy"`, `"medium"`, etc., add one:

```rust
impl ModelTier {
    pub fn key(&self) -> &'static str {
        match self {
            ModelTier::Heavy => "heavy",
            ModelTier::Medium => "medium",
            ModelTier::General => "general",
            ModelTier::Fast => "fast",
            ModelTier::Embed => "embed",
        }
    }
}
```

- [ ] **Step 6: Ensure old `ensure_servers_running()` still exists as a thin wrapper**

Replace the body of `ensure_servers_running()` so it delegates to `ServerManager`:

```rust
pub async fn ensure_servers_running_via(
    manager: &ServerManager,
    needed_tiers: Option<&[&str]>,
) -> Result<()> {
    let tiers: Vec<ModelTier> = match needed_tiers {
        Some(names) => names.iter()
            .filter_map(|n| ModelTier::from_key(n))
            .collect(),
        None => vec![
            ModelTier::Heavy, ModelTier::Medium,
            ModelTier::General, ModelTier::Fast, ModelTier::Embed,
        ],
    };

    for tier in tiers {
        if let Err(e) = manager.ensure_loaded(tier).await {
            eprintln!("  {} → {}", tier.display_name(), e);
        }
    }
    Ok(())
}
```

- [ ] **Step 7: Run full test suite**

Run: `cargo test`
Expected: All existing tests + new tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/server_manager.rs src/memory.rs
git commit -m "feat(server): memory-aware lifecycle with LRU eviction"
```

---

### Task 4: Config Additions

**Files:**
- Modify: `crates/oni-core/src/config.rs`

- [ ] **Step 1: Add memory config fields to ServerConfig**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    // ... existing fields ...

    /// Memory headroom to keep free (bytes). Default: 4GB.
    #[serde(default = "default_memory_headroom")]
    pub memory_headroom: u64,

    /// GGUF file size → runtime memory multiplier. Default: 1.3.
    #[serde(default = "default_memory_multiplier")]
    pub memory_multiplier: f64,
}

fn default_memory_headroom() -> u64 {
    4 * 1024 * 1024 * 1024 // 4 GB
}

fn default_memory_multiplier() -> f64 {
    1.3
}
```

Update the `Default` impl for `ServerConfig` to include these new fields.

- [ ] **Step 2: Add `from_key()` and `key()` to ModelTier if not present**

In `crates/oni-core/src/types.rs`, ensure `ModelTier` has:
- `from_key(s: &str) -> Option<ModelTier>` — parse "heavy", "medium", etc.
- `key(&self) -> &'static str` — return lowercase tier name

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add crates/oni-core/src/config.rs crates/oni-core/src/types.rs
git commit -m "feat(config): add memory headroom and multiplier settings"
```

---

### Task 5: TUI Integration + Error Recovery

**Files:**
- Modify: `crates/oni-tui/src/app.rs`
- Modify: `src/main.rs`

This is the critical task — wiring the `ServerManager` into the actual runtime.

- [ ] **Step 1: Create ServerManager in `main.rs` and pass to TUI**

In `src/main.rs`, inside `run_chat()`:
```rust
// After config is loaded, before launching TUI:
let server_manager = Arc::new(ServerManager::new(
    config.server.clone(),
    config.models.clone(),
    config.server.memory_headroom,
    config.server.memory_multiplier,
));
server_manager.restore_state().await;

// Replace old ensure_servers_running() call:
ensure_servers_running_via(&server_manager, needed_tiers.as_deref()).await;

// Pass to TUI:
oni_tui::run(
    router,
    db,
    db_path,
    agent_config,
    ui_config,
    models_config,
    server_config,
    server_manager.clone(), // NEW parameter
).await
```

- [ ] **Step 2: Update `oni_tui::run()` signature to accept `Arc<ServerManager>`**

Update the TUI's public `run()` function to take an `Arc<ServerManager>` and pass it into the agent task spawner.

- [ ] **Step 3: Replace `ensure_tier_server()` with ServerManager in agent task**

In `app.rs`, the `SetTier` handler (line 1219-1223):

```rust
AgentCommand::SetTier(new_tier) => {
    agent.set_tier(new_tier);
    // Replace ensure_tier_server with:
    match server_manager.ensure_loaded(new_tier).await {
        Ok(()) => {
            agent.event_bus.publish(AgentEvent::SystemMessage(
                format!("{} server ready", new_tier.display_name())
            ));
        }
        Err(e) => {
            agent.event_bus.publish(AgentEvent::Error(
                format!("Failed to load {}: {}", new_tier.display_name(), e)
            ));
        }
    }
}
```

- [ ] **Step 4: Add COMPUTE ERROR retry logic to the RunTurn handler**

In `app.rs`, the `RunTurn` handler (line 1211-1217):

```rust
AgentCommand::RunTurn(message) => {
    match agent.run_turn(&message).await {
        Ok(_) => {}
        Err(e) => {
            let err_str = e.to_string();
            // Detect COMPUTE ERROR — attempt recovery
            if err_str.contains("COMPUTE ERROR")
                || err_str.contains("compute error")
                || (err_str.contains("500") && err_str.contains("SERVER_ERROR"))
            {
                agent.event_bus.publish(AgentEvent::SystemMessage(
                    "COMPUTE ERROR detected — recovering...".into()
                ));
                let tier = agent.current_tier();
                match server_manager.recover(tier).await {
                    Ok(()) => {
                        // Retry ONCE
                        agent.event_bus.publish(AgentEvent::SystemMessage(
                            "Recovery complete — retrying...".into()
                        ));
                        match agent.run_turn(&message).await {
                            Ok(_) => {}
                            Err(retry_err) => {
                                agent.event_bus.publish(AgentEvent::Error(
                                    format!("Retry failed: {}", retry_err)
                                ));
                            }
                        }
                    }
                    Err(recover_err) => {
                        agent.event_bus.publish(AgentEvent::Error(
                            format!("Recovery failed: {}", recover_err)
                        ));
                    }
                }
            } else {
                agent.event_bus.publish(AgentEvent::Error(err_str));
            }
        }
    }
}
```

- [ ] **Step 5: Downgrade COMPUTE ERROR from critical_error to regular error**

In `app.rs`, update the `AgentEvent::Error` handler (line 960-976). The COMPUTE ERROR should no longer trigger `critical_error` since we handle recovery above. Add a check:

```rust
AgentEvent::Error(text) => {
    self.is_thinking = false;

    let lower = text.to_lowercase();

    // Compute errors are handled by retry logic — show as regular message
    if lower.contains("compute error") || lower.contains("retry failed") {
        self.messages.push(DisplayMessage::Error(text));
        return;
    }

    // Only truly unrecoverable errors go to critical screen
    let is_critical = lower.contains("connection refused")
        || lower.contains("failed to connect")
        || lower.contains("os error 111")
        || lower.contains("no such host");

    if is_critical {
        self.critical_error = Some(text);
    } else {
        self.messages.push(DisplayMessage::Error(text));
    }
}
```

- [ ] **Step 6: Add `SystemMessage` variant to AgentEvent if not present**

Check if `AgentEvent::SystemMessage(String)` exists. If not, add it and handle it in the TUI event handler — display as a styled system message (similar to Error but informational).

- [ ] **Step 7: Delete `ensure_tier_server()` function**

Remove the entire `ensure_tier_server()` function (lines 1943-2065) from `app.rs`. It's fully replaced by `ServerManager`.

- [ ] **Step 8: Build and test**

Run: `cargo build && cargo test`
Expected: PASS

- [ ] **Step 9: Commit**

```bash
git add src/main.rs crates/oni-tui/src/app.rs crates/oni-agent/src/agent.rs
git commit -m "feat(server): wire ServerManager into TUI with COMPUTE ERROR recovery"
```

---

### Task 6: Status Messages in TUI

**Files:**
- Modify: `crates/oni-tui/src/app.rs` (event handler)
- Modify: `crates/oni-tui/src/ui/chat.rs` (render system messages)

- [ ] **Step 1: Handle SystemMessage events**

In `app.rs`, add handler for `AgentEvent::SystemMessage`:
```rust
AgentEvent::SystemMessage(text) => {
    self.messages.push(DisplayMessage::System(text));
}
```

Add `System(String)` variant to `DisplayMessage` enum if not present.

- [ ] **Step 2: Render system messages in chat view**

In `chat.rs`, render `DisplayMessage::System` as a dim informational line:
```rust
DisplayMessage::System(text) => {
    lines.push(Line::from(Span::styled(
        format!("  ▸ {}", text),
        Style::default().fg(palette::MUTED).bg(palette::BG),
    )));
}
```

- [ ] **Step 3: Add loading status to server_manager operations**

In `ServerManager::ensure_loaded()` and `stop_server()`, emit events through a callback or return structured results that the TUI can display. Simplest approach: the caller (agent task in app.rs) publishes events before/after calling the manager:

```rust
// In SetTier handler:
agent.event_bus.publish(AgentEvent::SystemMessage(
    format!("Loading {} model...", new_tier.display_name())
));
match server_manager.ensure_loaded(new_tier).await {
    Ok(()) => {
        agent.event_bus.publish(AgentEvent::SystemMessage(
            format!("{} ready ✓", new_tier.display_name())
        ));
    }
    // ...
}
```

- [ ] **Step 4: Build, test, install**

Run: `cargo build && cargo test && cargo install --path .`
Expected: All pass, binary installed globally.

- [ ] **Step 5: Commit**

```bash
git add crates/oni-tui/src/app.rs crates/oni-tui/src/ui/chat.rs
git commit -m "feat(tui): system message display for server lifecycle events"
```

---

### Task 7: Cleanup + Final Integration

**Files:**
- Modify: `src/main.rs` (remove old ensure_servers_running calls)
- Modify: `src/server_manager.rs` (remove old function if fully replaced)

- [ ] **Step 1: Remove or deprecate old `ensure_servers_running()`**

If the old function is no longer called anywhere, remove it. If it's used in other code paths, make it delegate to `ServerManager`.

- [ ] **Step 2: Final full test suite run**

Run: `cargo test`
Expected: All 216+ tests pass.

- [ ] **Step 3: Manual test checklist**

Test these scenarios manually:
1. Start ONI fresh — default tier loads correctly
2. `/tier heavy` — switches with loading message
3. `/tier code` — evicts heavy first if memory tight, loads code
4. `/tier fast` — loads alongside code if enough memory
5. Rapid tier cycling — no crashes
6. Kill a llama-server externally — ONI detects and restarts on next use
7. All servers stopped externally — ONI restarts needed server

- [ ] **Step 4: Install globally**

Run: `cargo install --path .`

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "feat(server): intelligent LLM layer — memory-aware model lifecycle management"
```
