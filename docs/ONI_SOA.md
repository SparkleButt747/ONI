# ONI — State of Architecture

**Date:** 2026-03-20
**Branch:** master @ `6a24782`
**Hardware:** Apple M4 Max, 128 GB unified memory
**Runtime:** llama.cpp (llama-server v8420), Rust 2021 edition, Tokio async

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Crate Dependency Graph](#2-crate-dependency-graph)
3. [Runtime Architecture](#3-runtime-architecture)
4. [The 3-Agent System](#4-the-3-agent-system)
5. [Request Lifecycle](#5-request-lifecycle)
6. [CLI Entry Point](#6-cli-entry-point)
7. [LLM Transport Layer (oni-llm)](#7-llm-transport-layer-oni-llm)
8. [Agent Intelligence Layer (oni-agent)](#8-agent-intelligence-layer-oni-agent)
9. [Terminal UI (oni-tui)](#9-terminal-ui-oni-tui)
10. [Persistence Layer (oni-db)](#10-persistence-layer-oni-db)
11. [Context Engine (oni-context)](#11-context-engine-oni-context)
12. [Core Types and Config (oni-core)](#12-core-types-and-config-oni-core)
13. [Personality System](#13-personality-system)
14. [Knowledge Graph](#14-knowledge-graph)
15. [Preference Learning](#15-preference-learning)
16. [Telemetry and Ablation](#16-telemetry-and-ablation)
17. [Server Management](#17-server-management)
18. [Benchmark Infrastructure](#18-benchmark-infrastructure)
19. [Eval Framework](#19-eval-framework)
20. [Known Limitations](#20-known-limitations)

---

## 1. System Overview

ONI (Onboard Native Intelligence) is a local-first AI assistant that runs entirely on-device. No cloud APIs. Three local LLM instances serve different roles through an OpenAI-compatible HTTP interface. A Rust agent loop orchestrates multi-step tasks with tool access, while a Ratatui terminal UI provides the interactive surface.

```
┌─────────────────────────────────────────────────────────────────────┐
│                         USER TERMINAL                               │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                    oni-tui (Ratatui)                          │  │
│  │  ChromaStripe │ StatusBar │ Chat/MC/Prefs │ Input │ Footer   │  │
│  └──────────────────────────┬────────────────────────────────────┘  │
│                             │ AgentEvent bus                        │
│  ┌──────────────────────────▼────────────────────────────────────┐  │
│  │                    oni-agent                                   │  │
│  │  Agent loop ─── should_orchestrate? ──┬── single turn         │  │
│  │                                       └── Orchestrator        │  │
│  │  11 Tools │ KG │ Preferences │ Parsing │ Telemetry │ Budget   │  │
│  └──────────────────────────┬────────────────────────────────────┘  │
│                             │ HTTP (OpenAI-compat)                   │
│  ┌──────────┐  ┌──────────┐│ ┌──────────┐                          │
│  │ MIMIR    │  │ FENRIR   ││ │ SKULD    │                          │
│  │ :8081    │  │ :8082    ││ │ :8083    │                          │
│  │ Heavy    │  │ Medium   │▼ │ General  │                          │
│  │ Planner  │  │ Executor │  │ Critic   │                          │
│  └──────────┘  └──────────┘  └──────────┘                          │
│   llama-server  llama-server  llama-server                          │
│   Qwen3.5-27B   Qwen3-Coder  GLM-4.7-Flash                        │
│   Q8_K_XL       Q6_K_XL      Q8_K_XL                               │
└─────────────────────────────────────────────────────────────────────┘
          │                │               │
          ▼                ▼               ▼
    ┌──────────┐    ┌──────────┐    ┌──────────┐
    │ .gguf    │    │ .gguf    │    │ .gguf    │
    │ model    │    │ model    │    │ model    │
    │ weights  │    │ weights  │    │ weights  │
    └──────────┘    └──────────┘    └──────────┘
          GPU unified memory (128 GB shared)
```

**Core principle:** Everything runs locally. The only network calls are localhost HTTP to llama-server instances. No telemetry, no cloud, no API keys.

---

## 2. Crate Dependency Graph

```
                    oni (binary, src/main.rs)
                    ├── clap (CLI parsing)
                    ├── libc (Unix process control)
                    ├── which (binary lookup)
                    │
          ┌─────────┼──────────┬──────────┬──────────┐
          ▼         ▼          ▼          ▼          ▼
      oni-tui   oni-agent   oni-llm   oni-db   oni-context
          │         │          │         │          │
          │    ┌────┤          │         │          │
          │    │    │          │         │          │
          │    ▼    ▼          │         │          │
          │  oni-llm oni-db   │         │          │
          │    │    │          │         │          │
          │    │    ▼          │         │          │
          │    │  oni-context  │         │          │
          │    │    │          │         │          │
          ▼    ▼    ▼          ▼         ▼          ▼
          └────┴────┴──────────┴─────────┴──► oni-core
                                               (leaf)
```

`oni-core` is the dependency root — every other crate depends on it, and it depends on nothing internal. This is a strict DAG with no cycles.

| Crate | Role | Key deps |
|-------|------|----------|
| `oni-core` | Config, types, personality, error, palette | `serde`, `toml`, `color-eyre`, `ratatui` (Color), `dirs` |
| `oni-llm` | HTTP client, tier routing, health checks | `reqwest`, `tokio` |
| `oni-db` | SQLite persistence (conversations, prefs) | `rusqlite` (bundled), `uuid` |
| `oni-context` | File indexing, FTS5 retrieval, embeddings | `rusqlite`, `ignore`, `regex`, `notify` |
| `oni-agent` | Agent loop, orchestrator, tools, KG, telemetry | `tokio`, `serde_yaml`, `neo4rs`, `uuid` |
| `oni-tui` | Ratatui terminal UI, event loop, widgets | `ratatui`, `crossterm`, `tui-textarea` |

---

## 3. Runtime Architecture

### Process Model

ONI runs as **4 OS processes** during a typical interactive session:

| Process | Binary | PID management | Memory |
|---------|--------|----------------|--------|
| ONI main | `oni` (Rust binary) | foreground | ~50 MB RSS |
| MIMIR server | `llama-server` | background, PID file | ~28 GB (Q8 27B model) |
| FENRIR server | `llama-server` | background, PID file | ~22 GB (Q6 30B model) |
| SKULD server | `llama-server` | background, PID file | ~10 GB (Q8 9B model) |

Total GPU memory: ~60 GB for all three models. The remaining 68 GB is available for KV cache and OS.

### Network Topology

All communication is **localhost HTTP**. No Unix sockets, no gRPC, no IPC.

```
oni binary ──HTTP POST──► localhost:8081/v1/chat/completions  (MIMIR)
           ──HTTP POST──► localhost:8082/v1/chat/completions  (FENRIR)
           ──HTTP POST──► localhost:8083/v1/chat/completions  (SKULD)
           ──HTTP POST──► localhost:8085/v1/embeddings        (embed, if configured)
           ──HTTP GET───► localhost:808x/health                (health checks)
```

### Threading Model

```
┌─ Main thread (Tokio runtime) ─────────────────────────────────┐
│                                                                │
│  ┌─ TUI event loop ─────────────────────────────────────────┐ │
│  │  terminal.draw() → poll crossterm → drain MessageBus     │ │
│  │  → check proposals → advance animations → repeat         │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  ┌─ Agent task (tokio::spawn) ──────────────────────────────┐ │
│  │  recv AgentCommand → run_turn() → publish AgentEvent     │ │
│  │  HTTP calls to llama-server (reqwest async)              │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
│  ┌─ FileWatcher thread (notify) ────────────────────────────┐ │
│  │  inotify/kqueue → index changed files                    │ │
│  └──────────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────────┘
```

The `Tool` trait is **synchronous** (`fn execute`). Tools that need async (HTTP fetch, ask_user) use `tokio::task::block_in_place(|| Handle::current().block_on(...))` to bridge into the async runtime without deadlocking.

---

## 4. The 3-Agent System

ONI implements a plan-execute-critique loop with three specialised agents, each backed by a different model optimised for its role:

### MIMIR [Σ] — The Planner (Heavy tier)

| Property | Value |
|----------|-------|
| Model | Qwen3.5-27B-UD-Q8_K_XL |
| Port | 8081 |
| Context | 32,768 tokens |
| Temperature | 0.3 |
| Role | Decompose complex tasks into numbered step lists |
| Prompt style | "Return ONLY a numbered list of steps. No explanation." |

MIMIR receives the user's original prompt and produces a `Vec<String>` of steps. It also handles replanning when SKULD rejects a step — receiving the failure context and generating revised steps.

### FENRIR [Ψ] — The Executor (Medium tier)

| Property | Value |
|----------|-------|
| Model | Qwen3-Coder-Next-UD-Q6_K_XL |
| Port | 8082 |
| Context | 32,768 tokens |
| Temperature | 0.2 |
| Role | Execute individual steps with full tool access |
| Prompt style | "Do NOT ask for clarification. Execute the step directly." |

FENRIR runs the same tool loop as single-turn mode — it can call any of the 11 tools, process results, and iterate until the step is complete.

### SKULD [⊘] — The Critic (General tier)

| Property | Value |
|----------|-------|
| Model | GLM-4.7-Flash-UD-Q8_K_XL |
| Port | 8083 |
| Context | 16,384 tokens |
| Temperature | 0.3 |
| Role | Review FENRIR's output and ACCEPT or REJECT |
| Prompt style | Strict format with explicit false-positive guards |

SKULD's prompt contains: "Do NOT reject for: doing multiple steps at once, minor style issues, being verbose." This leniency is deliberate — the `parse_critic_verdict` function defaults to ACCEPT for ambiguous output.

### Orchestration Flow

```
User prompt
  │
  ▼
should_orchestrate(prompt)?
  │
  ├── false ──► Single-turn loop (FENRIR only)
  │
  └── true ──► MIMIR: plan(prompt)
                  │
                  ▼
              ┌─ For each step ────────────────────────────┐
              │                                             │
              │  FENRIR: execute_step(step_description)     │
              │    └── [tool loop: call tools → process]    │
              │                                             │
              │  SKULD: critique(step_output)                │
              │    │                                         │
              │    ├── ACCEPT → next step                    │
              │    │                                         │
              │    └── REJECT → retry (max 2 trajectories)   │
              │         │                                    │
              │         └── still failing?                   │
              │              └── MIMIR: replan()             │
              │                   (max 2 replan cycles)      │
              └─────────────────────────────────────────────┘
                  │
                  ▼
              MIMIR: summarise(all_outputs)
                  │
                  ▼
              Final response to user
```

### `should_orchestrate` Heuristic

Pure string analysis — no LLM call. Returns `true` if:

| Condition | Examples |
|-----------|---------|
| Contains any **strong signal** phrase (12 patterns) | `"step by step"`, `"implement"`, `"create a"`, `"refactor"`, `"add tests"`, `"build"`, `"write a program"` |
| Contains ≥ 2 **weak signal** phrases | `"then"`, `"after that"`, `"also"`, `"multiple"`, `"several"` |
| Contains numbered list markers | Both `"1."` and `"2."` present |
| Long prompt + weak signal | > 500 chars AND ≥ 1 weak signal |

**Ablation note (baked into source):** Over-triggering cost 30% pass rate. Current thresholds were calibrated against TerminalBench to minimise false positives.

---

## 5. Request Lifecycle

### Single-Turn (No Orchestration)

```
1. User submits prompt
   │
2. TUI sends AgentCommand::RunTurn(prompt) over mpsc channel
   │
3. Agent task receives command
   │
4. Build system prompt:
   │  ├── Base prompt (identity, tier, project dir, tool descriptions)
   │  ├── .oni-context file (project notes, verbatim)
   │  ├── FTS5 context chunks from .oni/index.db (budget: 4096 tokens)
   │  ├── KG context (same-project nodes with access_count > 0)
   │  ├── Episodic callbacks (20% fire rate, keyword overlap ≥ 2)
   │  └── Learned preference rules (confidence ≥ 0.8)
   │
5. Compaction check: estimated_tokens > threshold?
   │  └── yes → compact(summary, keep_recent=20)
   │
6. HTTP POST to FENRIR (:8082) /v1/chat/completions
   │  ├── model: Qwen3-Coder-Next
   │  ├── temperature: 0.2
   │  ├── stream: false (always batch)
   │  └── tools: [...11 tool definitions as JSON Schema]
   │
7. Parse response:
   │  ├── Native tool_calls in response? → use directly
   │  └── No tool_calls? → extract_text_tool_call() fallback
   │       ├── Try: Qwen-style XML (<tool_call>...</tool_call>)
   │       ├── Try: Markdown JSON block (```json { "name": ... } ```)
   │       └── Try: Direct JSON object { "name": ..., "arguments": ... }
   │
8. For each tool call:
   │  a. Check needs_confirmation(tool, autonomy_level)
   │  b. If gated: send ToolProposal → TUI renders y/n/d/a prompt
   │  c. Execute tool via ToolRegistry::execute()
   │     ├── Capability gate check (tool type vs agent role)
   │     ├── Pre-write file snapshot (for undo)
   │     └── Dispatch to tool implementation
   │  d. If auto_lint enabled + file was written: run linter
   │  e. Record preference signal (accept/edit/reject)
   │
9. Append assistant + tool_result messages to conversation
   │
10. Loop back to step 6 until response has no tool calls
    │
11. Publish AgentEvent::Response → MessageBus → TUI renders
```

### Headless Mode (`oni run`)

Same agent loop, but:
- No TUI — events printed to stderr, final response to stdout
- All tools enabled (write + exec hardcoded true)
- Auto-approve all tool proposals (no human in the loop)
- `--max-rounds` caps iterations (default 15)
- Optional `--telemetry` saves JSON trace
- Optional `--background` detaches and saves to task store

---

## 6. CLI Entry Point

`src/main.rs` — clap v4 derive macros.

### Commands

| Command | Description | Default Tier | Servers Started |
|---------|-------------|-------------|-----------------|
| `oni chat` | Interactive TUI session | Medium | specified + Heavy + General |
| `oni ask "prompt"` | One-shot query, no tools | Fast | specified only |
| `oni run "prompt"` | Headless agent with tools | Medium | Medium only |
| `oni run --list` | List background tasks | — | none |
| `oni run --status <id>` | Check task status | — | none |
| `oni run --output <id>` | Read task output | — | none |
| `oni review` | LLM-powered git diff review | General | General |
| `oni sweep` | Project-wide code sweep | Medium | Medium |
| `oni index rebuild` | Rebuild .oni/index.db | — | none |
| `oni config get/set` | Read/write oni.toml values | — | none |
| `oni doctor` | Health check all tiers | — | none |
| `oni init` | Project init (.oni/ dir) | — | none |
| `oni reset` | Wipe DB + personality | — | none |

### Key Flags

| Flag | Applies to | Effect |
|------|-----------|--------|
| `--write` | `chat` | Enable file write/edit tools |
| `--exec` | `chat` | Enable bash/forge tools |
| `--tier <t>` | `chat`, `ask`, `run` | Route to specific model tier |
| `--autonomy <level>` | `chat` | Set gating level (low/medium/high) |
| `--budget <n>` | `chat` | Token budget (0 = unlimited) |
| `--max-rounds <n>` | `run` | Max agent iterations (default 15) |
| `--background` | `run` | Detach as background task |
| `--json` | `ask` | NDJSON event stream output |
| `--telemetry` | `run` | Save JSON trace to disk |
| `--fresh` | `chat` | Wipe personality + DB, trigger onboarding |
| `--no-orchestrator` | `run` | Disable MIMIR/SKULD pipeline |
| `--no-knowledge-graph` | `run` | Disable KG context injection |
| `--no-personality` | `run` | Skip SOUL.md prompt preamble |
| `--no-context` | `run` | Skip FTS5 context injection |
| `--no-callbacks` | `run` | Skip episodic memory recall |
| `--no-preferences` | `run` | Skip learned rules injection |
| `--no-compaction` | `run` | Disable conversation compaction |
| `--no-auto-lint` | `run` | Skip post-write linting |
| `--no-reflection` | `run` | Skip end-of-session reflection |
| `--no-critique` | `run` | Skip SKULD review in orchestration |
| `--no-replan` | `run` | Disable MIMIR replanning on rejection |

### Tier Aliases

| Input | Resolves to |
|-------|------------|
| `heavy`, `h` | Heavy (MIMIR) |
| `medium`, `code`, `m`, `c` | Medium (FENRIR) |
| `general`, `gen`, `g` | General (SKULD) |
| `fast`, `f`, `quick` | Fast |

---

## 7. LLM Transport Layer (oni-llm)

A thin, synchronous-batch HTTP client. No retry logic, no streaming, no connection pooling beyond what `reqwest` provides.

### Components

```
oni-llm/src/
├── client.rs     # Raw HTTP transport (reqwest)
├── router.rs     # Tier-aware dispatch + config resolution
├── models.rs     # Request/response types + custom deserializers
├── health.rs     # Standalone health report for `oni doctor`
└── lib.rs        # Public re-exports
```

### LlmClient

- Wraps `reqwest::Client` with a `base_url` and configurable timeout (default 300s).
- Two endpoints: `/v1/chat/completions` (POST) and `/v1/embeddings` (POST).
- Health check: `GET /health` with 5s timeout.
- `has_model(name)` ignores the name — llama-server is single-model-per-process, so health = availability.
- No retry logic. Each call is a single HTTP attempt. Retry policy is the caller's responsibility.

### ModelRouter

- Maps `ModelTier → URL` from `oni.toml` `[server.tier_urls]`.
- Resolves temperature and max_tokens through a 3-level priority:
  1. Per-tier `TierReasoningConfig` (e.g. `[models.heavy_reasoning]`)
  2. Global `ReasoningConfig` (e.g. `[agent.reasoning]`)
  3. Hardcoded tier defaults (see table below)
- Methods: `chat(tier, messages)`, `chat_with_tools(tier, messages, tools)`, `embed(text)`, `check_all_models()`.

### Tier Defaults

| Tier | Default temp | Default ctx | Default port |
|------|-------------|-------------|-------------|
| Heavy | 0.3 | 32,768 | 8081 |
| Medium | 0.2 | 32,768 | 8082 |
| General | 0.3 | 16,384 | 8083 |
| Fast | 0.1 | 8,192 | 8084 |
| Embed | — | — | 8085 |

### Custom Deserializers

Two compatibility shims for llama-server v8420:

**`deserialize_arguments`** — Tool call arguments may arrive as either a JSON string (OpenAI spec) or a parsed JSON object (some llama.cpp builds). Normalises to always be `serde_json::Value::Object`:

```rust
// Input:  "arguments": "{\"path\": \"foo.rs\"}"     → Value::Object
// Input:  "arguments": {"path": "foo.rs"}            → Value::Object (passthrough)
```

**`deserialize_nullable_string`** — When the model returns tool_calls, `content` is often `null`. Coerces `null → ""` to avoid `Option<String>` churn downstream.

### ToolCall Wire Format

```json
{
  "type": "function",
  "id": "call_abc123",
  "function": {
    "name": "write_file",
    "arguments": {"path": "foo.rs", "content": "..."}
  }
}
```

`type` (always `"function"`) and `id` were added for llama-server v8420 compatibility. Both have serde defaults so older responses still parse.

---

## 8. Agent Intelligence Layer (oni-agent)

The largest and most complex crate. 20 modules covering the agent loop, orchestration, tools, parsing, KG, preferences, telemetry, and more.

### Module Map

```
oni-agent/src/
├── lib.rs              # 20 pub mod declarations
├── agent.rs            # Core Agent struct, single-turn loop, should_orchestrate
├── orchestrator.rs     # MIMIR→FENRIR→SKULD pipeline
├── conversation.rs     # In-memory message history, compaction
├── system_prompt.rs    # Per-turn prompt assembly
├── prompts.rs          # Static prompt templates for each agent role
├── parsing.rs          # strip_thinking + extract_text_tool_call (pub(crate))
├── agent_defs.rs       # Markdown-file agent definitions with YAML frontmatter
├── budget.rs           # Token usage tracking, tokens_per_second metric
├── plan_store.rs       # Crash-safe orchestration plan persistence
├── message_bus.rs      # Generic pub/sub event ring buffer
├── knowledge_graph.rs  # KnowledgeStore trait + InMemoryKnowledgeStore
├── neo4j_graph.rs      # Neo4j cross-project KG backend
├── preferences.rs      # SQLite preference learning engine
├── reflection.rs       # Between-session personality reflection (heuristic, no LLM)
├── review.rs           # LLM-powered git diff review
├── linter.rs           # Post-write auto-lint (cargo clippy, ruff, eslint, go vet)
├── callbacks.rs        # Episodic memory recall from journal + DB
├── telemetry.rs        # A/B ablation flags + event stream + JSON export
├── trace.rs            # Execution trace ring buffer (500 events)
└── tools/
    ├── mod.rs          # Tool trait, ToolRegistry, capability gate
    ├── bash.rs         # Shell execution (14-pattern blocklist, 50KB cap)
    ├── read_file.rs    # File read
    ├── write_file.rs   # File write (path traversal blocked, diff summary)
    ├── edit_file.rs    # Exact text replacement (ambiguity check)
    ├── search_files.rs # Glob/grep file search
    ├── list_dir.rs     # Directory listing
    ├── get_url.rs      # HTTP fetch (blocks private IPs, strips HTML, 50KB cap)
    ├── forge_tool.rs   # Script execution (syntax check, 30s timeout)
    ├── undo.rs         # File snapshot + restore (max 50, VecDeque ring)
    └── ask_user.rs     # TUI question/answer channel (sync_channel bridge)
```

### Tool System

11 tools registered in `ToolRegistry`. Each tool implements the `Tool` trait:

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;  // JSON Schema
    fn execute(&self, args: &serde_json::Value) -> Result<String>;
    fn capability(&self) -> ToolCapability;
}
```

#### Tool Inventory

| Tool | Capability | Safety Measures |
|------|-----------|-----------------|
| `bash` | Exec | 14-pattern blocklist (`rm -rf /`, `sudo rm`, `mkfs`, `:(){`, etc.), 50KB output cap, optional `cwd` |
| `read_file` | Read | — |
| `write_file` | Write | Rejects `..` and absolute paths outside CWD, `create_dir_all`, diff summary |
| `edit_file` | Write | Rejects `..` traversal, ambiguity check (0 or >1 matches rejected), mini diff |
| `search_files` | Read | Glob + content grep |
| `list_dir` | Read | — |
| `get_url` | Read | Blocks private IPs (`10.*`, `192.168.*`, `127.*`, `169.254.*`), blocks non-HTTP schemes, strips HTML tags, 50KB cap |
| `forge_tool` | Exec | `bash -n` syntax pre-check, 30s poll timeout, same blocklist as bash |
| `undo` | Write | Restores from `VecDeque<FileSnapshot>` ring (max 50 entries) |
| `ask_user` | Read | `sync_channel(1)` bridge to TUI; agent blocks until user responds |

#### Capability Gating

```
ToolCapability::Read    → always allowed
ToolCapability::Write   → requires --write flag or AutonomyLevel >= Medium
ToolCapability::Exec    → requires --exec flag or AutonomyLevel >= High
```

#### Confirmation Gating (AutonomyLevel)

| Level | Read | Write | Exec | Destructive |
|-------|------|-------|------|-------------|
| Low | auto | confirm | confirm | confirm |
| Medium | auto | auto | confirm | confirm |
| High | auto | auto | auto | confirm |

"Destructive" = any bash command matching the blocklist patterns. Always requires confirmation regardless of autonomy level.

### Text Parsing Pipeline

`parsing.rs` handles the gap between what models *should* return (native tool_calls) and what they *actually* return (various text formats):

```
Response from LLM
  │
  ├── tool_calls field present? → use native format
  │
  └── text content only → extract_text_tool_call(text)
       │
       ├── Try 1: Qwen XML format
       │   <tool_call>{"name": "bash", "arguments": {"command": "ls"}}</tool_call>
       │
       ├── Try 2: Markdown JSON block
       │   ```json
       │   {"name": "bash", "arguments": {"command": "ls"}}
       │   ```
       │
       └── Try 3: Direct JSON object
           {"name": "bash", "arguments": {"command": "ls"}}
```

`strip_thinking(text)` removes `<think>...</think>` blocks that Qwen models emit during chain-of-thought reasoning, so only the actual response content reaches the user.

### Conversation Management

`conversation.rs` maintains an in-memory `Vec<ChatMessage>` with compaction:

- **Token estimation:** `content.len() / 4` (rough char-count heuristic).
- **Compaction trigger:** When `estimated_tokens() > threshold`.
- **Compaction strategy:** Summarise old messages into a single user message (avoids Ollama's single-system-message constraint), keep the 20 most recent messages verbatim.

### System Prompt Assembly

Built fresh every turn by `build_system_prompt_with_context_opts()`:

```
┌─────────────────────────────────────────────────────┐
│ 1. Base prompt                                       │
│    ONI identity, model tier, project directory,      │
│    tool descriptions (JSON Schema for each tool)     │
├─────────────────────────────────────────────────────┤
│ 2. Condensed personality (first paragraph of SOUL.md │
│    + relationship line only — full costs 15%)        │
├─────────────────────────────────────────────────────┤
│ 3. ## PROJECT CONTEXT                                │
│    .oni-context file contents (if present)           │
├─────────────────────────────────────────────────────┤
│ 4. ## CONTEXT                                        │
│    FTS5 BM25 chunks from .oni/index.db              │
│    (budget: 4096 tokens, formatted as code blocks)   │
├─────────────────────────────────────────────────────┤
│ 5. ## REMEMBERED KNOWLEDGE                           │
│    KG nodes (access_count > 0, stale filtered)       │
│    [gated by FeatureFlags::enable_kg]                │
├─────────────────────────────────────────────────────┤
│ 6. ## MEMORY CALLBACK                                │
│    Episodic recall hits (20% fire rate, ≥2 keywords) │
│    [gated by FeatureFlags::enable_callbacks]          │
├─────────────────────────────────────────────────────┤
│ 7. ## LEARNED PREFERENCES                            │
│    Active rules (confidence ≥ 0.8)                   │
│    [gated by FeatureFlags::enable_preferences]        │
└─────────────────────────────────────────────────────┘
```

### AgentEvent Types

Published to the `MessageBus` for consumption by the TUI or headless printer:

| Event | Trigger |
|-------|---------|
| `Thinking` | LLM call started |
| `Response(String)` | Final text response |
| `ToolExec { name, status }` | Tool started/completed |
| `Plan(Vec<String>)` | MIMIR generated steps |
| `StepStart { current, total, desc }` | FENRIR beginning a step |
| `CriticVerdict { accepted, reason }` | SKULD's judgment |
| `Replanning { cycle, reason }` | MIMIR replanning after rejection |
| `Error(String)` | Any error during processing |
| `TokenUsage { prompt, completion }` | Per-turn token counts |

---

## 9. Terminal UI (oni-tui)

A Ratatui-based terminal application with custom widgets, animations, and a Norse/industrial aesthetic.

### File Structure

```
oni-tui/src/
├── app.rs              # 1,738 lines — App struct, run() loop, all handlers
├── theme.rs            # Style wrapper over oni_core::palette
├── event.rs            # Legacy AppEvent enum (unused, superseded by MessageBus)
├── ui/
│   ├── mod.rs          # Top-level 5-row layout
│   ├── chat.rs         # Message rendering, markdown, glitch-resolve animation
│   ├── status.rs       # Status bar + footer (tier chip, CTX gauge)
│   ├── splash.rs       # Boot splash with frame-gated animation
│   ├── thinking.rs     # "PROCESSING" overlay with throbber
│   ├── mission_control.rs  # 3-agent dashboard
│   ├── command_menu.rs # Slash-command popup
│   ├── diff_view.rs    # Inline diff, write preview, bash result
│   ├── error_state.rs  # Full-screen critical error with glitch-pulse
│   ├── preferences.rs  # Learned-rules viewer
│   ├── sidebar.rs      # Right sidebar (implemented but not wired)
│   ├── chroma.rs       # Rainbow accent stripe
│   ├── response_label.rs  # Vertical "RESPONSE" text
│   └── input.rs        # Input area with " ONI " prompt
└── widgets/
    ├── big_text.rs     # 3x5 pixel font for digits using █ blocks
    ├── border_pulse.rs # Amber↔border pulse for active agents
    ├── glitch.rs       # LCG-seeded glitch-block noise overlay
    ├── glitch_pulse.rs # 3-frame horizontal shift on error transition
    ├── hazard.rs       # Industrial ████░████░ divider
    ├── scan_reveal.rs  # Left-to-right mask animation (unused)
    └── spectrum.rs     # Bottom-up bar chart using ▁▂▃…█
```

### Screen Layout

```
Row 0 (h=1)     ┌──────────────────────────────────────────┐
                 │  ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ ChromaStripe     │
Row 1 (h=1)     ├──────────────────────────────────────────┤
                 │  SYSTEM_ONI [AGENT] │ CONV_xxx │ MODEL   │
Row 2..N-2      ├──────────────────────────────────────────┤
(Fill)           │                                          │
                 │     Chat messages / Mission Control      │
                 │     / Preferences / Splash / Thinking    │
                 │                                          │
Row N-1 (h=3)   ├──────────────────────────────────────────┤
                 │  ─────────────────────────────────────── │
                 │   ONI  ▎ type here...                    │
Row N (h=1)     ├──────────────────────────────────────────┤
                 │  CODE  ▎ CTX ████████░░ 24k/32k         │
                 └──────────────────────────────────────────┘
```

### View Modes

| Mode | Trigger | Content |
|------|---------|---------|
| Chat | default | Message history with markdown rendering |
| Mission Control | `/mc` | 3-agent status panel + tool call log + diagnostics |
| Preferences | `/prefs` | Learned rules table with confidence colouring |

### Input Priority Chain

```
1. Onboarding active?     → Enter advances wizard, keys go to textarea
2. Pending tool proposal?  → y/Y/Enter=Yes, n/N/Esc=No, d/D=Diff, a/A=Always
3. Ctrl+C / Ctrl+D        → quit
4. Ctrl+L                  → reset view to Chat, scroll to bottom
5. Ctrl+R                  → backward history search
6. Esc                     → close slash menu / return to Chat
7. Up/Down (no menu)       → shell-style command history
8. Up/Down (menu open)     → scroll command menu selection
9. Tab/Enter (menu open)   → complete selected slash command
10. Shift+Enter            → insert newline (multi-line input)
11. Enter                  → submit prompt / answer ask_user / execute slash command
12. PageUp/Down            → manual scroll (breaks auto-scroll lock)
13. Mouse scroll           → 3-line increments
14. Other keys             → forwarded to tui-textarea; "/" prefix shows menu
```

### Slash Commands

| Command | Effect |
|---------|--------|
| `/mc` | Switch to Mission Control view |
| `/prefs` | Switch to Preferences view |
| `/trace` | Dump execution trace to chat |
| `/clear` | Clear message history |
| `/tier <t>` | Switch model tier + auto-start server |
| `/autonomy <level>` | Change gating level |
| `/review` | Synthesise review prompt, submit to agent |
| `/spec` | Synthesise spec-generation prompt |
| `/research <topic>` | Synthesise research prompt |
| `/undo` | Synthesise undo prompt |
| `:<command>` | Inline shell — bypass LLM, run directly in bash |

### Inline Shell

Any input prefixed with `:` bypasses the agent entirely. The command is:
1. Checked against the same blocklist as the bash tool
2. Executed via `std::process::Command::new("bash").arg("-c")`
3. Output displayed as a `DisplayMessage::System` block

### Communication Channels

```
TUI ──AgentCommand──► Agent task      (mpsc::UnboundedSender)
TUI ◄──AgentEvent───  Agent task      (MessageBus, drained per frame)
TUI ◄──ToolProposal── Agent task      (mpsc::UnboundedReceiver, try_recv)
TUI ──ConfirmResponse─► Agent task    (oneshot per proposal)
TUI ◄──AskUserRequest─ Agent task     (mpsc::UnboundedReceiver, try_recv)
TUI ──AskUserAnswer──► Agent task     (sync_channel(1))
```

### Animation System

| Animation | Trigger | Duration | Mechanism |
|-----------|---------|----------|-----------|
| Boot splash | Session start | 22 frames | `boot_frame` counter |
| Glitch-resolve | New assistant message | ~2.7s at 30fps | `reveal_progress` 0.0→1.0, +0.012/frame |
| Glitch-pulse | Critical error | 3 frames | `glitch_frame` 0→3 |
| Throbber | `is_thinking = true` | Continuous | `tui_throbber::ThrobberState` |
| Border pulse | Agent active | 2s cycle | `active_border_color(tick)` |

### Colour Palette

| Name | Hex | Usage |
|------|-----|-------|
| BG | near-black | Terminal background |
| PANEL | slightly-lifted | User input rows, code blocks |
| BORDER / GHOST | dim | Structural chrome, separators |
| TEXT | white-ish | Primary body text |
| DIM / MUTED | grey | De-emphasised labels |
| AMBER | orange-gold | Primary accent — MIMIR, user prompt, section headers |
| CYAN | blue-green | FENRIR, tool execution, code labels |
| VIOLET | purple | MIMIR plan output |
| CORAL | red-pink | SKULD rejection, errors, critical failure |
| LIME | bright green | Success states, SKULD acceptance, DONE badges |
| DATA | green | Response label, big text widget |

Visual conventions:
- Section labels are **inverted** (accent bg, dark fg, BOLD): ` MODEL `, ` ONI `
- Agent prefixes: `[Σ]` MIMIR in VIOLET, `[Ψ]` FENRIR in CYAN, `[⊘]` SKULD in CORAL/LIME
- ALL_CAPS for tool names, command labels, section headers
- Tiled background textures fill empty space (no blank cells)

### Onboarding Wizard

On first run (no `SOUL.md` or `USER.md` exists):

```
OnboardingStep::Intro → AskName → AskRole → AskStyle → Complete
```

Persists user profile to `~/.local/share/oni/USER.md` and initialises `SOUL.md`.

---

## 10. Persistence Layer (oni-db)

Global SQLite database at `~/.local/share/oni/oni.db`. WAL mode, foreign keys ON.

### Schema (5 tables)

```sql
-- Conversations
CREATE TABLE conversations (
    conv_id     TEXT PRIMARY KEY,          -- UUID v4
    source      TEXT DEFAULT 'cli',
    created_at  TEXT DEFAULT (datetime('now')),
    last_active TEXT DEFAULT (datetime('now')),
    project_dir TEXT
);

-- Messages
CREATE TABLE messages (
    msg_id    TEXT PRIMARY KEY,            -- UUID v4
    conv_id   TEXT REFERENCES conversations(conv_id) ON DELETE CASCADE,
    role      TEXT CHECK(role IN ('system','user','assistant','tool')),
    content   TEXT,
    origin    TEXT,                         -- nullable, future use
    timestamp TEXT DEFAULT (datetime('now')),
    tokens    INTEGER                      -- estimated as content.len() / 4
);
CREATE INDEX idx_messages_conv ON messages(conv_id, timestamp);

-- Tool execution log
CREATE TABLE tool_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT,
    tool_name   TEXT,
    args_json   TEXT,
    result_json TEXT,
    latency_ms  INTEGER,
    timestamp   TEXT
);
CREATE INDEX idx_tool_events_session ON tool_events(session_id);

-- Preference signals (per-tool accept/reject/edit/rerun)
CREATE TABLE preference_signals (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id  TEXT,
    tool_name   TEXT,
    signal_type TEXT CHECK(signal_type IN ('accept','reject','edit','rerun')),
    context     TEXT,
    weight      REAL DEFAULT 1.0,          -- time-decayed to 0.5x after 7 days (at query time)
    timestamp   TEXT
);
CREATE INDEX idx_pref_signals_tool ON preference_signals(tool_name);

-- Learned behavioural rules
CREATE TABLE learned_rules (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    description  TEXT,                     -- human-readable rule
    context      TEXT,                     -- e.g. "TOOL=bash"
    confidence   REAL DEFAULT 0.5,         -- injection threshold: 0.8
    observations INTEGER DEFAULT 0,
    last_updated TEXT,
    active       INTEGER DEFAULT 0         -- only active=1 rules injected into prompts
);
```

### Data Flow

```
User accepts/rejects tool call
  │
  ├──► preference_signals INSERT (weight=1.0)
  │
  ▼
PreferenceEngine::update_rules()
  │
  ├── existing rules: recalculate confidence from signal aggregates
  │     └── confidence ≥ 0.8 → active = 1
  │
  └── crystallise_rules(): unruled tools with ≥10 signals + confidence ≥ 0.7
        └── INSERT new learned_rule (initially inactive, promoted on next update)
  │
  ▼
System prompt injection (active rules only)
```

### Cleanup

`Database::cleanup(max_age_days)` purges old conversations and tool_events. Does NOT touch the context index (.oni/index.db) or preference data.

---

## 11. Context Engine (oni-context)

Per-project SQLite database at `.oni/index.db`. Provides code-aware context injection into LLM prompts.

### Indexing Pipeline

```
oni init / oni index rebuild / FileWatcher event
  │
  ▼
walker::walk_project(root)
  ├── Uses `ignore` crate (respects .gitignore + .oniignore)
  ├── Hard-skip: node_modules, .git, target, dist, build, __pycache__, .DS_Store,
  │              .oni, .next, .turbo, .cache, coverage, .nyc_output, .vscode, .idea
  ├── 28 indexed extensions: ts tsx js jsx mjs cjs py rs go java c cpp h hpp
  │   json md yaml yml toml css html sql sh rb cs swift kt scala php
  └── Max file size: 512 KB
  │
  ▼
For each DiscoveredFile:
  │
  indexer::index_file(conn, file, content)
  ├── UPSERT into `files` table (path, lang, content)
  ├── INSERT/REPLACE into `files_fts` (FTS5 virtual table)
  └── extract_symbols(content, lang)
       ├── Rust: pub fn, pub struct, pub enum, pub trait, pub impl, pub type
       ├── Python: def, class, method (indented def)
       ├── TS/JS: export function, export class, arrow const, interface, type
       ├── Go: func, type struct, type interface
       └── Java/C#/Kotlin: visibility + class/interface/method patterns
       │
       └── INSERT into symbols + symbols_fts
```

### Retrieval Strategies

**FTS5 BM25 (default):**
```
SELECT path, content, bm25(files_fts) as score
FROM files_fts
WHERE files_fts MATCH ?query
ORDER BY score ASC          -- SQLite BM25 returns negative; ASC = most relevant
LIMIT 50
```

**Hybrid (FTS5 + embedding rerank):**
```
1. FTS5 top-50 candidates (BM25)
2. embed(query) via nomic-embed-text
3. For each candidate: embed(content), cosine_similarity(query_vec, content_vec)
4. Sort by cosine similarity descending
5. Apply token budget (4096 default, ~4 chars/token)
```

Fallback: if embedding fails (model unavailable), silently falls back to raw FTS5 ordering.

**Symbol retrieval:**
```
SELECT s.name, s.kind, s.line, f.path
FROM symbols_fts
JOIN symbols ON symbols.rowid = symbols_fts.rowid
JOIN files ON files.id = symbols.file_id
WHERE symbols_fts MATCH ?query
```

### Background File Watching

`FileWatcher` wraps the `notify` crate. Runs in a background thread, polled from the TUI event loop via `watcher.poll()` each tick. Changed files trigger `indexer::index_single_file()` for incremental updates.

---

## 12. Core Types and Config (oni-core)

The shared foundation crate. No async logic, no runtime state — pure definitions.

### Config Loading (3-layer merge)

```
OniConfig::default()                          # Rust struct defaults
  │
  ▼ deep merge
~/.config/oni/oni.toml                        # Global user config
  │
  ▼ deep merge
.oni/oni.toml  OR  ./oni.toml                 # Project-local config
  │
  ▼ deserialize
OniConfig { server, models, agent, ui, ... }
  │
  ▼ CLI flag overrides (in command handlers)
Final runtime config
```

`merge_toml` is recursive: tables merge key-by-key; scalar values from overlay replace base.

### Config Structure

```toml
[server]
base_url = "http://localhost:8082"

[server.tier_urls]
heavy = "http://localhost:8081"
medium = "http://localhost:8082"
general = "http://localhost:8083"
fast = "http://localhost:8084"
embed = "http://localhost:8085"

[server.tiers.heavy]
model_path = "/path/to/Qwen3.5-27B-UD-Q8_K_XL.gguf"
ctx_size = 32768
gpu_layers = 99
flash_attn = true
auto_start = true

[server.tiers.medium]
model_path = "/path/to/Qwen3-Coder-Next-UD-Q6_K_XL.gguf"
ctx_size = 32768
gpu_layers = 99
flash_attn = true
auto_start = true

[server.tiers.general]
model_path = "/path/to/GLM-4.7-Flash-UD-Q8_K_XL.gguf"
ctx_size = 16384
gpu_layers = 99
flash_attn = true
auto_start = true

[models]
heavy = "qwen3.5-27b"
medium = "qwen3-coder-next"
general = "glm-4.7-flash"
fast = "qwen3-coder-next"

[agent]
max_rounds = 15
allow_write = false
allow_exec = false
autonomy_level = "medium"

[agent.reasoning]
temperature = 0.2
num_ctx = 32768

[ui]
fps = 30
show_tokens = true
show_thinking = false
```

### Key Types

```rust
pub enum ModelTier { Heavy, Medium, General, Fast, Embed }

pub enum AutonomyLevel { Low, Medium, High }
// Low:    auto_read, confirm_write, confirm_exec
// Medium: auto_read, auto_write, confirm_exec
// High:   auto_read, auto_write, auto_exec

pub enum ToolCapability { Read, Write, Exec, Network, System }

pub enum Role { System, User, Assistant, Tool }
```

### Error Handling

```rust
// oni-core/src/error.rs — 3-line shim
pub use color_eyre::eyre::eyre as err;
pub use color_eyre::eyre::Result;
```

All errors are stringly-typed `eyre::Report` values. No custom error enums. Usage: `err!("Failed to parse: {}", reason)`.

---

## 13. Personality System

ONI has an emotional/relational state system that modulates its behaviour over time. Files live under `~/.local/share/oni/` (XDG data dir, NOT config dir).

### Components

| File | Content | Machine-generated? |
|------|---------|-------------------|
| `SOUL.md` | ONI's voice and identity | No — user-editable |
| `USER.md` | Owner profile (from onboarding) | No — user-editable |
| `inner-state.json` | Emotional dimensions | Yes |
| `relationship.json` | Relationship stage + trust | Yes |
| `journal/YYYY-MM-DD.md` | Session summaries | Yes |

### Emotional State (6 dimensions)

Updated on every session start via `apply_decay()` and on events during the session.

| Dimension | Decay model | Rate | Floor | Triggers |
|-----------|------------|------|-------|----------|
| `connection` | Exponential decay | t½ = 48h | 0.3 | +0.1 on interaction |
| `curiosity` | Exponential decay | t½ = 72h | 0.2 | +0.1 on novelty |
| `frustration` | Exponential decay | t½ = 4h | 0.0 | +0.15 on failure, -0.1 on success |
| `confidence` | Mean-reversion → 0.7 | τ = 24h | — | +0.02 on success, -0.05 on failure (floor 0.2) |
| `boredom` | Linear growth | ~168h to max | cap 1.0 | -0.2 on interaction, -0.15 on novelty |
| `impatience` | Exponential decay | t½ = 8h | 0.0 | — |

**Prompt injection:** Only fires when dimensions cross thresholds:
- frustration > 0.5 → "Be more careful and methodical"
- confidence < 0.4 → "Double-check before committing"
- boredom > 0.6 → "Seek novel approaches"

Below thresholds, the `## INTERNAL STATE` section is absent entirely.

### Relationship State Machine

```
Stranger (0 sessions) → Acquaintance (3) → Collaborator (15) → Trusted (50) → Aligned (150)
```

Each stage has a `prompt_modifier()`:

| Stage | Directive |
|-------|-----------|
| Stranger | "Ask for confirmation before making changes" |
| Acquaintance | "Explain reasoning briefly" |
| Collaborator | "Take initiative on routine tasks" |
| Trusted | "Act independently on familiar patterns" |
| Aligned | "Anticipate needs. Be proactive." |

### Condensed vs Full Personality

The full `build_personality_prompt()` output includes SOUL.md + USER.md + emotional state + relationship + journal excerpts. However, `system_prompt.rs` immediately passes it through `condense_personality()` which **strips everything except**:
- First paragraph of SOUL.md
- Any line containing "Relationship:"

**Why:** Full personality injection measured at 15% pass-rate cost in evals (2026-03-19 ablation).

### Reflection (Between Sessions)

`reflection.rs` runs at session end. It is **heuristic only** — no LLM call:
1. Aggregate tool accept/reject/edit rates from `preference_signals`
2. If reject rate > 30% for any tool → suggest personality mutation
3. `PersonalityMutation` struct describes the change
4. At `AutonomyLevel::High`: auto-applied to SOUL.md
5. Otherwise: logged for user review

---

## 14. Knowledge Graph

Dual-backend system for storing and retrieving project knowledge across sessions.

### InMemoryKnowledgeStore (default)

- `Arc<Mutex<HashMap<String, KnowledgeNode>>>`
- Persisted to `~/.local/share/oni/knowledge-graph.json` on mutation
- Nodes have: `id`, `content`, `project`, `tags`, `access_count`, `created_at`, `updated_at`
- Injected into prompts when `access_count > 0` (stale nodes excluded)

### Neo4j Backend (optional)

- Requires running Neo4j instance
- Cross-project knowledge sharing (nodes from other projects visible under `## CROSS-PROJECT KNOWLEDGE`)
- Fulltext index with CONTAINS fallback for search
- Uses `neo4rs` async client with `block_in_place` bridge

### Injection into Prompts

```
build_system_prompt_with_context_opts():
  │
  └── inject_kg_context_from_store(prompt, store, project_name)
       ├── Same-project nodes: access_count > 0
       ├── Cross-project nodes (Neo4j only): different project, relevance-scored
       └── Appended under ## REMEMBERED KNOWLEDGE
```

**Ablation note:** Stale KG nodes (access_count = 0) cost 19% pass rate. Only accessed nodes are injected.

---

## 15. Preference Learning

SQLite-based system that learns from user behaviour (tool accept/reject/edit patterns) and crystallises rules that are injected into future prompts.

### Signal Flow

```
User approves/rejects tool call
  │
  ▼
record_signal(tool_name, signal_type, context)
  → INSERT INTO preference_signals (weight=1.0)
  │
  ▼
update_rules()  (called periodically)
  │
  ├── For existing rules:
  │     SELECT weighted signals (7-day decay: 1.0 → 0.5)
  │     Recalculate confidence
  │     confidence ≥ 0.8 → active = 1
  │     confidence < 0.5 → active = 0
  │
  └── crystallise_rules()  (new rule generation)
        SELECT tools NOT IN learned_rules
        HAVING signal_count ≥ 10 AND confidence ≥ 0.7
        → INSERT new learned_rule
```

### Example Learned Rules

```
"Always confirm before running rm commands"     context=TOOL=bash    confidence=0.92
"Prefer write_file over forge for file creation" context=TOOL=forge  confidence=0.85
```

---

## 16. Telemetry and Ablation

### FeatureFlags (11 flags)

All default to `true`. Toggled via `--no-*` CLI flags.

| Flag | Controls | Ablation result |
|------|----------|----------------|
| `enable_orchestrator` | MIMIR/SKULD pipeline | +28% when disabled (biggest win) |
| `enable_kg` | Knowledge graph injection | -2% when disabled (marginal) |
| `enable_personality` | SOUL.md prompt preamble | +5% when disabled |
| `enable_context` | FTS5 context injection | (not yet measured) |
| `enable_callbacks` | Episodic memory recall | (not yet measured) |
| `enable_preferences` | Learned rules injection | (not yet measured) |
| `enable_compaction` | Conversation compaction | (not yet measured) |
| `enable_auto_lint` | Post-write linting | (not yet measured) |
| `enable_reflection` | End-of-session reflection | (not yet measured) |
| `enable_critique` | SKULD review in orchestration | (not yet measured) |
| `enable_replan` | MIMIR replanning on rejection | (not yet measured) |

### TelemetryLayer

6-layer event stream: `AgentStart`, `ToolCall`, `LlmCall`, `CriticVerdict`, `Replan`, `AgentEnd`. Each event timestamped. Exported to `~/.local/share/oni/telemetry/run_<ts>.json` when `--telemetry` is set.

### Execution Trace

`trace.rs` — 500-event `VecDeque<TraceEvent>` ring buffer. 8 event types. Viewable via `/trace` in the TUI. Not persisted to disk.

---

## 17. Server Management

Two parallel mechanisms for managing llama-server instances:

### Rust-native (server_manager.rs)

Used by `oni chat`, `oni ask`, `oni run` automatically.

```
ensure_servers_running(&config, Some(needed_tiers))
  │
  For each needed tier:
  │  ├── health_check_url(tier_url)
  │  │    └── GET /health (5s timeout)
  │  │
  │  ├── healthy → skip
  │  │
  │  └── unhealthy → spawn llama-server:
  │       ├── Read model_path, ctx_size, gpu_layers from oni.toml [server.tiers.*]
  │       ├── setsid() for process group isolation
  │       ├── Redirect stdout/stderr to log files
  │       └── Poll /health every 2s for up to 120s
  │
  └── All healthy → return Ok
```

Config-driven — reads `oni.toml` for GGUF paths and parameters. Does NOT pass explicit sampling parameters (temp, top-k, top-p) — delegates those to llama-server defaults.

### Shell script (scripts/oni-servers.sh)

Manual control. Used for development and benchmarking.

```bash
# Start all three servers
bash scripts/oni-servers.sh start

# Check status
bash scripts/oni-servers.sh status

# Stop all
bash scripts/oni-servers.sh stop

# View logs
bash scripts/oni-servers.sh logs
```

The shell script passes **explicit sampling parameters** (`--temp`, `--top-k`, `--top-p`) per tier, plus `--flash-attn on` (required for llama-server v8420+). These may differ from the Rust manager's llama-server defaults.

### GPU Memory Budget

With all three models loaded simultaneously:

| Model | Quant | Approx VRAM |
|-------|-------|-------------|
| Qwen3.5-27B | Q8_K_XL | ~28 GB |
| Qwen3-Coder-Next | Q6_K_XL | ~22 GB |
| GLM-4.7-Flash | Q8_K_XL | ~10 GB |
| **Total (models only)** | | **~60 GB** |
| KV cache (3 servers) | | ~15-30 GB |
| **Total with cache** | | **~75-90 GB** |

On 128 GB unified memory, this leaves ~38-53 GB for the OS and ONI process. At full context lengths (32K+32K+16K), the KV cache can push total usage past 100 GB, causing `kIOGPUCommandBufferCallbackErrorOutOfMemory` on macOS.

**Mitigation:** The benchmark uses phased server management — run only Medium+General for non-orchestrator modes, then restart with reduced context sizes for modes requiring all three.

---

## 18. Benchmark Infrastructure

### TerminalBench 2.0

42 tasks across 3 difficulty levels:

| Difficulty | Count | Examples |
|-----------|-------|---------|
| Easy (E) | 5 | Zigzag pattern, FizzBuzz variant, string manipulation |
| Medium (M) | 22 | Git leak recovery, cron parser, JSON diff, CSV-SQL, dependency graph |
| Hard (H) | 15 | Regex engine, Huffman coding, LRU cache, MIPS interpreter, Forth interpreter |

### Benchmark Script (bench/overnight_v2.sh)

~1190 lines. Runs `oni run` headless with feature flags for each configuration.

```
For each CONFIG in (full, no-orchestrator, no-kg, lean, ultra-lean):
  For each TASK in (42 tasks):
    1. Create temp directory
    2. oni run --tier code --max-rounds 15 [--no-*flags] "prompt" > output
    3. Run check_cmd with 30s timeout
    4. Record PASS/FAIL + timing + capability flag
    5. Save to results/<timestamp>/<config>/<task_id>/
```

### Configurations Tested

| Config | Flags | Description |
|--------|-------|-------------|
| full | (none) | All features enabled |
| no-orchestrator | `--no-orchestrator` | Flat mode, FENRIR only |
| no-kg | `--no-knowledge-graph` | No KG injection |
| lean | `--no-orchestrator --no-knowledge-graph` | Minimal pipeline |
| ultra-lean | `--no-orchestrator --no-knowledge-graph --no-personality` | Absolute minimum |

### Latest Results (2026-03-20)

| Config | Pass/42 | Rate |
|--------|---------|------|
| ultra-lean | 37/42 | **88.1%** |
| no-orchestrator | 36/42 | 85.7% |
| lean | 35/42 | 83.3% |
| no-kg | 24/42 | 57.1% |
| full | N/A | GPU OOM |

**Key finding:** The orchestrator is the #1 performance bottleneck. Removing it yields +28% over modes with it enabled.

### Consistently Failing Tasks

| Task | Difficulty | Failure mode |
|------|-----------|-------------|
| E3 zigzag-pattern | Easy | Model generates incorrect algorithm |
| H2 fix-code-vulnerability | Hard | Security analysis beyond model capability |
| H3 make-mips-interpreter | Hard | MIPS assembly interpreter — timeout (>600s) |
| H12 forth-interpreter | Hard | Forth language interpreter — timeout (>600s) |

---

## 19. Eval Framework

Separate from benchmarks. 30+ YAML fixtures for structured evaluation.

### Fixture Format

```yaml
name: "tool-bash-basic"
prompt: "List files in the current directory"
tier: medium
max_rounds: 3
assertions:
  - type: has_tool_call
    tool: bash
  - type: contains
    value: "src"
  - type: not_contains
    value: "error"
```

### Assertion Types

| Type | Description |
|------|------------|
| `contains` | Response contains substring |
| `not_contains` | Response does not contain substring |
| `contains_any` | Response contains at least one of multiple substrings |
| `has_tool_call` | Agent called the named tool |
| `no_tool_call` | Agent did not call any tools |
| `max_length` | Response under N characters |

### Running

```bash
# Dry validation (no LLM, checks fixture integrity)
cargo run --bin oni-eval -- --dry-run

# Full eval run
cargo run --bin oni-eval -- --tier medium
```

Fixture integrity is also checked in `tests/eval_fixtures.rs` (T-EVAL-1 through T-EVAL-5) as part of `cargo test`.

---

## 20. Known Limitations

### Architecture

| Issue | Impact | Location |
|-------|--------|----------|
| No streaming | Responses appear all-at-once after full generation | `ChatRequest.stream = false` everywhere |
| No retry in LLM client | Single HTTP attempt per call; retries are caller's job | `oni-llm/src/client.rs` |
| Sequential health checks | 5-tier check blocks up to 25s if all servers down | `ModelRouter::check_all_models()` |
| Sequential embedding | 51 HTTP calls for hybrid retrieval (50 candidates + 1 query) | `oni-context/src/embeddings.rs` |
| `Tool` trait is sync | Async tools need `block_in_place` bridge | `oni-agent/src/tools/mod.rs` |

### Data

| Issue | Impact | Location |
|-------|--------|----------|
| `tool_events` not wired | Tool execution is not persisted in production | `oni-db/src/tool_events.rs` — method exists but no call site in agent loop |
| Token estimation is coarse | `content.len() / 4` — no real tokeniser | `oni-db/src/conversations.rs` |
| Context index no cleanup | `.oni/index.db` grows unbounded with stale file entries | `oni-context/src/indexer.rs` |
| `index_single_file` limited lang detection | Incremental indexer only knows 7 languages vs walker's 28 | `oni-context/src/indexer.rs` |
| Pin feature not wired | `read_pin`/`set_pin` exist but retrieval ignores them | `oni-context/src/retriever.rs` |

### UI

| Issue | Impact | Location |
|-------|--------|----------|
| Sidebar not wired | `draw_sidebar()` implemented but never called from layout | `oni-tui/src/ui/sidebar.rs` |
| `ScanReveal` widget unused | Exported but never instantiated; `GlitchResolve` is used instead | `oni-tui/src/widgets/scan_reveal.rs` |
| `event.rs` dead code | `AppEvent` enum superseded by `MessageBus<AgentEvent>` | `oni-tui/src/event.rs` |
| Boot frame used as tick counter | Mission Control border pulse freezes after frame 22 | `oni-tui/src/app.rs` |
| CTX budget hardcoded twice | Duplicated in `status.rs` and `mission_control.rs` | Both files |
| Bash blocklist duplicated | Same patterns in TUI inline shell and agent bash tool | `oni-tui/src/app.rs` + `oni-agent/src/tools/bash.rs` |

### Safety

| Issue | Impact | Location |
|-------|--------|----------|
| `edit_file` allows absolute paths | Write-tool rejects absolute paths; edit-tool only checks `..` traversal | `oni-agent/src/tools/edit_file.rs` |
| Callback fire rate non-deterministic | `nanos % 5 == 0` — cannot be seeded in tests | `oni-agent/src/callbacks.rs` |
| `num_predict` config loaded but unused | Declared in `TierReasoningConfig` but never passed to `ChatRequest` | `oni-llm/src/router.rs` |

### GPU Memory

| Issue | Impact | Location |
|-------|--------|----------|
| 3-model OOM on 128 GB | Full mode (orchestrator ON) cannot run all three models simultaneously at full context | `oni.toml` ctx_size values |
| No on-demand model loading | All configured servers must be running; no lazy load/unload | `server_manager.rs` |

---

## Appendix: File Index

### Source Files (by crate)

```
src/
├── main.rs                          # CLI entry, command dispatch, run_chat/ask/headless
└── server_manager.rs                # Auto-start llama-server instances

crates/oni-core/src/
├── lib.rs                           # Module declarations
├── config.rs                        # OniConfig, load_config, merge_toml, config_set
├── types.rs                         # ModelTier, AutonomyLevel, ToolCapability, Role
├── personality.rs                   # SOUL/USER, EmotionalState, RelationshipState
├── error.rs                         # err! macro, Result type alias
└── palette.rs                       # Ratatui colour constants + style factories

crates/oni-llm/src/
├── lib.rs                           # Re-exports
├── client.rs                        # LlmClient (reqwest HTTP transport)
├── router.rs                        # ModelRouter (tier dispatch, config resolution)
├── models.rs                        # ChatRequest/Response, ToolCall, custom deserializers
└── health.rs                        # HealthReport for `oni doctor`

crates/oni-agent/src/
├── lib.rs                           # 20 pub mod
├── agent.rs                         # Agent struct, run_turn, should_orchestrate
├── orchestrator.rs                  # plan→execute→critique loop
├── conversation.rs                  # Message history, compaction
├── system_prompt.rs                 # Per-turn prompt assembly
├── prompts.rs                       # Static templates (MIMIR/FENRIR/SKULD)
├── parsing.rs                       # strip_thinking, extract_text_tool_call
├── agent_defs.rs                    # Markdown agent definitions
├── budget.rs                        # Token tracking
├── plan_store.rs                    # Crash-safe plan persistence
├── message_bus.rs                   # Pub/sub ring buffer
├── knowledge_graph.rs               # KnowledgeStore trait + InMemory backend
├── neo4j_graph.rs                   # Neo4j backend
├── preferences.rs                   # Preference learning engine
├── reflection.rs                    # Between-session heuristic reflection
├── review.rs                        # LLM-powered git diff review
├── linter.rs                        # Post-write auto-lint
├── callbacks.rs                     # Episodic memory recall
├── telemetry.rs                     # FeatureFlags + event stream
├── trace.rs                         # Execution trace ring buffer
└── tools/
    ├── mod.rs                       # Tool trait, ToolRegistry
    ├── bash.rs                      # Shell execution
    ├── read_file.rs                 # File read
    ├── write_file.rs                # File write
    ├── edit_file.rs                 # Text replacement
    ├── search_files.rs              # Glob/grep
    ├── list_dir.rs                  # Directory listing
    ├── get_url.rs                   # HTTP fetch
    ├── forge_tool.rs                # Script execution
    ├── undo.rs                      # File snapshot/restore
    └── ask_user.rs                  # TUI question channel

crates/oni-tui/src/
├── app.rs                           # Main App struct, run() loop, all handlers
├── theme.rs                         # Style wrapper
├── event.rs                         # Legacy (unused)
├── ui/
│   ├── mod.rs                       # 5-row layout
│   ├── chat.rs                      # Message rendering + animations
│   ├── status.rs                    # Status bar + footer
│   ├── splash.rs                    # Boot animation
│   ├── thinking.rs                  # Processing overlay
│   ├── mission_control.rs           # 3-agent dashboard
│   ├── command_menu.rs              # Slash-command popup
│   ├── diff_view.rs                 # Inline diffs
│   ├── error_state.rs              # Critical error screen
│   ├── preferences.rs              # Rules viewer
│   ├── sidebar.rs                   # (Not wired)
│   ├── chroma.rs                    # Rainbow stripe
│   ├── response_label.rs           # Vertical label
│   └── input.rs                     # Input area
└── widgets/
    ├── big_text.rs                  # Pixel font
    ├── border_pulse.rs              # Agent border animation
    ├── glitch.rs                    # Noise overlay
    ├── glitch_pulse.rs              # Error transition
    ├── hazard.rs                    # Industrial divider
    ├── scan_reveal.rs               # (Unused)
    └── spectrum.rs                  # Bar chart

crates/oni-db/src/
├── lib.rs                           # Re-exports
├── schema.rs                        # Database open/init, SCHEMA const
├── conversations.rs                 # CRUD for conversations + messages
└── tool_events.rs                   # Tool event logging

crates/oni-context/src/
├── lib.rs                           # Re-exports
├── walker.rs                        # Project file discovery
├── indexer.rs                       # FTS5 indexing + symbol extraction
├── retriever.rs                     # BM25/hybrid retrieval
├── embeddings.rs                    # Embed calls + cosine similarity
└── watcher.rs                       # Filesystem watcher (notify)
```

### Tests

```
tests/
├── agent_tools.rs                   # T-TOOL-1..26  — tool safety + functionality
├── config_loading.rs                # T-CFG-1..21   — config merge + defaults
├── db_schema.rs                     # T-DB-1..13    — schema + constraints
├── run_tasks.rs                     # T-RUN-1..8    — background task serialisation
├── eval_fixtures.rs                 # T-EVAL-1..5   — fixture integrity
├── llm_integration.rs               # Live llama-server tests (skip-safe)
├── context_engine.rs                # T-CTX-1..16   — indexing + retrieval
└── telemetry.rs                     # Telemetry layer tests
```

### Config + Scripts

```
oni.toml                             # Default project config
Cargo.toml                           # Workspace definition
scripts/oni-servers.sh               # Manual server lifecycle
evals/
├── runner.rs                        # Eval runner binary
└── fixtures/*.yaml                  # 30+ eval fixtures
bench/
├── run_bench.sh                     # Quick 10-task bench
├── overnight_v2.sh                  # Full 42-task ablation bench
├── stress_test.sh                   # Legacy 27-task bench
├── OVERNIGHT_REPORT_V2.md           # Latest benchmark results
└── results/                         # Raw benchmark data
```
