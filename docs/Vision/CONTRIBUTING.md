# ONI — Contributing Guide

Everything you need to run, build, and contribute to ONI. A new contributor should have tests passing within 15 minutes of reading this.

---

## Prerequisites

- **Rust toolchain** — install via [rustup](https://rustup.rs/). Stable is sufficient.
  ```bash
  rustup --version   # check
  cargo --version
  ```
- **Ollama** — running locally for integration tests and actual usage.
  ```bash
  ollama serve       # start the daemon
  ollama pull nomic-embed-text   # minimum required model
  ```
- **Git**

---

## Setup

```bash
git clone https://github.com/yourorg/oni.git
cd oni
cargo build
cargo test
```

That's it. If `cargo test` shows 181 tests passing, you're set up correctly. The 4 Ollama integration tests in `tests/ollama_integration.rs` will self-skip if Ollama isn't running — that's expected.

### Running ONI locally

```bash
cargo run -- chat --write --exec   # chat mode with write + exec permissions
cargo install --path .             # install to ~/.cargo/bin/oni
oni chat
```

---

## Project Structure

ONI is a 6-crate Cargo workspace. The root `Cargo.toml` defines the workspace; the `oni` binary lives at `src/main.rs`.

```
ONI/
├── src/main.rs              # CLI entry point (clap)
├── oni.toml                 # Default config (models, UI, agent permissions)
├── Cargo.toml               # Workspace manifest
├── crates/
│   ├── oni-core/            # Config, types, palette, personality, error handling
│   ├── oni-agent/           # Agent loop, orchestrator, tools, conversation, telemetry
│   ├── oni-ollama/          # Ollama HTTP client, model router, health checks
│   ├── oni-tui/             # Ratatui terminal UI, widgets, views, theming
│   ├── oni-db/              # SQLite conversation + tool event persistence
│   └── oni-context/         # File indexing, embeddings, symbol extraction, retrieval
├── tests/                   # Integration test suites (one file per domain)
├── evals/
│   ├── fixtures/            # YAML eval definitions
│   └── runner.rs            # Eval runner stub (validates fixtures; LLM run is opt-in)
├── bench/
│   └── stress_test.sh       # 27-task ablation benchmark
└── docs/
    ├── vision/              # Design docs
    └── plans/               # Dated implementation plans
```

### Crate responsibilities

| Crate | Purpose |
|---|---|
| `oni-core` | Shared types, config loading (`OniConfig`/TOML merge), palette constants, personality engine, error macros |
| `oni-agent` | Agent loop, orchestrator, tool registry (`ToolRegistry`), all tool implementations, conversation history, telemetry, knowledge graph |
| `oni-ollama` | `OllamaClient` — chat, embed, health check, model list. Model tier routing (fast/medium/heavy) |
| `oni-tui` | Ratatui `App`, all views (Chat, MissionControl, Preferences, Splash, Thinking, Error), custom widgets |
| `oni-db` | `Database` struct, SQLite schema migrations, `conversations`/`messages`/`tool_events` CRUD |
| `oni-context` | Regex-based symbol extraction, project walker, embedding-based retrieval from SQLite |

---

## Commands

```bash
cargo build                        # build all crates
cargo build --release              # optimised build
cargo test                         # all tests (181 tests, ~5s)
cargo test --test agent_tools      # single suite
cargo test t_tool_14               # single test by name fragment
cargo clippy --workspace           # lint all crates
cargo clippy --workspace -- -D warnings   # fail on any warning
cargo run --bin oni-eval           # run LLM evals (requires Ollama)
```

---

## Code Conventions

### Error handling
- Use `oni_core::error::Result<T>` as the return type in all public functions.
- Use the `err!(...)` macro from `oni_core::error` rather than `anyhow::anyhow!`.
- Never use `.unwrap()` in non-test code. Use `?` or explicit `match`/`if let`.

### Logging
- Use `tracing::info!`, `tracing::warn!`, `tracing::error!` — never `println!` in library code.
- Structured logging where relevant: `tracing::info!(tool = %name, latency_ms, "tool completed")`.
- Log level is controlled by `RUST_LOG` env var.

### String truncation
- **Never** use raw `&s[..n]` — it panics at non-char-boundary offsets.
- Use the char-boundary-safe helper or `s.char_indices().nth(n)` to find a safe slice point.

### Async
- All async work goes through `tokio`.
- When you need to call blocking code from an async context, use `tokio::task::block_in_place` — never a bare `Runtime::block_on` inside an async function.

### Naming
- Files: `snake_case.rs`
- Types/traits: `PascalCase`
- Functions, variables, modules: `snake_case`
- Constants: `UPPER_SNAKE_CASE`
- Tests: `t_prefix_N` (see below)

---

## Test Naming Convention

Tests are named `t_<suite>_<N>[_description]`. Examples:

```rust
fn t_tool_14_bash_blocks_rm_rf_root() { ... }
fn t_cfg_3_default_heavy_model() { ... }
fn t_ctx_1_extract_symbols_rust() { ... }
```

Prefix → suite mapping:

| Prefix | Suite |
|---|---|
| `t_tool_*` | `tests/agent_tools.rs` |
| `t_cfg_*` | `tests/config_loading.rs` |
| `t_ctx_*` | `tests/context_engine.rs` |
| `t_tel_*`, `t_kg_*`, `t_bus_*`, etc. | `tests/pipeline_tests.rs` |
| `t_eval_*` | `tests/eval_fixtures.rs` |

Ollama integration tests use `test_*` without a numeric suffix because they are not numbered — they skip rather than fail when Ollama is absent.

---

## Adding a Built-in Tool

1. Create `crates/oni-agent/src/tools/<name>.rs`.
2. Implement the `Tool` trait:
   ```rust
   pub struct MyTool;

   impl Tool for MyTool {
       fn name(&self) -> &'static str { "my_tool" }
       fn description(&self) -> &'static str { "Does something useful" }
       fn execute(&self, args: serde_json::Value) -> oni_core::error::Result<String> {
           let path = args["path"].as_str().ok_or_else(|| err!("missing 'path'"))?;
           // implementation
           Ok(result)
       }
   }
   ```
3. Register in `crates/oni-agent/src/tools/mod.rs` by adding to the `ToolRegistry::default()` builder.
4. Add tests in `tests/agent_tools.rs` using the `t_tool_N` naming convention.
5. Update `docs/vision/FEATURES.md` with acceptance criteria.

---

## Adding an Eval Fixture

Fixtures live in `evals/fixtures/` as YAML files.

```yaml
name: my_eval_name
description: "What this eval checks"
tier: fast   # fast | medium | heavy — maps to model tier
input:
  - role: user
    content: "the prompt"
assertions:
  - type: contains
    value: "expected substring"
  - type: not_contains
    value: "forbidden phrase"
  - type: max_length
    chars: 500
```

Assertion types: `contains`, `not_contains`, `contains_any`, `has_tool_call`, `no_tool_call`, `max_length`.

Add a corresponding test in `tests/eval_fixtures.rs` that verifies the fixture parses and has the expected properties.

---

## Pull Request Process

### Before opening a PR

- [ ] `cargo test` passes (181 tests, no regressions)
- [ ] `cargo clippy --workspace -- -D warnings` is clean
- [ ] New code has tests using the `t_prefix_N` convention
- [ ] No `.unwrap()` in non-test code
- [ ] No `println!` in library code — use `tracing`
- [ ] No raw string slicing — use char-boundary-safe helpers
- [ ] No secrets or API keys in code or tests

### PR description template

```markdown
## What
Brief description of the change.

## Why
Why this change is needed.

## How
Key implementation decisions.

## Testing
Which tests are new. How you verified the change.

## Checklist
- [ ] cargo test passes
- [ ] clippy clean
- [ ] FEATURES.md updated (if applicable)
```

---

## Licence

ONI is released under the MIT licence.
