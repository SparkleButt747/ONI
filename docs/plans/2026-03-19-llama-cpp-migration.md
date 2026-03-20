# ONI — Ollama → llama.cpp Migration Plan

**Date:** 2026-03-19
**Scope:** Replace Ollama runtime with llama-server (llama.cpp). Multi-instance architecture (one server per tier).
**Risk:** High — changes the entire inference path. Mechanical but wide-reaching.

---

## Architecture Decision: Multi-Instance

llama-server loads **one GGUF at a time**. ONI uses 3-5 model tiers. Two options:

| Approach | Pros | Cons |
|----------|------|------|
| **Multi-instance (one port per tier)** | Clean separation, can tune params per model, models stay loaded | Multiple processes to manage |
| **Router mode (`--models-dir`)** | Single process, LRU eviction | New feature, less control, swap latency |

**Decision:** Multi-instance. Each tier gets its own llama-server on a dedicated port. A launcher script starts/stops them. This matches load-on-demand — only start a server when that tier is needed.

### Port assignments

```
Heavy  (MIMIR)   → :8081  (Qwen3.5-27B)
Medium (FENRIR)  → :8082  (Qwen3-Coder-Next)
General (SKULD)  → :8083  (GLM-4.7-Flash)
Fast             → :8084  (qwen3.5:9b — keep on Ollama until GGUF pulled)
Embed            → :8085  (nomic-embed-text — keep on Ollama or separate server)
```

---

## File-by-File Changes

### 1. `crates/oni-core/src/config.rs` — Config restructure

**Rename `OllamaConfig` → `ServerConfig`:**

```rust
// Before
pub struct OllamaConfig {
    pub base_url: String,
    pub timeout_secs: u64,
    pub keep_alive: i64,
}

// After
pub struct ServerConfig {
    pub base_url: String,          // Default: "http://localhost:8082" (medium tier)
    pub timeout_secs: u64,
    pub tier_urls: HashMap<String, String>,  // NEW: per-tier URL overrides
}
```

- [ ] Delete `keep_alive: i64` field and `default_keep_alive()` fn
- [ ] Add `tier_urls: HashMap<String, String>` with defaults for each tier
- [ ] Rename `OniConfig.ollama` → `OniConfig.server`
- [ ] Change `default_base_url()` from `:11434` to `:8082`
- [ ] Add backward-compat: if `[ollama]` section exists in TOML, read it as `[server]` with a tracing::warn

**Config example (`oni.toml`):**
```toml
[server]
timeout_secs = 300

[server.tier_urls]
heavy = "http://localhost:8081"
medium = "http://localhost:8082"
general = "http://localhost:8083"
fast = "http://localhost:8084"
embed = "http://localhost:8085"
```

### 2. `crates/oni-ollama/src/models.rs` — Request/Response types

**ChatRequest:**
```rust
// Before (Ollama)
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub keep_alive: Option<i64>,          // DELETE
    pub options: Option<HashMap<...>>,    // DELETE
    pub tools: Option<Vec<Value>>,
}

// After (OpenAI)
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    pub temperature: Option<f32>,         // NEW (was inside options)
    pub max_tokens: Option<u32>,          // NEW (was options.num_ctx)
    pub tools: Option<Vec<Value>>,
}
```

- [ ] Delete `keep_alive` field
- [ ] Delete `options` HashMap field
- [ ] Add `temperature: Option<f32>` (top-level, skip_serializing_if None)
- [ ] Add `max_tokens: Option<u32>` (top-level, skip_serializing_if None)

**ChatResponse:**
```rust
// Before (Ollama)
pub struct ChatResponse {
    pub model: String,
    pub message: ResponseMessage,
    pub done: bool,
    pub total_duration: Option<u64>,      // Nanoseconds — DELETE
    pub prompt_eval_count: Option<u64>,   // DELETE (moved to usage)
    pub eval_count: Option<u64>,          // DELETE (moved to usage)
    pub eval_duration: Option<u64>,       // DELETE
}

// After (OpenAI)
pub struct ChatResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Option<UsageStats>,
}

pub struct Choice {
    pub index: u32,
    pub message: ResponseMessage,
    pub finish_reason: Option<String>,    // "stop" | "tool_calls"
}

pub struct UsageStats {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}
```

- [ ] Add `Choice` and `UsageStats` structs
- [ ] Add accessor methods on ChatResponse for backward compat:
  ```rust
  impl ChatResponse {
      pub fn message(&self) -> &ResponseMessage {
          &self.choices[0].message
      }
      pub fn prompt_tokens(&self) -> u64 {
          self.usage.as_ref().map_or(0, |u| u.prompt_tokens)
      }
      pub fn completion_tokens(&self) -> u64 {
          self.usage.as_ref().map_or(0, |u| u.completion_tokens)
      }
  }
  ```

**ChatMessage — NO CHANGE.** Both Ollama and OpenAI use `{role, content, tool_calls}`.

**ToolCall / ToolCallFunction — NO CHANGE.** Same shape in both APIs.

**EmbedRequest/EmbedResponse:**
```rust
// Before (Ollama: /api/embed)
pub struct EmbedResponse {
    pub model: String,
    pub embeddings: Vec<Vec<f32>>,
}

// After (OpenAI: /v1/embeddings)
pub struct EmbedResponse {
    pub data: Vec<EmbeddingObject>,
}
pub struct EmbeddingObject {
    pub embedding: Vec<f32>,
    pub index: usize,
}
```

- [ ] Replace `embeddings` field with `data: Vec<EmbeddingObject>`
- [ ] Update `router.rs` and `embeddings.rs` accessors

**Delete:** `TagsResponse`, `ModelInfo` — no `/api/tags` equivalent.

### 3. `crates/oni-ollama/src/client.rs` — Endpoint swap

| Method | Before | After |
|--------|--------|-------|
| `health_check()` | `GET /api/tags` → `Vec<ModelInfo>` | `GET /health` → `{"status":"ok"}` |
| `chat()` | `POST /api/chat` | `POST /v1/chat/completions` |
| `embed()` | `POST /api/embed` | `POST /v1/embeddings` |
| `has_model()` | Checks `/api/tags` list | `GET /v1/models` → single loaded model |

- [ ] `health_check()` → return `Result<bool>` instead of `Result<Vec<ModelInfo>>`
- [ ] `chat()` → POST to `/v1/chat/completions`
- [ ] `embed()` → POST to `/v1/embeddings`
- [ ] `has_model()` → call `GET /v1/models`, check if model name matches
- [ ] Update all error strings: "Ollama" → "llama-server"
- [ ] Default port: `11434` → `8080` (but overridden by tier_urls config)

### 4. `crates/oni-ollama/src/router.rs` — Remove keep_alive, per-tier URLs

- [ ] Remove `keep_alive: i64` field
- [ ] Remove `keep_alive` from `ModelRouter::new()` signature
- [ ] Add `tier_urls: HashMap<ModelTier, String>` field
- [ ] Route requests to the correct URL based on tier:
  ```rust
  pub async fn chat(&self, tier: ModelTier, messages: Vec<ChatMessage>) -> Result<ChatResponse> {
      let url = self.tier_urls.get(&tier).unwrap_or(&self.client.base_url);
      // POST to {url}/v1/chat/completions
  }
  ```
- [ ] Replace `default_options()` with direct `temperature`/`max_tokens` on ChatRequest
- [ ] Update `embed()` accessor: `resp.data[0].embedding` instead of `resp.embeddings[0]`

### 5. `crates/oni-ollama/src/health.rs` — Simplify

- [ ] Rename `ollama_running` → `server_running`
- [ ] Instead of listing models from `/api/tags`, ping each tier URL's `/health`
- [ ] `check_health()` iterates `tier_urls`, hits `/health` on each, reports per-tier status

### 6. `crates/oni-agent/src/agent.rs` — Token stat accessors

5 occurrences of:
```rust
let prompt_tokens = response.prompt_eval_count.unwrap_or(0);
let completion_tokens = response.eval_count.unwrap_or(0);
let duration_ns = response.total_duration.unwrap_or(0);
```

Replace with:
```rust
let prompt_tokens = response.prompt_tokens();
let completion_tokens = response.completion_tokens();
// duration_ns: measure wall-clock time with Instant::elapsed()
```

- [ ] Add `let start = std::time::Instant::now();` before each chat call
- [ ] Replace `total_duration` with `start.elapsed().as_nanos() as u64`
- [ ] Use accessor methods for token counts

### 7. `crates/oni-agent/src/orchestrator.rs` — Same token stats

Same pattern, 5 occurrences. Same fix as agent.rs.

### 8. `crates/oni-agent/src/budget.rs` — Duration source

`total_duration_ns` is fed from Ollama's `total_duration`. After migration, it comes from wall-clock `Instant::elapsed()`. No structural change — just the caller passes different values.

### 9. `crates/oni-context/src/embeddings.rs` — Response shape

- [ ] `resp.embeddings[0]` → `resp.data[0].embedding`
- [ ] Error string: "Ollama" → "llama-server"

### 10. `src/main.rs` — Construction sites

7 occurrences of `OllamaClient::new(...)` + `ModelRouter::new(...)`:
- [ ] Remove `config.ollama.keep_alive` argument from all `ModelRouter::new()` calls
- [ ] Pass `config.server.tier_urls` to router
- [ ] Rename `config.ollama` → `config.server` in all references
- [ ] Update `run_doctor()`: replace "ollama pull" suggestions with GGUF file paths

### 11. `oni.toml` — Config update

```toml
# Before
[ollama]
base_url = "http://localhost:11434"
timeout_secs = 300
keep_alive = -1

# After
[server]
timeout_secs = 300

[server.tier_urls]
heavy = "http://localhost:8081"
medium = "http://localhost:8082"
general = "http://localhost:8083"
fast = "http://localhost:8084"
embed = "http://localhost:8085"
```

### 12. `crates/oni-ollama/src/lib.rs` — Update exports

- [ ] Remove `EmbedRequest` from re-exports (if unused externally)
- [ ] Add `Choice`, `UsageStats`, `EmbeddingObject` if needed externally
- [ ] Consider renaming crate from `oni-ollama` to `oni-llm` (optional, cosmetic)

---

## Crate rename (optional)

Rename `oni-ollama` → `oni-llm` to be backend-agnostic:
- Rename directory: `crates/oni-ollama/` → `crates/oni-llm/`
- Update `Cargo.toml` workspace members
- Update all `use oni_ollama::` → `use oni_llm::`
- Update `[dependencies]` in all crates

**Decision:** Do this rename as part of the migration. It's mechanical and makes the codebase honest about what it is.

---

## Launcher Script

Create `scripts/oni-servers.sh`:

```bash
#!/bin/bash
# Start/stop llama-server instances for each tier

MODELS_DIR="$HOME/.cache/llama.cpp/models"
LLAMA_SERVER=$(which llama-server)

case "$1" in
  start)
    echo "Starting ONI model servers..."

    # MIMIR (Heavy) — Qwen3.5-27B Dense (Planner)
    $LLAMA_SERVER \
      --model "$MODELS_DIR/Qwen3.5-27B-UD-Q8_K_XL.gguf" \
      --port 8081 --flash-attn --ctx-size 32768 \
      --cache-type-k q8_0 --cache-type-v q8_0 \
      --n-gpu-layers 99 --threads 8 --threads-batch 16 \
      --batch-size 512 --ubatch-size 512 \
      --parallel 2 --jinja \
      --reasoning-format deepseek \
      --chat-template-kwargs '{"enable_thinking":true}' \
      --temp 0.6 --top-k 20 --top-p 0.95 --min-p 0.0 \
      --repeat-penalty 1.0 \
      > /tmp/oni-heavy.log 2>&1 &
    echo "  Heavy (MIMIR) → :8081 [PID $!]"

    # FENRIR (Medium) — Qwen3-Coder-Next 80B MoE (Executor)
    $LLAMA_SERVER \
      --model "$MODELS_DIR/UD-Q6_K_XL/Qwen3-Coder-Next-UD-Q6_K_XL-00001-of-00003.gguf" \
      --port 8082 --flash-attn --ctx-size 65536 \
      --cache-type-k q4_0 --cache-type-v q4_0 \
      --n-gpu-layers 99 --threads 8 --threads-batch 16 \
      --batch-size 512 --ubatch-size 256 \
      --parallel 1 --jinja \
      --temp 1.0 --top-k 40 --top-p 0.95 --min-p 0.01 \
      --repeat-penalty 1.0 \
      > /tmp/oni-medium.log 2>&1 &
    echo "  Medium (FENRIR) → :8082 [PID $!]"

    # SKULD (General) — GLM-4.7-Flash 30B MoE (Critic)
    $LLAMA_SERVER \
      --model "$MODELS_DIR/GLM-4.7-Flash-UD-Q8_K_XL.gguf" \
      --port 8083 --flash-attn --ctx-size 32768 \
      --cache-type-k q4_0 --cache-type-v q4_0 \
      --n-gpu-layers 99 --threads 8 --threads-batch 16 \
      --batch-size 512 --ubatch-size 256 \
      --parallel 2 --jinja \
      --temp 0.7 --top-p 1.0 --min-p 0.01 \
      --repeat-penalty 1.0 \
      > /tmp/oni-general.log 2>&1 &
    echo "  General (SKULD) → :8083 [PID $!]"

    echo "All servers started. Logs in /tmp/oni-*.log"
    ;;

  stop)
    echo "Stopping ONI model servers..."
    pkill -f "llama-server.*--port 808[1-5]"
    echo "Done."
    ;;

  status)
    for port in 8081 8082 8083 8084 8085; do
      if curl -s "http://localhost:$port/health" >/dev/null 2>&1; then
        model=$(curl -s "http://localhost:$port/v1/models" | python3 -c "import sys,json; print(json.load(sys.stdin)['data'][0]['id'])" 2>/dev/null)
        echo "  :$port — UP ($model)"
      else
        echo "  :$port — DOWN"
      fi
    done
    ;;

  *)
    echo "Usage: $0 {start|stop|status}"
    exit 1
    ;;
esac
```

---

## Migration Order

1. **Types first** (`models.rs`) — define new structs alongside old ones with `#[cfg]` or just replace
2. **Client** (`client.rs`) — swap endpoints
3. **Router** (`router.rs`) — remove keep_alive, add tier_urls
4. **Config** (`config.rs`) — rename section, add tier_urls
5. **Health** (`health.rs`) — per-tier health check
6. **Agent/Orchestrator** — swap to accessor methods
7. **Embeddings** — swap response shape
8. **main.rs** — update construction sites
9. **oni.toml** — new config format
10. **Launcher script** — create and test
11. **Tests** — update all Ollama-specific test expectations
12. **Rename crate** — oni-ollama → oni-llm (last, mechanical)

---

## Test Strategy

- All 216 existing tests must pass (most don't hit the network)
- Add integration tests that verify:
  - Request serialisation matches OpenAI JSON format
  - Response deserialisation handles llama-server's exact output
  - Health check works against a running llama-server
  - Per-tier URL routing sends to correct port
  - Fallback when a tier's server is down
- The 4 Ollama integration tests in `ollama_integration.rs` get renamed/rewritten

---

## Rollback Plan

If migration breaks:
1. Ollama models are NOT deleted until migration is fully verified
2. Config has `[ollama]` backward-compat reading
3. Can switch back by reverting `oni.toml` to `[ollama]` section
4. `ollama serve` starts the old runtime immediately

---

## Post-Migration Cleanup (ONLY after full verification)

- [ ] `ollama rm qwen3.5:35b qwen3-coder:30b glm-4.7-flash:q4_k_m qwen3.5:9b`
- [ ] `brew services stop ollama` (or `brew uninstall ollama` if no longer needed)
- [ ] Remove `[ollama]` backward-compat from config parser
- [ ] Update CLAUDE.md, MODEL_UPGRADES.md

---

## Optimal Parameters Per Model

### MIMIR — Qwen3.5-27B Dense (Planner)

```bash
llama-server \
  --model "$MODELS/Qwen3.5-27B-UD-Q8_K_XL.gguf" \
  --port 8081 \
  --ctx-size 32768 \
  --n-gpu-layers 99 \
  --flash-attn \
  --cache-type-k q8_0 --cache-type-v q8_0 \
  --threads 8 --threads-batch 16 \
  --batch-size 512 --ubatch-size 512 \
  --parallel 2 \
  --jinja \
  --reasoning-format deepseek \
  --chat-template-kwargs '{"enable_thinking":true}' \
  --temp 0.6 --top-k 20 --top-p 0.95 --min-p 0.0 \
  --repeat-penalty 1.0
```

| Flag | Value | Reason |
|------|-------|--------|
| ctx-size | 32768 | Native 262K but q8_0 KV costs ~2GB/slot at 32K. Sufficient for planning |
| cache-type | q8_0/q8_0 | Dense model — q8_0 KV cuts memory 50% vs bf16 with negligible quality loss |
| temp | 0.6 | Official Qwen3.5 recommendation for thinking mode |
| top-k | 20 | Official Qwen3.5 recommendation (all modes) |
| reasoning-format | deepseek | Enables `<think>` block extraction. Planner SHOULD think |
| threads | 8 / 16 batch | TG is bandwidth-bound (8 P-cores), PP is compute-bound (16 P-cores) |
| parallel | 2 | One hot + one warm. Planner rarely concurrent |

**Speculative decoding:** BLOCKED — llama.cpp Issue #20039. Qwen3.5-0.8B as draft model crashes. PR #20700 (MTP support) in draft. Skip for now, revisit when merged.

**Chat template:** Use `--jinja` with GGUF-embedded template. Do NOT use `--chat-template qwen3`. Tool calling uses Hermes-style `<tool_call>` JSON. Known issue #19872 may report "does not natively describe tools" — functional, cosmetic warning.

---

### FENRIR — Qwen3-Coder-Next 80B MoE (Executor)

```bash
llama-server \
  --model "$MODELS/UD-Q6_K_XL/Qwen3-Coder-Next-UD-Q6_K_XL-00001-of-00003.gguf" \
  --port 8082 \
  --ctx-size 65536 \
  --n-gpu-layers 99 \
  --flash-attn \
  --cache-type-k q4_0 --cache-type-v q4_0 \
  --threads 8 --threads-batch 16 \
  --batch-size 512 --ubatch-size 256 \
  --parallel 1 \
  --jinja \
  --temp 1.0 --top-k 40 --top-p 0.95 --min-p 0.01 \
  --repeat-penalty 1.0
```

| Flag | Value | Reason |
|------|-------|--------|
| ctx-size | 65536 | Code editing benefits from 64K+. At q4_0 KV on MoE, costs ~4GB/slot |
| cache-type | q4_0/q4_0 | MoE has many attention heads but only 3B active params — aggressive KV quant is safe |
| temp | 1.0 | Official Qwen3-Coder-Next recommendation |
| top-k | 40 | Higher than Qwen3.5's 20 — non-thinking model needs more breadth |
| ubatch-size | 256 | Smaller than default; MoE expert routing adds overhead per micro-batch |
| parallel | 1 | 80B MoE is memory-intensive; single slot avoids KV doubling |

**Speculative decoding:** NOT recommended for MoE (poor draft acceptance rates). ngram-mod IS viable for code:
```
--spec-type ngram-mod --spec-ngram-size-n 24 --draft-min 48 --draft-max 64
```
No draft model needed. Native Metal support.

**Chat template:** `--jinja` with embedded template. This model does NOT think — do not pass `enable_thinking`. Tool calls use Hermes-style `<tool_call>` format.

---

### SKULD — GLM-4.7-Flash 30B MoE (Critic)

```bash
llama-server \
  --model "$MODELS/GLM-4.7-Flash-UD-Q8_K_XL.gguf" \
  --port 8083 \
  --ctx-size 32768 \
  --n-gpu-layers 99 \
  --flash-attn \
  --cache-type-k q4_0 --cache-type-v q4_0 \
  --threads 8 --threads-batch 16 \
  --batch-size 512 --ubatch-size 256 \
  --parallel 2 \
  --jinja \
  --temp 0.7 --top-p 1.0 --min-p 0.01 \
  --repeat-penalty 1.0
```

| Flag | Value | Reason |
|------|-------|--------|
| ctx-size | 32768 | Critic reviews individual outputs, not full sessions. 32K sufficient |
| cache-type | q4_0/q4_0 | Same MoE rationale as FENRIR |
| temp | 0.7 | Z.ai official recommendation for tool calling / reasoning review |
| top-p | 1.0 | Z.ai explicitly recommends 1.0 (not 0.95) for tool/reasoning tasks |
| parallel | 2 | Critic may pipeline multiple reviews |

**Speculative decoding:** ngram-mod viable. SKULD writes short structured judgements (accept/reject + rationale) — highly repetitive:
```
--spec-type ngram-mod --spec-ngram-size-n 24 --draft-min 32 --draft-max 48
```

**Chat template:** `--jinja` only — do NOT use `--chat-template chatml` or any named template. GLM embedded Jinja is the canonical format. Known issue #19009: "does not natively describe tools" warning — cosmetic, tool calls still work. Ensure GGUF is post-Jan-21-2026 upload (Unsloth fixed templates).

---

### Quick Reference

| Model | Role | Port | ctx | cache-k/v | temp | top-k | top-p | min-p | Spec decode |
|-------|------|------|-----|-----------|------|-------|-------|-------|-------------|
| Qwen3.5-27B | MIMIR | 8081 | 32K | q8_0/q8_0 | 0.6 | 20 | 0.95 | 0.0 | Blocked (bug) |
| Qwen3-Coder-Next 80B | FENRIR | 8082 | 64K | q4_0/q4_0 | 1.0 | 40 | 0.95 | 0.01 | ngram-mod |
| GLM-4.7-Flash 30B | SKULD | 8083 | 32K | q4_0/q4_0 | 0.7 | — | 1.0 | 0.01 | ngram-mod |

### Hardware Notes (M4 Max 128GB)

| Topic | Value |
|-------|-------|
| Threads (TG) | 8 — bandwidth-bound on Metal, more threads = contention |
| Threads (PP) | 16 — compute-bound, use all P-cores |
| GPU layers | Always 99 — all models fit in unified memory |
| Memory budget | ~35.5 + ~60 + ~32 = ~128 GB weights. **Cannot run all three simultaneously at these quants.** Load-on-demand is correct. |
| Flash attention | Enable on all three |
| KV quant | q8_0 for dense (MIMIR), q4_0 for MoE (FENRIR/SKULD) |

### Sources

- [Qwen3.5 docs](https://qwen.readthedocs.io/en/latest/run_locally/llama.cpp.html) — official params
- [Unsloth model docs](https://unsloth.ai/docs/models/) — per-model configs
- [llama.cpp speculative.md](https://github.com/ggml-org/llama.cpp/blob/master/docs/speculative.md) — MoE spec decoding limits
- [llama.cpp Issue #20039](https://github.com/ggml-org/llama.cpp/issues/20039) — Qwen3.5 spec decode crash
- [llama.cpp Issue #19872](https://github.com/ggml-org/llama.cpp/issues/19872) — tool template warning
- [KVSplit research](https://news.ycombinator.com/item?id=44009321) — K8V4 asymmetric quant on Apple Silicon
