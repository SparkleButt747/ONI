# ONI — Tech Stack

Actual technology as of the Rust rewrite. Verified against Cargo.toml files.

---

## Language & Build

| Choice | Detail |
|---|---|
| **Language** | Rust 2021 edition |
| **Build** | Cargo workspace, 6 crates + root binary |
| **Binary** | `oni` — path `src/main.rs` |

### Workspace crates

| Crate | Purpose |
|---|---|
| `oni-core` | Config, types, personality, error handling, palette |
| `oni-agent` | Agent loop, orchestrator, tools, conversation, preferences, reflection |
| `oni-ollama` | Ollama HTTP client, model router, health checks |
| `oni-tui` | Ratatui TUI, views, widgets, theming |
| `oni-db` | SQLite schema, conversation/message/tool-event persistence |
| `oni-context` | File indexing, symbol extraction, retrieval, file watcher, embeddings API |

---

## Runtime & Async

| Component | Crate | Version |
|---|---|---|
| Async runtime | `tokio` | 1 (full features) |
| Error handling | `color-eyre` | 0.6 |
| Logging | `tracing` | 0.1 |
| Log formatting | `tracing-subscriber` | 0.3 (env-filter feature) |

---

## TUI

| Component | Crate | Version |
|---|---|---|
| TUI framework | `ratatui` | 0.29 |
| Terminal backend | `crossterm` | 0.28 |
| Multi-line input | `tui-textarea` | 0.7 |
| Spinner widget | `throbber-widgets-tui` | 0.8 |

---

## HTTP / Networking

| Component | Crate | Version | Used by |
|---|---|---|---|
| Ollama client | `reqwest` | 0.12 (json feature) | `oni-ollama`, `oni-agent` (get_url tool) |

No separate HTTP client for the `get_url` tool — it reuses `reqwest` via `tokio::task::block_in_place`.

---

## Database

| Component | Crate | Version | Detail |
|---|---|---|---|
| SQLite | `rusqlite` | 0.33 | `bundled` + `modern_sqlite` features (statically linked) |

**WAL mode** enabled at schema creation (`PRAGMA journal_mode = WAL`).
**FTS5** used for symbol/file search in the context index.

### Schema overview

Main DB at `~/.local/share/oni/oni.db`:
- `conversations` — conv_id, source, timestamps, project_dir
- `messages` — conv_id FK, role, content, origin, tokens
- `tool_events` — session_id, tool_name, args_json, result_json, latency_ms
- `preference_signals` — session_id, tool_name, signal_type, context, weight
- `learned_rules` — description, context, confidence, observations, active

Context index at `.oni/index.db` (project-local):
- `files` — path, language
- `symbols` — name, kind, line, file FK — FTS5 indexed

---

## Serialisation

| Format | Crate | Version |
|---|---|---|
| JSON | `serde` + `serde_json` | 1 |
| YAML | `serde_yaml` | 0.9 |
| TOML | `toml` | 0.8 |

---

## CLI Parsing

| Crate | Version | Feature |
|---|---|---|
| `clap` | 4 | derive |

Subcommands: `chat`, `ask`, `run`, `sweep`, `review`, `doctor`, `init`, `index`, `prefs`, `config`, `pin`.

---

## File System

| Component | Crate | Version | Purpose |
|---|---|---|---|
| Config/data dirs | `dirs` | 6 | XDG-compatible paths (`config_dir`, `data_local_dir`) |
| Gitignore-aware walk | `ignore` | 0.4 | `.oniignore` + `.gitignore` support in `oni-context` |
| File watching | `notify` | 7 | Recursive file system events, 500 ms poll interval |

---

## Other Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `uuid` | 1 (v4 feature) | Conversation and message IDs |
| `regex` | 1 | Symbol extraction (replaces tree-sitter) |
| `tempfile` | 3 | Test isolation (dev-dependency) |

---

## LLM Backend

**Ollama** running at `localhost:11434` (configurable via `oni.toml`).

**Batch mode only.** All requests use `stream: false`. No streaming responses to the TUI — full response arrives then is displayed.

### Model tiers

Four tiers mapped to Ollama model names in `oni.toml`:

| Tier | Default role | Configurable |
|---|---|---|
| `heavy` | MIMIR (Planner) | Yes |
| `medium` | FENRIR (Executor) | Yes |
| `general` | code review, sweep | Yes |
| `fast` | SKULD (Critic), `oni ask` | Yes |
| `embed` | Embedding (nomic-embed-text) | Yes |

Model router (`oni-ollama/src/router.rs`) selects model name from config and passes `keep_alive` to Ollama to control model unloading.

### Native tool calling

`ModelRouter::chat_with_tools` sends tool schemas to Ollama in the `tools` field. Whether Ollama uses native tool calling or the agent falls back to text-parsed tool calls depends on model capability. Text-based parsing (`extract_text_tool_call` in `parsing.rs`) is the primary path; `strip_thinking` handles `<thinking>...</thinking>` blocks.

---

## Config

TOML format. Two layers merged at runtime:
1. `~/.config/oni/oni.toml` — global
2. `./oni.toml` — project (overrides global)

`oni config show` prints the merged result. `oni config set` is a stub — edit the file directly.

---

## Testing

| Tool | Purpose |
|---|---|
| `cargo test` | Unit + integration tests |
| `tempfile` | Isolated temporary directories in tests |

Integration tests in `tests/`. Eval fixtures (YAML) in `evals/`. Stress tests in `bench/`.

Test naming convention: `t_prefix_N` (e.g. `t_tool_14_bash_blocks_rm_rf_root`).

---

## Distribution

```
cargo build --release   # produces target/release/oni
cargo install --path .  # installs to ~/.cargo/bin/oni
```

No npm, no Homebrew tap, no pre-built binaries — Rust toolchain required.
