# ONI — System Architecture

---

## High-Level Module Map

```
oni/
├── packages/
│   ├── cli/               # Commander.js entry, subcommand routing
│   ├── agent/             # LangGraph state machine, sub-agents, tool broker
│   ├── context/           # tree-sitter indexer, ripgrep, retrieval pipeline
│   ├── auth/              # OAuth PKCE, keytar, token refresh
│   ├── sync/              # claude.ai WebSocket sync daemon
│   ├── tui/               # ink components (Mission Control, REPL, diffs)
│   ├── plugins/           # MCP client, plugin registry, plugin manager
│   ├── prefs/             # Preference learning, tool events, rule engine
│   └── db/                # SQLite schemas, migrations, query helpers
├── scripts/               # Dev tooling, installers
└── docs/                  # This directory
```

---

## Data Flow — Single Turn

```
User input
    │
    ▼
CLI layer (commander.js)
    │  parses subcommand, mode (prefix / pipe / REPL)
    ▼
Context Engine
    │  resolves intent → retrieves relevant chunks → packs context
    ▼
Preference Engine
    │  loads active learned rules → injects into system prompt
    │  scores available tools → decides propose vs auto-use
    ▼
Agent Core (LangGraph)
    │
    ├── [Σ Planner node]
    │     decomposes mission, sets tool budget, asks ≤1 clarifying question
    │
    ├── [⚡ Executor node]
    │     calls Claude API (streaming SSE)
    │     executes tool calls (read_file / write_file / bash / plugins)
    │     streams output to TUI
    │
    └── [⊘ Critic node]
          reviews output against original intent
          verdict: accept → DONE
          verdict: reject → back to Planner (max 2 replans)
    │
    ▼
Feedback capture
    │  records tool_events (outcome, latency, user response)
    │  updates preference weights in SQLite
    ▼
TUI / stdout
    │  ink (REPL / Mission Control) or plain stdout (pipe mode)
    ▼
Sync daemon (if active)
       pushes assistant turn back to claude.ai conversation
```

---

## Agent State Machine

```typescript
type AgentState =
  | 'idle'        // waiting for user input
  | 'planning'    // Σ Planner active
  | 'executing'   // ⚡ Executor active, tools running
  | 'reviewing'   // ⊘ Critic active
  | 'blocked'     // executor hit unexpected blocker, escalating to user
  | 'done'        // task complete, awaiting next input

interface ONIState {
  mission:      string           // original user request
  plan:         string[]         // ordered subtasks from Planner
  toolBudget:   number           // max tool calls this turn
  toolsUsed:    ToolCall[]       // completed tool calls
  output:       string           // accumulated executor output
  replanCount:  number           // Critic→Planner cycles (max 2)
  criticVerdict: 'accept' | 'reject' | null
  blocker:      string | null    // escalation reason
}
```

**Transition guards:**
- `executing → reviewing`: only when executor signals completion (no blocker)
- `reviewing → planning`: only when `replanCount < 2`; else → `blocked`
- `blocked → idle`: user provides input to resolve blocker

---

## Context Engine Pipeline

```
Query arrives
    │
    ├── 1. Symbol lookup (SQLite FTS5)
    │       query → known symbol names → exact AST match
    │       sub-5ms, highest priority
    │
    ├── 2. Import graph traversal
    │       expand from matched file → transitive deps (depth ≤ 3)
    │       captures callers + callees without user specifying
    │
    ├── 3. BM25 fulltext search (ripgrep)
    │       for patterns not caught by symbol lookup
    │       strings, comments, config keys, dynamic references
    │
    ├── 4. Re-rank
    │       score = BM25_score × recency_weight × graph_centrality
    │       recency: exp(-elapsed_minutes / 30)
    │       dedup overlapping chunks
    │
    └── 5. Context pack
            assemble: system_prompt + CLAUDE.md + top-N chunks + active_diff + git_blame
            enforce token budget (default: 80k tokens for retrieved code)
```

**Index update flow (incremental):**
```
chokidar file-change event
    │ debounce 200ms
    ▼
tree-sitter parse changed file
    │
    ├── update symbols table
    ├── update import_edges table
    ├── update chunks_fts (FTS5)
    └── update files.last_indexed
```

---

## Context Window Budget (200k tokens)

| Slot | Allocation | Notes |
|---|---|---|
| System prompt + rules | ~10k (5%) | ONI persona + active learned rules |
| CLAUDE.md + project context | ~8k (4%) | Per-project context file |
| Retrieved code chunks | ~80k (40%) | Top-N ranked chunks from context engine |
| Conversation history | ~50k (25%) | Compacted after 60% budget consumed |
| Active diff + tool output | ~30k (15%) | Current file changes, bash output |
| Reserved (response) | ~20k (10%) | Headroom for Claude's response |

**Compaction trigger:** when `history_tokens > 0.6 × budget`, Critic summarises old turns into a digest. Raw history pruned, digest retained. Never a hard reset.

---

## Preference Learning Pipeline

```
Tool proposed to user
    │
    ├── user accepts all       → +1.0 to each proposed tool / intent pair
    ├── user accepts subset    → +1.0 accepted, -1.0 rejected
    ├── user skips all (s)     → -1.0 all proposed tools for this intent
    ├── user sets always (a)   → weight → 1.0, bypass proposal forever
    └── user modifies command  → +0.5 partial accept, captures preferred command pattern
    │
    ▼
tool_events table
    │  records: tool_name, intent_vec, outcome, response, ts
    ▼
preferences table
    │  weight = rolling average of outcomes × recency_decay
    │  decay: weight *= 0.97^days_since_last_obs (applied on read)
    ▼
Rule crystallisation (background job, on session end)
    │  SELECT tools where confidence > 0.85 AND n_obs > 10
    │  Promote to learned_rules table
    │  Inject into system prompt on next session
    ▼
learned_rules → system prompt injection
    "When debugging TypeScript, run tsc --noEmit before reading files."
    "User skips web_search during debugging sessions."
```

---

## Sync Architecture

**Mechanism:** Poll claude.ai internal conversation API using OAuth token. Write messages to local SQLite. Push terminal turns back to claude.ai via PATCH.

```
claude.ai conversation
    │  GET /api/organizations/:org/chat_conversations/:conv_id
    │  poll every 2s (active) / 30s (idle)
    ▼
Sync daemon (background process)
    │  diffs new messages against local SQLite
    │  writes new messages with origin='web'
    ▼
Local SQLite (conversations, messages)
    │
    ├── ONI terminal turn arrives
    │     Claude API call with conv_id context
    │     response written to SQLite with origin='terminal'
    │
    └── PATCH back to claude.ai
          POST assistant turn to conversation endpoint
          appears in claude.ai UI as normal message
```

**Conflict resolution:** last-write-wins per message. If both web and terminal write simultaneously (different turns), both are appended in timestamp order. If the same turn is written from both (race condition), `msg_id` deduplication prevents double-insert.

**Failure modes:**
- Sync daemon crash → restarts automatically (launchd/systemd unit)
- claude.ai API breaks → graceful degradation to local-only mode; `oni mc` shows sync status as STALE
- Token expiry → triggers re-auth flow

---

## MCP Plugin Architecture

```
Plugin manifest (.oni/plugins.json)
    │  name, source, transport, enabled, tools[], auth
    ▼
Plugin Manager (oni plugin add/rm/enable/disable)
    │
    ├── stdio transport
    │     spawn local process → communicate over stdin/stdout
    │     zero network, sandboxed
    │
    └── SSE transport
          connect as EventSource client
          auth: Bearer token from keytar
    │
    ▼
MCP Client (tools/list call on connect)
    │  discovers available tools from server
    ▼
Tool Broker
    │  merges: built-ins + all plugin tools
    │  namespaces collisions: "create_pr" from github → "github:create_pr"
    ▼
Claude API call (tools: broker.allTools())
    │  Claude picks tools agnostically
    ▼
Tool execution
    │  ONI calls MCP server with tool name + arguments
    │  returns result to Claude context
```

---

## Process Model

```
oni chat (foreground)
├── ink TUI process (main)
├── Agent core (in-process)
├── Context engine (in-process)
└── Sync daemon (background, separate process)
    └── Communicates via SQLite (not IPC)

oni run --background (async tasks)
├── Worker process spawned via child_process.fork()
├── Reports progress to SQLite task queue
└── Mission Control polls SQLite for status
```

**Why SQLite for IPC?** Avoids Unix socket lifecycle management. Simpler crash recovery — worker state is fully in SQLite, not in-memory. Mission Control just reads the DB.

---

## Error Handling Strategy

| Error class | Handling |
|---|---|
| Network timeout (API) | Retry ×3 with exponential backoff (1s, 2s, 4s). Fail to `blocked` state after. |
| Tool execution failure | Executor reports failure inline. Critic evaluates whether to retry or surface. |
| Context window exceeded | Compaction triggered. If compaction insufficient, surface to user with summary. |
| Auth token expired | Silent refresh. If refresh fails, prompt `oni login`. |
| Sync daemon crash | Auto-restart via launchd/systemd. Log to `~/.local/share/oni/sync.log`. |
| MCP server crash | Plugin marked `status=error` in DB. ONI continues without plugin tools. |
| Critic veto loop | After 2 replans, surface to user: "Critic rejected twice. Here's the problem." |

---

## SQLite Schema Summary

See `API_CONTRACTS.md` for full column definitions.

**Tables:**
- `conversations` — active and archived conv sessions
- `messages` — all turns (user + assistant), with origin tag
- `tool_events` — every tool call with outcome and timing
- `preferences` — per-tool per-intent preference weights
- `learned_rules` — crystallised rules injected into system prompt
- `installed_plugins` — plugin registry with auth config
- `plugin_tools` — tools exposed by each plugin
- `sync_log` — sync daemon activity log
- `tasks` — async background task queue

**Index DB (separate file):**
- `files` — indexed files with language and timestamps
- `symbols` — function/class/export definitions
- `import_edges` — directed import graph
- `chunks_fts` — FTS5 virtual table for text search
