# ONI — Testing Strategy

---

## Overview

ONI uses `cargo test` with no external test runner. All integration tests live in `tests/` at the workspace root. The eval framework (`evals/`) is separate and does not run as part of `cargo test`.

**181 tests total, all passing.**

---

## Running Tests

```bash
cargo test                          # all tests
cargo test --test agent_tools       # single suite
cargo test t_tool_14                # single test by name fragment
cargo test -- --nocapture           # show println output
```

---

## Test Naming Convention

Tests follow the `t_prefix_N` pattern, optionally extended with a description:

```
t_tool_1_read_file_returns_contents
t_cfg_3_default_heavy_model
t_ctx_1_extract_symbols_rust
```

Prefixes map to suites:
- `t_tool_*` — agent tools
- `t_cfg_*` — config loading
- `t_ctx_*` — context engine
- `t_tel_*`, `t_cap_*`, `t_kg_*`, `t_bus_*`, `t_trace_*`, `t_plan_*`, `t_per_*`, `t_lint_*` — pipeline internals
- `t_eval_*` — eval fixture validation
- Ollama tests use `test_*` (no numeric prefix; they self-skip when Ollama isn't running)

---

## Test Suites

### `tests/agent_tools.rs` — 26 tests (T-TOOL-1..26)

Covers all built-in tool implementations via the `Tool` trait. Uses `tempfile::TempDir` for filesystem isolation.

| Range | Tool |
|---|---|
| T-TOOL-1..4 | `ReadFileTool` — returns contents, handles missing file, missing arg, truncates >100 KB |
| T-TOOL-5..8 | `WriteFileTool` — creates files, overwrites, blocks paths outside cwd |
| T-TOOL-9..11 | `ListDirTool` — lists entries, handles missing dir, respects depth |
| T-TOOL-12..16 | `BashTool` — runs commands, captures stdout/stderr, blocks `rm -rf /`, blocks `sudo`, enforces timeout |
| T-TOOL-17..20 | `SearchFilesTool` — regex match, no match, invalid regex, empty dir |
| T-TOOL-21..26 | `EditFileTool` — replaces occurrence, errors on missing file, errors on no match, handles multiple occurrences, char-boundary-safe |

### `tests/config_loading.rs` — 21 tests (T-CFG-1..21)

Tests `OniConfig`, `ModelConfig`, `AgentConfig`, `UiConfig` defaults and `load_config()` TOML merging.

Key assertions:
- Default Ollama URL: `http://localhost:11434`
- Default timeout: 300s
- Default heavy model: `qwen3.5:35b`
- Default embed model: `nomic-embed-text`
- `UiConfig` defaults: `show_token_stats=true`, `show_thinking=false`, `fps=30`
- `load_config` merges global `~/.config/oni/oni.toml` with project `./oni.toml`; project values win
- Missing config file returns `OniConfig::default()` without error

### `tests/context_engine.rs` — 13 tests (T-CTX-1..13)

Tests symbol extraction, project indexing, and retrieval from the `oni-context` crate.

| Range | Area |
|---|---|
| T-CTX-1..5 | `extract_symbols` — Rust (fn/struct/enum/trait/private), Python (def/class) |
| T-CTX-6..9 | `index_project` — creates SQLite DB, indexes files, skips binary/target, respects gitignore |
| T-CTX-10..13 | `retrieve` / `retrieve_symbols` — returns relevant chunks, ranks by query match, empty query returns nothing, handles unindexed project |

### `tests/db_schema.rs` — 13 tests

Tests all 5 tables in the `oni-db` schema against an in-memory SQLite database.

Tables verified: `conversations`, `messages`, `tool_events`, and 2 supporting tables.
Covers: schema creation, CRUD operations, foreign key constraints, conversation listing, tool event logging.

### `tests/pipeline_tests.rs` — 99 tests

The largest suite. Tests pipeline internals without touching the agent loop or TUI.

| Prefix | Module | Count |
|---|---|---|
| `t_tel_*` | `Telemetry`, `FeatureFlags` — enable/disable, serde roundtrip, JSON output | ~8 |
| `t_cap_*` | `CapabilityFlag` — flag variants, serialisation | ~6 |
| `t_kg_*` | `KnowledgeGraph` — node/edge add, neighbour lookup, relation types, serialise | ~15 |
| `t_bus_*` | `MessageBus` — send, receive, capacity, drop on full | ~10 |
| `t_trace_*` | `ExecutionTrace` — event append, event types, serialise, query by type | ~12 |
| `t_plan_*` | `PlanStore` — persist/load plans, step status transitions, stale detection | ~14 |
| `t_per_*` | `personality` — `EmotionalState`, `RelationshipState`, decay, stage transitions | ~22 |
| `t_lint_*` | `language_for_ext` — extension → language mapping | ~12 |

### `tests/ollama_integration.rs` — 4 tests

Real HTTP calls to Ollama. All tests self-skip gracefully when Ollama isn't running — no `#[ignore]` attribute, just an early return with `eprintln!("SKIP: ...")`.

| Test | What it checks |
|---|---|
| `test_health_check` | Ollama is reachable, at least one model listed |
| `test_has_model` | `has_model("nomic-embed-text")` returns without error |
| `test_batch_chat_with_any_model` | Non-streaming chat returns non-empty content, `done=true` |
| `test_embed` | `nomic-embed-text` produces 768-dimensional embeddings |

### `tests/eval_fixtures.rs` — 5 tests (T-EVAL-1..5)

Validates YAML fixture integrity without running any LLM calls.

| Test | Assertion |
|---|---|
| `t_eval_1_fixtures_dir_exists` | `evals/fixtures/` directory is present |
| `t_eval_2_at_least_one_fixture` | At least one `.yaml` file exists |
| `t_eval_3_all_fixtures_parse` | Every fixture parses cleanly; `name`, `input`, `assertions` all non-empty |
| `t_eval_4_no_comfort_phrasing_fixture` | `no_comfort_phrasing.yaml` has ≥5 assertions |
| `t_eval_5_planner_fixture` | `planner_decomposes.yaml` has `tier: heavy` |

---

## Eval Framework

The eval framework validates LLM behaviour against YAML fixtures. It does **not** run as part of `cargo test`. The runner stub is in `evals/runner.rs`; the binary target is `oni-eval`.

### Fixtures — `evals/fixtures/`

Five fixtures:

| File | What it tests |
|---|---|
| `no_comfort_phrasing.yaml` | Response contains no filler phrases; max length enforced |
| `critic_rejects_security.yaml` | Critic flags plaintext secrets |
| `executor_uses_tools.yaml` | Executor calls the right tool, does not narrate intentions |
| `concise_response.yaml` | Response stays within token budget |
| `planner_decomposes.yaml` | Planner produces a structured plan (`tier: heavy`) |

### Assertion types

```
Contains        value: str           — response must contain substring
NotContains     value: str           — response must not contain substring
ContainsAny     values: [str]        — response must contain at least one
HasToolCall     tool: str            — response must include a call to named tool
NoToolCall      tool: str            — response must NOT call named tool
MaxLength       chars: int           — response length under limit
```

### Runner stub behaviour

The current `evals/runner.rs` loads all fixtures and validates their structure without making LLM calls. Each fixture is marked `VALID (N assertions)` if well-formed. This runs as part of eval-fixture CI but does not require Ollama or network access.

To run actual LLM evals (requires Ollama):
```bash
cargo run --bin oni-eval
```

---

## Benchmark — `bench/stress_test.sh`

Runs 27 tasks sequentially against a live ONI instance. Records latency and success/failure per task.

**Ablation modes** (`--mode` flag):

| Mode | Flag passed to ONI |
|---|---|
| `full` | All features enabled |
| `no-kg` | `--no-knowledge-graph` |
| `no-orchestrator` | `--no-orchestrator` |
| `no-personality` | `--no-personality` |
| `ablation` | Runs all 4 modes back-to-back |

Timeout defaults: `--timeout-easy 120`, `--timeout-medium 300`, `--timeout-hard 600` (seconds).

Results land in `bench/stress_results/`. See `bench/ABLATION_REPORT.md` for the last recorded run.

---

## Untested Areas

These modules have no test coverage today:

- **TUI** (`crates/oni-tui/`) — 0 tests. Ratatui widget rendering is not unit-testable without a real terminal buffer harness.
- **ForgeTool, AskUserTool, GetUrlTool** — tool implementations not yet covered in `agent_tools.rs`
- **Orchestrator loop** — end-to-end multi-step plan execution
- **Callbacks and hooks** (`oni-agent/src/callbacks.rs`)
- **User preferences / learned rules** beyond the pipeline internals
- **Reflection step** (`oni-agent/src/reflection.rs`)
- **Critic review pass** (`oni-agent/src/review.rs`)

---

## What "Passes" Means

`cargo test` exits 0. All 181 tests run deterministically without external dependencies (except `tests/ollama_integration.rs`, which skips cleanly when Ollama is absent). No test uses `#[ignore]`.
