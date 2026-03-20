# ONI — Feature Reference

Features listed here exist in code. Nothing speculative.

---

## CLI Commands

| Command | Status | Notes |
|---|---|---|
| `oni chat` | Implemented | Interactive TUI. Flags: `--write`, `--exec`, `--tier`, `--autonomy`, `--budget`, `--fresh` |
| `oni ask` | Implemented | One-shot Q&A. Reads from stdin if no args. `--json` flag emits NDJSON event stream |
| `oni run` | Implemented | Headless agent loop. Background task management (`--background`, `--list`, `--kill`, `--logs`). Feature-flag knobs for A/B testing |
| `oni sweep` | Implemented | Codebase-wide task via headless agent. `--write`/dry-run modes, `--glob` filter |
| `oni review` | Implemented | Reviews staged git diff, emits issues with severity tags and a verdict |
| `oni config show` | Implemented | Prints merged config as TOML |
| `oni config set` | Stub | Prints path to edit; actual set not yet implemented |
| `oni prefs` | Implemented | Subcommands: `show`, `reset`, `export` (JSONL), `import`, `forget <tool>` |
| `oni index stats` | Implemented | Shows file/symbol counts from `.oni/index.db` |
| `oni index rebuild` | Implemented | Alias for `oni init` |
| `oni init` | Implemented | Indexes current project into `.oni/index.db` |
| `oni pin <path>` | Implemented | Restricts context retrieval to a subtree. `--reset` to clear |
| `oni doctor` | Implemented | Health-checks Ollama + each model tier, prints data dir and system info |

Default invocation (`oni` with no subcommand) launches chat with write+exec enabled.

---

## Agent System

### 3-Agent Orchestrator (MIMIR / FENRIR / SKULD)

Three model tiers map onto three roles:

- **MIMIR** (Heavy tier) — Planner. Decomposes tasks into ordered steps. Emits `PlanGenerated` event.
- **FENRIR** (Medium tier) — Executor. Works through plan steps with tool access. Emits `ExecutorStep` events.
- **SKULD** (Fast tier) — Critic. Reviews each step's output, returns accept/reject verdict. Emits `CriticVerdict` event.

The orchestrator is triggered by a heuristic (`should_orchestrate`) in `agent.rs`. For simple turns the agent runs flat (no planning loop).

### Multi-Trajectory Sampling

On Critic rejection, the Executor retries the step with up to `max_trajectories` (default: 2) alternative approaches before replanning. Controlled by `FeatureFlags::multi_trajectory`.

### Plan Persistence

Plans are saved to disk (per project directory hash) as `PersistedPlan` structs with per-step `StepStatus`. Allows resuming multi-step tasks across sessions.

### Replan Loop

On consecutive Critic rejections, MIMIR replans with the failure reason injected. Maximum 2 replan cycles before the orchestrator gives up and escalates to the user. Emits `Replanning` event.

### Context Compaction

Conversation history is compacted when token count approaches the configured threshold. Retention policy is configurable via `CompactionConfig`.

### Budget Tracking

Per-session token budget (`--budget` flag, 0 = unlimited). Monthly limit also configurable. `BudgetExhausted` event fires when hit.

### Autonomy Levels

Three levels (Low / Medium / High) gate when tool proposals require explicit user confirmation. Passed as `AutonomyLevel` enum to the agent.

### Feature Flags (`oni run`)

All major subsystems can be disabled individually for A/B testing via `--no-*` flags on `oni run`:
`knowledge_graph`, `reflection`, `personality`, `callbacks`, `compaction`, `multi_trajectory`, `orchestrator`, `auto_lint`, `emotional_state`, `forge_tool`, `undo_tracking`.

---

## Tools (11)

All tools implement the `Tool` trait (`name`, `description`, `schema`, `execute`). Read-only tools are always registered; write/exec tools gated by `allow_write`/`allow_exec` flags.

| Tool | Gate | Description |
|---|---|---|
| `read_file` | always | Reads file contents. Truncates at 100 KB with message |
| `write_file` | `--write` | Writes file. Snapshots to undo history first |
| `edit_file` | `--write` | Targeted in-place edit. Snapshots to undo history first |
| `bash` | `--exec` | Executes bash command. Optional `cwd`. Truncates output at 50 KB |
| `list_dir` | always | Lists directory contents |
| `search_files` | always | Regex search via ripgrep. Supports `file_pattern` glob filter |
| `get_url` | always | HTTP GET, strips HTML to text, capped at 50 KB. Blocks private/localhost addresses |
| `ask_user` | always | Pauses agent and sends a question to the TUI for user input |
| `forge_tool` | `--exec` | Generates and executes a one-off bash script. Syntax-checked before run |
| `undo` | always | Reverts last write/edit using snapshot stack (50 entries) |
| auto-linter | `--exec` implicit | Runs after every write/edit. Language-detected: Rust (clippy), Python (ruff), JS/TS (eslint). Returns truncated output or None if clean |

**Security on `bash` and `forge_tool`**: a shared blocklist rejects patterns including `rm -rf /`, `rm -rf /*`, `sudo rm`, `dd if=`, `mkfs`, fork bombs, pipe-to-shell patterns.

**`get_url` additional restrictions**: only `http://` and `https://` schemes; private network ranges (`localhost`, `127.0.0.1`, `10.*`, `192.168.*`, `169.254.*`) are blocked.

**`read_file` path traversal**: inherits OS-level access; no explicit traversal check beyond what the kernel enforces.

---

## TUI Views

Built with ratatui 0.29 + crossterm 0.28. Three views switchable at runtime.

### Chat View (`ui/chat.rs`)
- Conversation history rendered with markdown-like formatting (bold, code blocks, diff colouring)
- Scroll through history
- Inline diff view for write/edit tool calls
- Tiled dim background texture fills empty space
- Thinking/spinner state while LLM is generating
- Status bar shows model, tokens, tok/s

### Mission Control (`ui/mission_control.rs`)
- 4 BigText stat cards: turns, total tokens, tok/s, tool call count
- Scrollable tool call log (chronological, last N entries)
- Sub-agent status panel: Planner / Executor / Critic states
- Session info footer

### Preferences View (`ui/preferences.rs`)
- Lists all `learned_rules` from SQLite
- Colour-coded by confidence: ACTIVE (≥80%, lime), LEARNING (≥50%, amber), WEAK (dim)
- Shows confidence percentage, context tag, observation count per rule

### Additional UI components
- `splash.rs` — boot/splash screen
- `diff_view.rs` — unified diff rendering
- `error_state.rs` — error display
- `thinking.rs` — spinner/thinking indicator
- `command_menu.rs` — command palette
- `sidebar.rs`, `status.rs`, `response_label.rs` — layout helpers

### Input
- `tui-textarea` for multi-line input with history
- Inline shell (`:` prefix intercept within the TUI)

---

## Personality System

Lives in `oni-core/src/personality.rs`. Files stored at `~/.local/share/oni/`.

### SOUL.md
ONI's identity file: voice, opinions, working style. User-editable. Default template created during onboarding. Injected into every system prompt.

### USER.md
Owner profile generated during onboarding (name, role, working style, notes). Injected into system prompt.

### Emotional State (`inner-state.json`)
6 floating-point values (0.0–1.0): `confidence`, `curiosity`, `frustration`, `connection`, `boredom`, `impatience`.

Time-based decay applied on session start with per-dimension half-lives:
- Frustration: 4 h
- Connection: 48 h
- Curiosity: 72 h
- Boredom grows ~1 week to max without interaction

Updates triggered by: `on_success`, `on_failure`, `on_interaction`, `on_novelty`. Translated into prompt modifiers when thresholds are crossed (e.g. frustration > 0.5 → "take a step back").

### Relationship Progression (`relationship.json`)
State machine: Stranger → Acquaintance → Collaborator → Trusted → Aligned. Session-count thresholds: 3 / 15 / 50 / 150. Each stage injects behaviour modifiers (e.g. Trusted: "push back when their approach is wrong").

### Daily Journal
Journal entries written to `~/.local/share/oni/journal/YYYY-MM-DD.md` per session.

### Reflection Engine (`agent/src/reflection.rs`)
Heuristic analysis (no LLM call) of accumulated preference signals. Identifies high-trust tools (>90% accept, ≥10 observations) and produces `PersonalityMutation` proposals for SOUL.md additions. Run between sessions.

---

## Preference Learning

### Signal Recording (`preferences.rs`)
Four signal types: `Accept`, `Reject`, `Edit`, `Rerun`. Written to `preference_signals` table with tool name, context string, weight 1.0, session ID.

### Learned Rules
Stored in `learned_rules` table with `confidence`, `observations`, `active` flag. Active rules (confidence ≥ 0.8) are fetched and injected into the system prompt at session start.

### Rule Crystallisation
Reflection engine promotes patterns to rules. `oni prefs show` displays current rules with confidence/observation counts.

### Callback System (`callbacks.rs`)
Searches journal entries for keyword overlap with the current query. Fires ~20% of the time to avoid noise. Injects relevant past episode text into the system prompt.

---

## Context Engine

### Indexer (`oni-context/src/indexer.rs`)
`oni init` / `oni index rebuild` walks the project and extracts symbols by language using regex patterns. No tree-sitter — pure regex.

**Languages with symbol extraction**: Rust, Python, TypeScript, JavaScript, Go, Java, C#, Kotlin. Symbol kinds: `fn`/`struct`/`enum`/`trait`/`impl`/`type` (Rust), `def`/`class`/`method` (Python), `function`/`class`/`arrow`/`const`/`type` (TS/JS), `func`/`struct`/`interface` (Go), `class`/`method` (Java/C#/Kotlin).

Written to SQLite FTS5 tables: `files`, `symbols`. Stored at `.oni/index.db` (project-local).

### Walker (`walker.rs`)
Uses the `ignore` crate for gitignore-aware traversal. Respects `.oniignore` if present. Skips: `node_modules`, `.git`, `target`, `dist`, `build`, `__pycache__`, `.oni`, `.next`, `.turbo`, `.cache`, `coverage`, `.vscode`, `.idea`. Max file size: 512 KB.

### Retriever (`retriever.rs`)
FTS5 BM25 query against the symbol/file index. Token budget enforced (default 8 192 tokens ≈ chars/4). Pin restricts results to a path prefix stored in `.oni/pin`.

### File Watcher (`watcher.rs`)
Uses `notify 7` to watch the project directory recursively. Skips `.git/`, `node_modules/`, `target/`, `.oni/`. Poll interval: 500 ms. Sends changed paths through a channel for incremental re-indexing.

### `.oni-context` file
If `.oni-context` exists in the project root, its contents are injected verbatim into the system prompt. Project-specific context without re-indexing.

### Vector Embeddings (`embeddings.rs`)
API implemented (calls Ollama `nomic-embed-text`). Cosine similarity helper included. **Not wired into retrieval** — BM25 is the active retrieval path.

---

## Security Model

| Control | Mechanism |
|---|---|
| Read-only by default | `write_file` and `edit_file` only registered when `--write` flag passed |
| No exec by default | `bash` and `forge_tool` only registered when `--exec` flag passed |
| Bash blocklist | Pattern-matched against normalised command (lowercase, collapsed whitespace) |
| Private URL blocking | `get_url` rejects localhost, 127.0.0.1, 10.x, 192.168.x, 169.254.x |
| Forge safety | Same blocklist as bash + bash syntax check (`bash -n`) before execution |
| Output truncation | `read_file` ≤ 100 KB, `bash` ≤ 50 KB, `get_url` ≤ 50 KB |
| Undo stack | 50-entry file snapshot stack; `undo` tool reverts last write/edit |

---

## Available But Not Wired

| Feature | Location | Status |
|---|---|---|
| MessageBus | `oni-agent/src/message_bus.rs` | Implemented, not connected to any agent path |
| ExecutionTrace | `oni-agent/src/trace.rs` | Implemented, not persisted or exposed |
| Vector embeddings | `oni-context/src/embeddings.rs` | Ollama API wired, not used in retrieval |
| KnowledgeGraph | `oni-agent/src/knowledge_graph.rs` | In-memory graph with persistence, not wired into agent loop |
