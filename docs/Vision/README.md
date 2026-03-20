# ONI — Onboard Native Intelligence

Local-first AI assistant powered by Ollama. Runs entirely on your machine. No cloud, no API keys, no telemetry.

---

## What it is

ONI is a terminal AI assistant built in Rust with a Ratatui TUI. It uses a 3-agent orchestrator (Planner → Executor → Critic) backed by local Ollama models, with a full tool system, persistent personality, preference learning, and a codebase context engine.

---

## Key Features

- **3-agent orchestration** — MIMIR (planner) decomposes tasks into steps; FENRIR (executor) runs them with tools; SKULD (critic) reviews each step and can replan up to 2 cycles
- **11 tools** — read/write/edit file, bash, list directory, search files, get URL, ask user, undo, forge (dynamic tool generation), and tool gating via `--write`/`--exec` flags
- **Personality system** — SOUL.md (identity/voice), USER.md (owner profile), 6-value emotional state with time decay, 5-stage relationship tracker, daily session journal
- **Preference learning** — records accept/reject/edit signals per tool, promotes high-confidence rules into the system prompt automatically
- **Context engine** — FTS5 BM25 retrieval on an indexed project database, symbol extraction for 7 languages, file watcher for incremental updates, `.oniignore` support
- **Mission Control view** — ratatui TUI with Chat, MissionControl, and Preferences panes; sub-agent status, tool call log, diff previews, burn rate display
- **Knowledge graph** — in-memory, JSON-persisted cross-session discovery store with typed nodes and edges
- **Autonomy levels** — Low/Medium/High; controls which tool calls require user confirmation
- **Batch mode only** — no streaming; responses arrive in full (Ollama `stream: false`)

---

## Quick Start

```bash
# Prerequisites: Rust toolchain + Ollama running locally
cargo build --release

# One-shot question
cargo run -- ask "what is a race condition"

# Interactive chat (read-only)
cargo run -- chat

# Interactive chat with file write and shell execution
cargo run -- chat --write --exec

# Global install
cargo install --path .
oni chat --write --exec --autonomy high

# Check model availability
oni doctor

# Index current project for context retrieval
oni init
```

---

## CLI Commands

| Command | Purpose |
|---------|---------|
| `oni chat` | Interactive TUI session |
| `oni ask <question>` | One-shot question (also reads stdin) |
| `oni run <prompt>` | Headless agent run for benchmarking |
| `oni doctor` | Check Ollama + model health |
| `oni init` | Index project files for context |
| `oni sweep <goal>` | Codebase-wide autonomous task |
| `oni review` | Review staged git changes |
| `oni prefs` | Show/manage learned preferences |
| `oni pin <path>` | Pin context retrieval to a subtree |
| `oni config` | Show configuration |

---

## Architecture Overview

6-crate Cargo workspace:

| Crate | Role |
|-------|------|
| `oni-core` | Config, types, personality system, error handling |
| `oni-agent` | Agent loop, orchestrator, tools, conversation, preferences, knowledge graph |
| `oni-ollama` | Ollama HTTP client, model router (5 tiers), health checks |
| `oni-tui` | Ratatui TUI app, 3 views, widgets, theming |
| `oni-db` | SQLite persistence (conversations, tools, preferences, rules) |
| `oni-context` | File indexer, FTS5 retrieval, symbol extraction, file watcher |

Entry point: `src/main.rs` (clap CLI, routes to crates).

---

## Configuration

TOML format. Merged in order: built-in defaults → `~/.config/oni/oni.toml` → `./.oni/oni.toml` (project).

Model tiers map to actual Ollama model names in `oni.toml`. Defaults:

| Tier | Purpose | Default |
|------|---------|---------|
| Heavy | Planning, reasoning | `qwq:32b` |
| Medium | Execution, coding | `qwen2.5-coder:14b` |
| General | Critic, review | `llama3.1:8b` |
| Fast | Quick answers | `llama3.2:3b` |
| Embed | Context embedding | `nomic-embed-text` |

---

## Document Index

| File | Contents |
|------|---------|
| `README.md` | Project overview (this file) |
| `ARCHITECTURE.md` | Detailed architecture — crates, data flows, subsystems |
| `FEATURES.md` | Full feature specifications |
| `DESIGN_SYSTEM.md` | Visual language, colour palette, theming |
| `SECURITY.md` | Permissions model, threat model |
| `FUTURE_WORK.md` | Planned work, unactivated infrastructure |
