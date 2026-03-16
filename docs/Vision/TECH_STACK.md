# ONI — Tech Stack

Every decision here has a rationale. If you want to change something, read the rationale first.

---

## Runtime

| Choice | Rationale |
|---|---|
| **TypeScript (strict)** | Type safety critical for agent state machines. Strict mode catches the errors that matter. |
| **Node.js 20+ LTS** | Stable, long-support, native ESM, `fetch` built-in. No Bun/Deno — ecosystem compatibility required. |
| **tsx** | Zero-config TypeScript execution for development. No transpile step in dev loop. |
| **esbuild** (via tsup) | Fast bundling for distribution. Sub-second build times. |

**Why not Rust/Go?** The bottleneck in an LLM agent is network latency, not process speed. TypeScript gives us ink (React TUI), faster iteration, and access to the Node.js ecosystem. Revisit if spawning or IPC becomes a bottleneck in v3+.

---

## CLI Framework

| Layer | Package | Version | Purpose |
|---|---|---|---|
| TUI rendering | `ink` | ^5.0 | React for terminals. Mission Control dashboard built as React components. |
| Command routing | `commander.js` | ^12.0 | Subcommand parsing, help generation, version flag. |
| Interactive prompts | `@inquirer/prompts` | ^7.0 | Confirm dialogs, selection menus (y/n/diff prompts). |
| Colours | `chalk` | ^5.0 | Terminal colour output in non-ink contexts (pipe mode). |
| Hyperlinks | `terminal-link` | ^3.0 | Clickable links in supported terminals. |

**Why ink over blessed/charm?** ink lets Mission Control be built as stateful React components with `useState`/`useEffect`. The mental model matches the complexity. Blessed requires imperative terminal drawing — harder to maintain.

---

## Authentication

> **Updated March 2026:** Anthropic banned third-party OAuth in Feb 2026. API key auth with keytar provides identical security.

| Component | Choice | Rationale |
|---|---|---|
| Primary auth | API key | Standard Anthropic API key from platform.anthropic.com. Only supported third-party auth method. |
| Secondary auth | Claude Code passthrough | Reads existing Claude Code credentials. Personal use only — same developer, same machine. |
| Token storage | `keytar` | OS keychain integration: macOS Keychain, libsecret (Linux), DPAPI (Windows). Keys never in plaintext. |
| Budget control | Local enforcement | `--budget` (session) and `--monthly-limit` flags. Anthropic API has no server-side spending cap. |

**Auth flow:**
1. `oni login --key sk-ant-...` or interactive prompt
2. Validates key against Anthropic API
3. Stores in OS keychain via keytar
4. Resolved on each session: env var → keychain → Claude Code credentials

---

## AI / API Layer

| Component | Choice | Notes |
|---|---|---|
| SDK | `@anthropic-ai/sdk` | Official SDK. Streaming, tool use, error types. |
| Model | `claude-sonnet-4-6` | Balanced cost/capability for default. Configurable. |
| Transport | Streaming SSE | All responses streamed. No blocking calls. |
| Tool calling | Anthropic tool_use | Native function calling. Tool schemas defined in Zod, converted to JSON Schema. |
| Context limit | 200k tokens | Sonnet context window. Budget managed explicitly — see ARCHITECTURE.md. |

**Model override:**
```bash
oni config set model claude-opus-4-6   # for complex planning tasks
oni config set model claude-haiku-4-5  # for fast, cheap tasks
```

---

## Agent Core

| Component | Choice | Rationale |
|---|---|---|
| State machine | `langgraph` (JS port) | Explicit graph of Planner→Executor→Critic states. Conditional edges for Critic veto loop. Debuggable, serialisable. |
| Schema validation | `zod` | All tool inputs/outputs validated. Provides JSON Schema for Claude tool definitions. |
| Event bus | `eventemitter3` | Decoupled communication between agent core, TUI, and sync daemon. |
| Process management | Node.js `child_process` | Bash tool spawning. Timeout enforcement. Output streaming. |

**Agent state machine:**
```
idle → planning → executing → reviewing → done
                              reviewing → planning  (critic rejects, max 2 replans)
               executing → blocked  (escalate to user)
```

---

## Context Engine

| Component | Choice | Purpose |
|---|---|---|
| AST parsing | `tree-sitter` + `node-tree-sitter` | Symbol extraction for TS, JS, Python, Rust, Go, C/C++, Java. |
| Full-text search | `ripgrep` (via `execa`) | Sub-100ms search on 500k LOC. Fallback for dynamic patterns. |
| File watching | `chokidar` | Incremental re-index on file save. Debounced at 200ms. |
| Index storage | SQLite FTS5 | Full-text search on symbol/chunk index. Integrated with preference DB. |
| Ranking | BM25 (custom) | Symbol lookup + BM25 + recency decay. No vector embeddings in v1. |

**No embeddings in v1.** SQLite FTS5 + AST symbol lookup covers 90% of retrieval quality. Vector search (sqlite-vss or LanceDB) planned for v2 if users hit quality ceiling.

---

## Persistence

| Store | Tech | Contents |
|---|---|---|
| Main DB | `better-sqlite3` | Conversations, messages, preferences, tool events, learned rules, plugin registry, sync log |
| Index DB | SQLite FTS5 | File symbols, chunks, import edges (separate DB for index isolation) |
| Config | XDG config dir | `~/.config/oni/config.json` — model, thresholds, global settings |
| Session logs | JSONL | Append-only log of all sessions. Never truncated. Rotated at 100MB. |
| Secrets | OS keychain | OAuth tokens, plugin auth tokens. Via keytar. |

**DB location:** `~/.local/share/oni/oni.db` (XDG data dir)

**better-sqlite3 is synchronous** — correct for CLI. No async overhead, no connection pool needed, simpler error handling.

---

## Sync / Network

| Component | Choice | Purpose |
|---|---|---|
| HTTP client | `undici` | HTTP/2, native fetch-compatible, faster than axios for streaming |
| WebSocket | `ws` | claude.ai sync WebSocket connection |
| EventSource | `eventsource` | SSE fallback for sync polling |
| Polling interval | 2s (active), 30s (idle) | Active = conversation in progress. Idle = no recent turn. |

---

## MCP Plugin System

| Component | Choice | Purpose |
|---|---|---|
| MCP client | `@modelcontextprotocol/sdk` | Official MCP client. Handles stdio + SSE transports. |
| Transport: stdio | `child_process.spawn` | Local MCP servers. ONI spawns, communicates over stdin/stdout. |
| Transport: SSE | `eventsource` | Remote MCP servers. Auth via Bearer token in headers. |
| Tool broker | Custom | Merges built-in + plugin tools. Namespaces collisions as `plugin:tool`. |

---

## Distribution

| Channel | Method | Target |
|---|---|---|
| npm | `npm install -g oni-cli` | Primary. Node.js users. |
| Homebrew | Custom tap | macOS users without Node.js. |
| curl installer | Shell script | Linux. Downloads pre-built binary. |
| Standalone binary | `pkg` or `oclif` | Single-file executable. No Node.js required. |

---

## Tooling

| Tool | Purpose |
|---|---|
| `biome` | Linting + formatting. Replaces ESLint + Prettier. Faster. |
| `vitest` | Unit and integration tests. |
| `tsup` | Build/bundle. Outputs CJS + ESM. |
| `changesets` | Changelog and versioning. |
| `husky` | Pre-commit hooks (lint, type-check). |

---

## Dependency Philosophy

- **Zero runtime dependencies** where stdlib suffices
- **Official SDKs** over third-party wrappers (Anthropic SDK, MCP SDK)
- **battle-tested** over cutting-edge for storage and CLI primitives
- **Single DB engine** (SQLite) for all persistence — no Redis, no Postgres, no network DB
- Audit `package.json` on every PR. Justify every new dependency.
