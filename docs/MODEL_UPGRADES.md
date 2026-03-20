# ONI — Model Upgrade Proposals

**Machine:** M4 Max, 128GB unified memory, 40 GPU cores
**Runtime:** Ollama (GGUF format, batch mode, `stream: false`) — see [llama.cpp migration](#runtime-upgrade-ollama--llamacpp) below
**Date:** 2026-03-19

---

## Current Stack

| Role | Model | Size | Weaknesses |
|------|-------|------|------------|
| MIMIR (Planner) | `qwen3.5:35b` (MoE, 3B active) | 23 GB | Only 3B active params — shallow reasoning for planning |
| FENRIR (Executor) | `qwen3-coder:30b` | 18 GB | SWE-bench ~59%, struggles with multi-file edits |
| SKULD (Critic) | `glm-4.7-flash:q4_k_m` (MoE, 3B active) | 19 GB | Standard quant — Dynamic 2.0 would improve quality at same size |
| Fast | `qwen3.5:9b` | 6.6 GB | Fine for simple tasks |
| Embed | `nomic-embed-text` | 274 MB | Works well |

**Total loaded:** ~67 GB (with `keep_alive = -1` all models stay resident)

---

## Unsloth Dynamic 2.0 Quantisation

All upgrades use Unsloth Dynamic 2.0 GGUFs:

- **Per-layer quantisation** — each layer gets its own quant type to minimise that layer's accuracy loss
- **Custom calibration** — 300K-1.5M tokens of curated data per model
- **~7.5% better KL divergence** than standard imatrix at equivalent file sizes
- **`UD-` prefix** — `XL` suffix means important layers upcast to higher precision
- **Ollama compatible** — `ollama run hf.co/unsloth/<model>:<quant>`

---

## Chosen Upgrades (load-on-demand)

Strategy: one model loaded at a time, highest quality quants. Swap latency ~2-5 seconds.

### 1. FENRIR (Executor/Coding): Qwen3-Coder-Next 80B — UD-Q6_K_XL

**Architecture:** 80B MoE, 512 experts, 10 active per token (~3B active). Hybrid Gated DeltaNet + Gated Attention.
**Context:** 262,144 tokens (256K native)
**Chosen quant:** UD-Q6_K_XL

| Benchmark | Qwen3-Coder-Next | Current (qwen3-coder:30b) |
|-----------|-------------------|---------------------------|
| SWE-bench Verified | **70.6%** | ~59% |
| LiveCodeBench v6 | **74.5%** | — |
| HumanEval | **94.1%** | — |
| SWE-bench Pro | **44.3%** | — |

**Why:** Massive coding quality leap. Only 3B active params per token so inference speed is comparable to the current 30B. 256K context handles large codebases. SWE-bench 70.6% is elite for local.

```
ollama run hf.co/unsloth/Qwen3-Coder-Next-GGUF:UD-Q6_K_XL
```

---

### 2. MIMIR (Planner/Reasoning): Qwen3.5-27B Dense — UD-Q8_K_XL

**Architecture:** 27B dense (ALL 27B params active per token — 9x more compute than current 35B MoE).
**Context:** 262,144 tokens native, up to 1M with YaRN
**Chosen quant:** UD-Q8_K_XL (35.5 GB)

| Benchmark | Qwen3.5-27B (dense) | Current (qwen3.5:35b MoE) |
|-----------|----------------------|---------------------------|
| MMLU-Pro | **86.1** | 85.3 |
| SWE-bench Verified | **72.4%** | — |
| LiveCodeBench v6 | **80.7** | 74.6 |
| IFEval | **95.0** | 91.9 |
| CodeForces | 1899 | 2028 |
| MMMU | **82.3** | 81.4 |

**Why:** Dense = all 27B params active vs only 3B in current MoE. Sharper reasoning, instruction following (IFEval 95.0 vs 91.9), and coding (SWE-bench 72.4%). For planning/decomposition, IFEval matters more than CodeForces.

```
ollama run hf.co/unsloth/Qwen3.5-27B-GGUF:UD-Q8_K_XL
```

---

### 3. SKULD (Critic): GLM-4.7-Flash — UD-Q8_K_XL

**Architecture:** 30B MoE, 3B active. Same model, highest quality Dynamic 2.0 quant.
**Chosen quant:** UD-Q8_K_XL

| Benchmark | GLM-4.7-Flash |
|-----------|---------------|
| SWE-bench Verified | 59.2% |
| AIME 2025 | 91.6% |
| GPQA | 75.2% |
| LiveCodeBench v6 | 64.0% |
| t2-Bench (agent) | 79.5% |

**Why:** Already the best model for its class on agentic benchmarks (t2-Bench 79.5%). UD-Q8_K_XL gives near-lossless quality — significant jump over the current standard Q4_K_M.

```
ollama run hf.co/unsloth/GLM-4.7-Flash-GGUF:UD-Q8_K_XL
```

---

## oni.toml Changes

```toml
[models]
heavy = "hf.co/unsloth/Qwen3.5-27B-GGUF:UD-Q8_K_XL"
medium = "hf.co/unsloth/Qwen3-Coder-Next-GGUF:UD-Q6_K_XL"
general = "hf.co/unsloth/GLM-4.7-Flash-GGUF:UD-Q8_K_XL"
fast = "qwen3.5:9b"
embed = "nomic-embed-text"
default_tier = "Medium"

[ollama]
keep_alive = 300  # 5 minutes — load on demand, unload when idle
```

---

## Other Models Considered

| Model | Why not |
|-------|---------|
| Nemotron-3-Super-120B (MoE, 12B active) | UD-Q4_K_S at 79GB fits but 12B active = slower. SWE-bench 60.5%. Overkill for single role |
| Nemotron-3-Nano-30B (MoE, 3.5B active) | AIME25 99.2% but SWE-bench 38.8%. Math beast, weak coder |
| gpt-oss-20b (OpenAI, MoE, 3.6B active) | ~12GB but weaker than GLM-4.7-Flash on agentic benchmarks |
| gpt-oss-120b (OpenAI, MoE, 5.1B active) | All quants ~62-65GB (suspicious). No published benchmarks |
| GLM-5 (754B MoE, 40B active) | Smallest quant 176GB. Does not fit on 128GB |

---

## Runtime Upgrade: Ollama → llama.cpp

Ollama is a Go wrapper around llama.cpp. Moving to llama-server directly unlocks features Ollama doesn't expose and removes overhead.

### What you gain

| Feature | Ollama | llama.cpp (llama-server) |
|---------|--------|--------------------------|
| Speculative decoding | Not supported (issue #9216, PR #8134 unmerged for 1+ year) | Full support (`--model-draft`, `--draft N`). **2-3x throughput** on code/structured output |
| KV cache quant types | q8_0, q4_0, f16 only | f32/f16/bf16/q8_0/q4_0/q4_1/iq4_nl/q5_0/q5_1 — independent K and V type control |
| Flash attention | Off by default, env var to enable | On by default (`-fa auto`) |
| Context window | Conservative VRAM reservation — 11K on 16GB where llama.cpp gets 32K | Full `--ctx-size` control, no silent truncation |
| Parallel slots | OLLAMA_NUM_PARALLEL — limited, spills to CPU under load | `-np N` — stays on GPU, proper KV cache management |
| Split-GGUF | Not supported (issue #5245) | Native multi-file GGUF loading |
| YaRN context extension | Limited RoPE scaling via Modelfile | Full `--rope-scaling yarn` + `--yarn-orig-ctx` |
| Per-layer GPU offload | Automatic, no override | `-ngl N` — exact control over what hits Metal |
| New quant type support | Lags upstream by weeks-months (pinned llama.cpp fork) | Day-1 support for new GGUF types |
| Multi-GPU | Not supported | `--split-mode layer/row` + `--tensor-split` |
| Grammar/structured output | Basic | Full GBNF grammar enforcement + JSON schema |
| LoRA hot-swap | Modelfile ADAPTER only | Runtime `/lora` endpoint |

### Performance delta

| Metric | llama.cpp | Ollama | Gap |
|--------|-----------|--------|-----|
| Single-user generation | Baseline | 5-6% slower | Go HTTP layer overhead |
| 5 parallel requests | ~25 t/s sustained | ~8 t/s (CPU spill) | 3x slower under load |
| VRAM overhead (7B Q4_K_M) | 6.2 GB | 6.8 GB | ~10% more |
| Context achievable on 16GB | 32K tokens | 11K tokens | Ollama reserves conservatively |

Source: [NeuralNet Solutions benchmark](https://neuralnet.solutions/ollama-vs-llama-cpp-which-framework-is-better-for-inference)

### M4 Max native Metal benchmarks (llama.cpp)

| Model | Prompt (pp512) | Generation (tg128) |
|-------|---------------|-------------------|
| LLaMA 7B Q4_0 | 713.9 t/s | 70.0 t/s |

Source: [llama.cpp Discussion #4167](https://github.com/ggml-org/llama.cpp/discussions/4167)

These are the raw Metal ceiling. Ollama runs 5-20% below this.

### What you lose

| Loss | Mitigation |
|------|-----------|
| `ollama pull` one-liner | `llama-server -hf org/repo:tag` pulls from HuggingFace directly |
| Modelfile (system prompt, template, params baked in) | `--system-prompt` and `--chat-template` flags, or YAML config with llama-swap |
| `keep_alive` auto-unload | llama-swap handles this; router mode has LRU eviction |
| `ollama list` / `ollama ps` | `/health` and `/models` endpoints |
| macOS auto-start (launchd) | Write a plist or use llama-swap as daemon |

### Migration path

**1. Install**
```bash
brew install llama.cpp
# or from source for latest Metal optimisations:
cmake -B build -DGGML_METAL=ON -DGGML_NATIVE=ON && cmake --build build --config Release
```

**2. Start as OpenAI-compatible endpoint**
```bash
llama-server \
  --hf unsloth/Qwen3-Coder-Next-GGUF:UD-Q6_K_XL \
  --port 11434 \
  --flash-attn \
  --ctx-size 32768 \
  --cache-type-k q8_0 \
  --cache-type-v q4_0 \
  --parallel 4 \
  --gpu-layers 99
```

Exposes `/v1/chat/completions` — same as Ollama's OpenAI-compatible endpoint.

**3. Router mode (multi-model, Ollama-like)**
```bash
llama-server --models-dir ~/.cache/llama.cpp --models-max 3
```
Auto-discovers GGUFs, routes by `model` field, LRU evicts when limit hit.

**4. Speculative decoding (the big win)**
```bash
llama-server \
  --model Qwen3.5-27B-UD-Q8_K_XL.gguf \
  --model-draft qwen3.5-0.5b-q8_0.gguf \
  --draft 8
```
2-3x throughput on high-draftability outputs (code generation, structured JSON).

### ONI code changes needed

The `oni-ollama` crate calls Ollama's `/api/chat` endpoint. To switch to llama-server:
- Change the endpoint to `/v1/chat/completions` (OpenAI format)
- Adjust request/response serialisation to match OpenAI schema
- Or: keep Ollama's API format and run llama-server behind a thin proxy that translates

The cleanest approach: add a `backend` config option (`ollama` | `llama-cpp`) and implement both request formats in `oni-ollama/src/client.rs`.

### MLX note

Apple's MLX framework is 20-30% faster than llama.cpp on Apple Silicon ([arXiv 2511.05502](https://arxiv.org/abs/2511.05502)), but lacks the GGUF ecosystem, speculative decoding, and grammar support. Not recommended as primary backend yet, but worth monitoring.

---

## Sources

- [Unsloth Dynamic 2.0 GGUFs Blog](https://unsloth.ai/blog/dynamic-v2)
- [Unsloth Dynamic 2.0 Documentation](https://unsloth.ai/docs/basics/unsloth-dynamic-2.0-ggufs)
- [Unsloth Dynamic 2.0 Quants Collection](https://huggingface.co/collections/unsloth/unsloth-dynamic-20-quants)
- [Qwen3-Coder-Next Blog](https://qwen.ai/blog?id=qwen3-coder-next)
- [Qwen3-Coder-Next Benchmarks](https://www.marc0.dev/en/blog/qwen3-coder-next-70-swe-bench-with-3b-active-params-local-ai-just-got-real-1770197534528)
- [Qwen3.5 27B vs 35B Comparison](https://artificialanalysis.ai/models/comparisons/qwen3-5-35b-a3b-vs-qwen3-5-27b)
- [llama.cpp server README](https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md)
- [llama.cpp Apple Silicon benchmarks](https://github.com/ggml-org/llama.cpp/discussions/4167)
- [Ollama vs llama.cpp benchmarks](https://neuralnet.solutions/ollama-vs-llama-cpp-which-framework-is-better-for-inference)
- [llama.cpp vs Ollama vs vLLM comparison](https://insiderllm.com/guides/llamacpp-vs-ollama-vs-vllm/)
- [KV cache quantisation in Ollama](https://smcleod.net/2024/12/bringing-k/v-context-quantisation-to-ollama/)
- [llama.cpp model management](https://huggingface.co/blog/ggml-org/model-management-in-llamacpp)
- [MLX vs llama.cpp on Apple Silicon](https://arxiv.org/abs/2511.05502)
- [Ollama speculative decoding issue #9216](https://github.com/ollama/ollama/issues/9216)
