# ONI — Onboard Native Intelligence

A local-first AI terminal assistant powered by llama.cpp. No cloud, no API keys, no telemetry — just your hardware and your models.

## What is ONI?

ONI is a Rust-based AI coding assistant that runs entirely on your machine using llama.cpp as the inference backend. It features a 3-agent orchestration system, a full Ratatui terminal UI, persistent conversations, a knowledge graph, and a tool system that can read, write, search, and execute — all driven by local GGUF models through an OpenAI-compatible API.

## Features

- **3-agent orchestration** — MIMIR (planner), FENRIR (executor), and SKULD (critic) collaborate on multi-step tasks with automatic replanning
- **11+ built-in tools** — bash execution, file read/write/edit, search, code review, undo, forge (dynamic tool generation), and more
- **Ratatui TUI** — full terminal interface with streaming responses, inline shell, token stats, and theming
- **Conversation persistence** — SQLite-backed conversation history with resume support
- **Knowledge graph** — in-memory and Neo4j backends for cross-project context
- **Context retrieval** — hybrid FTS5 + embedding reranking over indexed project files
- **Preference learning** — learns your patterns over time and adapts tool behaviour
- **Personality system** — emotional state tracking, decay, and relationship modelling
- **Feature flags** — disable any subsystem (orchestrator, knowledge graph, reflection, etc.) for A/B testing
- **Deep telemetry** — per-run JSON reports for benchmarking and ablation studies
- **Background tasks** — detach long-running tasks, check status and logs later
- **Multi-tier model routing** — 5 server tiers (heavy, medium, general, fast, embed) with per-tier config
- **Code review** — `oni review` analyses staged git diffs and gives severity-rated feedback
- **Autonomous sweeps** — `oni sweep` plans and executes codebase-wide operations with dry-run support

## Architecture

ONI uses a 3-agent system inspired by Norse mythology:

| Agent | Symbol | Role | Default Tier |
|-------|--------|------|--------------|
| **MIMIR** | Σ | Decomposes complex tasks into step-by-step plans | Heavy |
| **FENRIR** | Ψ | Executes individual steps with full tool access | Medium |
| **SKULD** | ⊘ | Reviews outputs, can reject and trigger replanning | General |

For simple queries, FENRIR handles everything directly. For multi-step tasks, MIMIR generates a plan, FENRIR executes each step, and SKULD verifies the results — with automatic replanning if SKULD rejects an output.

All agents communicate through a generic pub/sub **event bus** (`MessageBus<AgentEvent>`), which the TUI and headless runner both consume for real-time status updates.

The inference backend is **llama-server** (from llama.cpp), exposing an OpenAI-compatible `/v1/chat/completions` endpoint. Each model tier runs on its own port, allowing different models to serve different roles simultaneously.

## Prerequisites

- **Rust toolchain** — 1.75+ (2021 edition). Install via [rustup](https://rustup.rs/).
- **llama.cpp** — specifically `llama-server`. Build from source or grab a release from [ggml-org/llama.cpp](https://github.com/ggml-org/llama.cpp).
- **GGUF models** — at least one compatible model. ONI defaults to:
  - Heavy: `Qwen3.5-27B-UD-Q8_K_XL`
  - Medium: `Qwen3-Coder-Next-UD-Q6_K_XL`
  - General: `GLM-4.7-Flash-UD-Q8_K_XL`
- **SQLite** — bundled via `rusqlite`, no system install needed.
- **Neo4j** (optional) — for cross-project knowledge graphs. Disable with `[neo4j] enabled = false` in config.

## Installation

```bash
git clone https://github.com/SparkleButt747/ONI.git
cd ONI
cargo build --release
```

The binary lands at `target/release/oni`. Optionally install it to your PATH:

```bash
cargo install --path .
```

## Configuration

ONI loads config from two locations (project overrides global):

- **Global:** `~/.config/oni/oni.toml`
- **Project:** `./oni.toml` (in the working directory)

### Minimal oni.toml

```toml
[server]
base_url = "http://localhost:8082"
timeout_secs = 300
auto_start = true
models_dir = "~/.cache/llama.cpp/models"

[server.tier_urls]
heavy = "http://localhost:8081"
medium = "http://localhost:8082"
general = "http://localhost:8083"
fast = "http://localhost:8084"
embed = "http://localhost:8085"

[models]
heavy = "Qwen3.5-27B-UD-Q8_K_XL"
medium = "Qwen3-Coder-Next-UD-Q6_K_XL"
general = "GLM-4.7-Flash-UD-Q8_K_XL"
fast = "qwen3.5:9b"
embed = "nomic-embed-text"
default_tier = "Medium"

[agent]
max_tool_rounds = 10
allow_write = false
allow_exec = false

[ui]
fps = 30
show_thinking = false
show_token_stats = true
```

Each tier in `[server.tiers.<name>]` can specify `gguf`, `ctx_size`, `cache_type_k/v`, `flash_attn`, `threads`, `gpu_layers`, and `extra_args`. See the included `oni.toml` for a full example.

### Server management

ONI can auto-start llama-server instances, or you can manage them manually:

```bash
bash scripts/oni-servers.sh start    # start all configured tiers
bash scripts/oni-servers.sh stop     # stop all
bash scripts/oni-servers.sh status   # check which tiers are running
```

## Usage

### Interactive chat (TUI)

```bash
oni chat --write --exec              # full permissions
oni chat --tier heavy                # use heavy model
oni chat --autonomy high             # high autonomy level
oni chat --fresh                     # wipe state, start onboarding
```

### One-shot question

```bash
oni ask "What does this function do?"
oni ask --tier heavy "Explain the borrow checker"
echo "Summarise this" | oni ask       # pipe from stdin
oni ask --json "query"                # NDJSON event stream output
```

### Headless run (no TUI)

```bash
oni run "Refactor the error handling in src/main.rs"
oni run --tier heavy --max-rounds 20 "Fix all clippy warnings"
oni run --telemetry "Add tests for the parser"    # save telemetry report
```

#### Feature flags for A/B testing

```bash
oni run --no-orchestrator "task"       # flat mode, no MIMIR/SKULD
oni run --no-knowledge-graph "task"    # disable KG context
oni run --no-reflection "task"         # disable reflection engine
oni run --no-personality "task"        # disable SOUL.md personality
```

#### Background tasks

```bash
oni run --background "Long running task"
oni run --list                         # list all background tasks
oni run --status task_123456           # check a specific task
oni run --logs task_123456             # view task output
oni run --kill task_123456             # terminate a task
```

### Project indexing

```bash
oni init                               # index current project for context retrieval
oni index stats                        # show index statistics
oni index rebuild                      # rebuild from scratch
oni pin src/core                       # pin context retrieval to a subtree
oni pin --reset                        # clear pin
```

### Code review

```bash
git add -p                             # stage changes
oni review                             # review staged diff
oni review --tier heavy                # use heavy model for deeper review
```

### Autonomous sweep

```bash
oni sweep "Remove all dead code"                        # dry-run by default
oni sweep --write "Add error handling to all pub fns"   # actually write changes
oni sweep --glob "src/**/*.rs" "Fix naming conventions" # filter by glob
```

### Configuration and preferences

```bash
oni config show                        # print resolved config
oni config set agent.max_tool_rounds 20
oni prefs show                         # view learned preferences
oni prefs reset                        # wipe learned preferences
oni prefs export prefs.jsonl           # export to file
oni prefs import prefs.jsonl           # import from file
oni prefs forget bash                  # forget prefs for a specific tool
```

### Health check

```bash
oni doctor                             # check server, models, data dir, system info
```

## Development

### Build and test

```bash
cargo build                            # debug build
cargo build --release                  # release build
cargo test --workspace                 # run all tests
cargo clippy --workspace               # lint
```

### Project layout

```
ONI/
├── src/main.rs              # CLI entry point (clap)
├── oni.toml                 # Default config
├── scripts/oni-servers.sh   # llama-server launcher
├── crates/
│   ├── oni-core/            # Config, types, personality, error handling
│   ├── oni-agent/           # Agent loop, orchestrator, tools, conversation
│   ├── oni-llm/             # LLM HTTP client, model router, health checks
│   ├── oni-tui/             # Ratatui terminal UI, widgets, theming
│   ├── oni-db/              # SQLite conversation/tool persistence
│   └── oni-context/         # File indexing, embeddings, code walker
├── tests/                   # Integration tests
├── evals/                   # Eval fixtures (YAML) + runner
├── bench/                   # Stress tests + ablation benchmarks
└── docs/                    # Design docs and implementation plans
```

### Crate responsibilities

| Crate | Purpose |
|-------|---------|
| `oni` | Binary crate — CLI parsing, server management, command dispatch |
| `oni-core` | Shared types, config loading (global + project merge), personality system, error macros |
| `oni-agent` | Core agent loop, 3-agent orchestrator, tool registry (11+ tools), message bus, knowledge graph, preference learning, telemetry |
| `oni-llm` | HTTP client for llama-server, per-tier model routing, health checks, reasoning config |
| `oni-tui` | Ratatui terminal UI — app state, input handling, widgets, inline shell, theming |
| `oni-db` | SQLite persistence for conversations, tool executions, and learned preferences |
| `oni-context` | Project file indexing, FTS5 search, embedding-based retrieval, code walker |

## License

This project is licensed under **CC-BY-NC-SA-4.0** — see [LICENSE](LICENSE) for details.
