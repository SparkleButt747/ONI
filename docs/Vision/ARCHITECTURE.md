# ONI — System Architecture

---

## Crate Structure

6-crate Cargo workspace rooted at the repo root.

| Crate | Path | Role |
|-------|------|------|
| `oni-core` | `crates/oni-core/` | Config (TOML), shared types, personality system, error handling macros |
| `oni-agent` | `crates/oni-agent/` | Agent loop, orchestrator, tool registry, conversation, preferences, knowledge graph, telemetry |
| `oni-ollama` | `crates/oni-ollama/` | Ollama HTTP client, model router, health checks |
| `oni-tui` | `crates/oni-tui/` | Ratatui TUI app, 3 views, widgets, theming |
| `oni-db` | `crates/oni-db/` | SQLite persistence (rusqlite, WAL mode) |
| `oni-context` | `crates/oni-context/` | File indexer, FTS5 retrieval, symbol extraction, file watcher |

Entry point: `src/main.rs` — clap CLI, routes subcommands to crates.

---

## Agent System

### Orchestrator

The orchestrator runs when `should_orchestrate()` returns true. Heuristics that trigger it:

- Prompt contains multi-file keywords (refactor, migrate, audit, implement across, etc.)
- 2 or more weak signals (test, write, update, create, analyse, find, check, add, remove, fix)
- Prompt is a numbered list
- Long prompt (>200 chars) with at least 1 weak signal

Otherwise the agent loop handles the request directly.

### Three-Agent Pipeline

```
User prompt
    │
    ▼
MIMIR (Planner) — ModelTier::Heavy
    │  Decomposes the task into ordered steps.
    │  Produces a PersistedPlan saved to the project directory.
    ▼
FENRIR (Executor) — ModelTier::Medium
    │  Executes each step in the plan using the tool registry.
    │  Records preference signals on successful tool execution.
    ▼
SKULD (Critic) — ModelTier::General
    │  Reviews executor output against the original intent.
    │
    ├── accept → done
    │
    └── reject
          │
          ├── try alternative trajectory (max_trajectories=2)
          │     re-run FENRIR with same step, different approach
          │
          └── if still rejected → replan (max_replan_cycles=2)
                    │
                    └── back to MIMIR for revised plan
                          if replan budget exhausted → surface to user
```

Key constants: `max_replan_cycles = 2`, `max_trajectories = 2`.

### Direct Agent Loop

For non-orchestrated requests the agent loop runs a single conversation turn:

1. Build system prompt (personality + learned rules + context chunks)
2. Call `ModelRouter::chat_with_tools()` — batch mode, no streaming
3. Execute any tool calls returned by the model
4. Record preference signals
5. Apply auto-lint if `feature_flags.auto_lint` and the tool was `write_file` or `edit_file`
6. Check compaction trigger; compact if needed
7. Return response to TUI or stdout

---

## Tool System

### ToolRegistry

```rust
ToolRegistry::new_with_channels(allow_write: bool, allow_exec: bool, ask_user_channel: Option<AskUserChannel>)
```

Tools registered unconditionally:

| Tool | Description |
|------|-------------|
| `read_file` | Read file contents |
| `list_dir` | List directory entries |
| `search_files` | Regex search across files |
| `get_url` | Fetch a URL |
| `undo` | Revert last write/edit operation |

Tools registered conditionally:

| Tool | Condition |
|------|-----------|
| `ask_user` | Only when an `AskUserChannel` is provided (TUI sessions) |
| `write_file` | `allow_write` flag (`--write` CLI flag) |
| `edit_file` | `allow_write` flag |
| `bash` | `allow_exec` flag (`--exec` CLI flag) |
| `forge_tool` | `allow_exec` flag — dynamic tool generation at runtime |

### Undo System

`UndoHistory` capacity: 50 snapshots. Before every `write_file` or `edit_file` call, the registry snapshots the target path. The `undo` tool restores the most recent snapshot.

### Autonomy Levels

`AutonomyLevel::Low / Medium / High` — controls which tool calls require user confirmation before execution.

---

## Conversation Management

### Token Estimation

ONI estimates token count from character count (no tokeniser dependency). Compaction triggers when either threshold is exceeded:

| Trigger | Default |
|---------|---------|
| `token_threshold` | 19,660 (≈ 60% of 32,768) |
| `message_threshold` | 40 messages |

### Compaction

When the threshold is hit, the conversation history is pruned. A `retention_window` of recent messages is kept verbatim; older messages are dropped. No summarisation pass — raw history is truncated. The compacted history continues into the next request.

---

## Context Engine

### Indexing

`oni init` (or `oni index`) walks the project tree and builds a SQLite index at `.oni/context.db`.

- Walker uses the `ignore` crate (`WalkBuilder`) with `.oniignore` support
- Files larger than 512 KB are skipped
- `ALWAYS_SKIP` dirs: `node_modules`, `.git`, `target`, `dist`, `build`, `__pycache__`, `.next`, `.cache`, `vendor`
- FTS5 tables: `files_fts` (porter unicode61 tokenizer), `symbols_fts`
- Symbol extraction by language via regex:

| Language | Extracted symbols |
|----------|-------------------|
| Rust | `fn`, `struct`, `enum`, `trait`, `impl` |
| Python | `def`, `class` |
| TypeScript/JavaScript | `function`, `class`, `const/let/var` (arrow fns) |
| Go | `func`, `type` |
| Java / C# / Kotlin | `class`, `interface`, `fun/void/public` methods |

### File Watcher

`notify` crate watches the project root. On file change events:

1. Debounce
2. Re-index changed file via `index_single_file()`
3. Update `files_fts` and `symbols_fts` incrementally

### Retrieval

`retrieve(query, conn)`:

1. FTS5 BM25 query against `files_fts` — `ORDER BY rank ASC` (lower rank = more relevant in SQLite FTS5), `LIMIT 50`
2. Fit results into token budget: `DEFAULT_TOKEN_BUDGET = 8192`, estimated at 4 chars/token
3. Return ranked chunks

`retrieve_symbols(query, conn)`:

1. FTS5 BM25 query against `symbols_fts`
2. Deduplicate by file path
3. Return symbol list

**Context pinning:** If `.oni/pin` exists, retrieval is scoped to the pinned path subtree (`oni pin <path>`).

**Project context file:** `.oni-context` in the project root is injected directly into the system prompt.

---

## Personality System

All personality files live at `~/.local/share/oni/`.

| File | Role |
|------|------|
| `SOUL.md` | ONI's identity, voice, and core values. Edited by the user. |
| `USER.md` | Owner profile (name, role, working style). Populated via onboarding. |
| `inner-state.json` | Serialised `EmotionalState` — persists between sessions. |
| `relationship.json` | Serialised `RelationshipState` — tracks session count and stage. |
| `journal/YYYY-MM-DD.md` | Daily session journal entries. |

### EmotionalState

Six f64 values, each time-decaying independently:

| Value | Decay / Recovery |
|-------|-----------------|
| `confidence` | Decays toward 0; recovers toward 0.7 over time |
| `curiosity` | Half-life ≈ 72 hours |
| `frustration` | Half-life ≈ 4 hours (fades quickly) |
| `connection` | Half-life ≈ 48 hours |
| `boredom` | Grows at rate 1/168h (weekly accumulation) |
| `impatience` | Half-life ≈ 8 hours |

### RelationshipState

Five stages based on cumulative session count:

| Stage | Sessions required |
|-------|------------------|
| Stranger | 0 |
| Acquaintance | 3 |
| Collaborator | 15 |
| Trusted | 50 |
| Aligned | 150 |

### Prompt Assembly

`build_personality_prompt()` assembles:

1. `SOUL.md` full text
2. `USER.md` full text
3. Emotional modifier clauses (e.g. "you are slightly frustrated")
4. Relationship modifier clauses (e.g. "you know this person well")
5. Recent journal entries — last N days, truncated to 500 chars per day

### Onboarding

If `USER.md` does not exist, the TUI enters `OnboardingStep` flow to collect the owner profile before the first session.

### Reflection

On session end, a reflection pass can write a journal entry summarising the session. Controlled by `feature_flags.reflection`.

---

## Preference Learning

### Signal Recording

Every tool execution outcome is recorded as a `preference_signal`:

| Signal type | Condition |
|-------------|-----------|
| `accept` | Tool executed successfully and user did not reject |
| `reject` | User declined the tool proposal |
| `edit` | User modified the tool arguments before execution |
| `rerun` | User requested re-execution |

### Scoring

Signals are scored with time decay. Confidence rises toward 1.0 on repeated accepts, decays toward 0 on rejects and with age.

### Rule Crystallisation

Background process (on session end):

1. Query `preference_signals` for tool/intent pairs with `confidence >= 0.8`
2. Insert or update row in `learned_rules` with `active = 1`
3. On next session, active rules are injected into the system prompt verbatim

Example injected rule: `"When the user asks to edit a file, prefer edit_file over write_file."`

---

## Knowledge Graph

In-memory store, JSON-persisted at `~/.local/share/oni/knowledge-graph.json`.

**Node types:** `Discovery`, `Fact`, `FileContext`, `Pattern`, `UserPreference`, `Error`

**Edge relations:** `RelatedTo`, `CausedBy`, `DependsOn`, `Resolves`, `Contradicts`, `Supersedes`

Structure: `HashMap<String, KnowledgeNode>` + `Vec<KnowledgeEdge>`. Nodes are keyed by UUID. Relevance scoring and garbage collection run periodically to prune stale nodes.

Controlled by `feature_flags.knowledge_graph`.

---

## TUI

Built with `ratatui` + `crossterm`. Owns the terminal exclusively — ONI never writes to stdout during a TUI session. All logs go to `~/.local/share/oni/oni.log`.

### Views

| View | Key | Description |
|------|-----|-------------|
| Chat | `1` or default | Conversational REPL with message history |
| MissionControl | `2` | Sub-agent status, tool call log, diff previews, burn rate |
| Preferences | `3` | Learned rules and preference signal browser |

### MissionControl Widgets

- `SubAgentStatus` — live status of MIMIR, FENRIR, SKULD
- `ToolDetail` — inline previews: diff view for `write_file`/`edit_file`, command block for `bash`
- Burn rate display — token usage estimate

### Slash Commands

Available in the chat input:

| Command | Effect |
|---------|--------|
| `/clear` | Clear conversation history |
| `/pin <path>` | Pin context retrieval to a subtree |
| `/prefs` | Show learned preference rules |
| `/autonomy <level>` | Set autonomy level (low/medium/high) |

---

## Database

SQLite with WAL mode and foreign key enforcement. Managed by `oni-db`.

### Tables

| Table | Purpose |
|-------|---------|
| `conversations` | Session records |
| `messages` | All turns (user + assistant) with role and token estimates |
| `tool_events` | Every tool call with timing and outcome |
| `preference_signals` | Raw accept/reject/edit/rerun signals per tool call |
| `learned_rules` | Crystallised rules with confidence, observation count, active flag |

`preference_signals.signal_type` is constrained: `CHECK (signal_type IN ('accept','reject','edit','rerun'))`.

`learned_rules.active = 1` means the rule is currently injected into the system prompt.

---

## Configuration

TOML format. Loaded and deep-merged in order:

1. Built-in defaults (compiled in)
2. `~/.config/oni/oni.toml` (user global)
3. `./.oni/oni.toml` (project-local, if present)

### Model Tiers

| Tier | Default model | Temperature | Context window | Used by |
|------|--------------|-------------|----------------|---------|
| Heavy | `qwq:32b` | 0.3 | 32,768 | MIMIR (planner) |
| Medium | `qwen2.5-coder:14b` | 0.2 | 32,768 | FENRIR (executor) |
| General | `llama3.1:8b` | 0.3 | 16,384 | SKULD (critic), review |
| Fast | `llama3.2:3b` | 0.1 | 8,192 | Quick answers, `oni ask` |
| Embed | `nomic-embed-text` | — | — | Context embeddings |

Model names are overridable per tier in `oni.toml`.

### Relevant Config Sections

```toml
[agent]
compaction_token_threshold = 19660
compaction_message_threshold = 40

[models]
heavy = "qwq:32b"
medium = "qwen2.5-coder:14b"
general = "llama3.1:8b"
fast = "llama3.2:3b"
embed = "nomic-embed-text"
```

---

## Model Routing

`oni-ollama::ModelRouter` handles all Ollama HTTP calls.

- All requests use `stream: false` — batch mode only, responses arrive complete
- `chat()` for plain conversation turns
- `chat_with_tools()` for native Ollama tool calling (used by FENRIR and the direct agent loop)
- `embed()` delegates to the Embed tier model

The router selects the Ollama model name from `OniConfig::models` based on the requested `ModelTier`.

---

## Telemetry

Per-session stats collected in a thread-safe `Arc<Mutex<TelemetryInner>>`.

### Feature Flags

11 flags, all default `true`. Each can be disabled at runtime via `oni run --no-<flag>`:

| Flag | Controls |
|------|----------|
| `knowledge_graph` | Knowledge graph read/write |
| `reflection` | End-of-session reflection journal entry |
| `personality` | Personality prompt assembly |
| `callbacks` | Hook callbacks |
| `compaction` | Conversation compaction |
| `multi_trajectory` | Alternative trajectory sampling on critic reject |
| `orchestrator` | 3-agent orchestrator (falls back to direct loop) |
| `auto_lint` | Auto-lint after write/edit tool calls |
| `emotional_state` | Emotional state modifiers in personality prompt |
| `forge_tool` | Dynamic tool generation |
| `undo_tracking` | Undo history snapshots |

### Metrics

Per-call stats: model tier used, token counts, latency, tool names, critic verdict. Exportable as JSON via `telemetry.to_json()` or saved to a file via `telemetry.save_to_file()`. Useful for benchmarking with `oni run`.

---

## Unwired Infrastructure

Code exists but is not active in the current agent loop:

| Module | Location | Status |
|--------|----------|--------|
| `MessageBus` | `crates/oni-agent/src/message_bus.rs` | In-memory `VecDeque<BusMessage>`. Pub/sub for agent events (Discovery, Warning, TaskComplete, TaskFailed, FileChanged). Not connected to orchestrator or TUI. |
| `ExecutionTrace` | `crates/oni-agent/src/trace.rs` | `VecDeque<TraceEvent>` for recording execution steps. Not wired to agent loop. |
| Vector embeddings | `crates/oni-context/src/embeddings.rs` | `embed()` / `embed_batch()` via `nomic-embed-text`, `cosine_similarity()` implemented. Not used in the retrieval pipeline — retrieval uses FTS5 BM25 only. |

---

## Data Paths

| Data | Location |
|------|----------|
| Personality files | `~/.local/share/oni/` |
| Knowledge graph | `~/.local/share/oni/knowledge-graph.json` |
| Journal | `~/.local/share/oni/journal/YYYY-MM-DD.md` |
| Application log | `~/.local/share/oni/oni.log` |
| Global config | `~/.config/oni/oni.toml` |
| Project config | `./.oni/oni.toml` |
| Project index DB | `.oni/context.db` |
| Context pin | `.oni/pin` |
| Project context file | `.oni-context` |
| Persisted plans | Project directory (`.oni/plan-*.json`) |
