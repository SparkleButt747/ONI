# ONI — API Contracts

Internal interfaces, message formats, and SQLite schemas. Any module boundary must be documented here.

---

## SQLite Schemas

### Main DB: `~/.local/share/oni/oni.db`

#### `conversations`
```sql
CREATE TABLE conversations (
  conv_id       TEXT PRIMARY KEY,           -- UUID or claude.ai conv ID
  source        TEXT NOT NULL,              -- 'local' | 'claude_ai'
  created_at    INTEGER NOT NULL,           -- unix timestamp ms
  last_active   INTEGER NOT NULL,
  last_sync     INTEGER,                    -- null if local-only
  sync_cursor   TEXT,                       -- pagination cursor for sync API
  sync_status   TEXT DEFAULT 'local',       -- 'live' | 'stale' | 'error' | 'local'
  project_dir   TEXT                        -- cwd at session start
);
```

#### `messages`
```sql
CREATE TABLE messages (
  msg_id        TEXT PRIMARY KEY,           -- UUID
  conv_id       TEXT NOT NULL REFERENCES conversations(conv_id),
  role          TEXT NOT NULL,              -- 'user' | 'assistant'
  content       TEXT NOT NULL,             -- full message content
  origin        TEXT NOT NULL,             -- 'terminal' | 'web'
  ts            INTEGER NOT NULL,          -- unix timestamp ms
  tokens        INTEGER,                   -- token count if known
  sub_agent     TEXT                       -- 'planner' | 'executor' | 'critic' | null
);
CREATE INDEX messages_conv ON messages(conv_id, ts);
```

#### `tool_events`
```sql
CREATE TABLE tool_events (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id    TEXT NOT NULL,             -- conv_id of session
  tool_name     TEXT NOT NULL,             -- e.g. 'bash', 'github:create_pr'
  plugin        TEXT,                      -- null for built-ins
  intent_key    TEXT NOT NULL,             -- hashed intent vector
  args_hash     TEXT,                      -- hash of tool arguments
  outcome       TEXT NOT NULL,             -- 'accepted' | 'rejected' | 'modified' | 'auto'
  response      TEXT,                      -- what user typed (if modified)
  latency_ms    INTEGER,                   -- tool execution time
  ts            INTEGER NOT NULL
);
CREATE INDEX tool_events_tool ON tool_events(tool_name, intent_key);
```

#### `preferences`
```sql
CREATE TABLE preferences (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  tool_name     TEXT NOT NULL,
  intent_key    TEXT NOT NULL,             -- hashed intent context
  weight        REAL NOT NULL DEFAULT 0.5, -- 0.0 → 1.0
  n_obs         INTEGER NOT NULL DEFAULT 0,
  last_updated  INTEGER NOT NULL,         -- unix timestamp ms
  UNIQUE(tool_name, intent_key)
);
```

#### `learned_rules`
```sql
CREATE TABLE learned_rules (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  condition_json TEXT NOT NULL,            -- JSON: { intent, context, tool }
  action         TEXT NOT NULL,            -- natural language rule for system prompt
  confidence     REAL NOT NULL,            -- 0.0 → 1.0
  n_fired        INTEGER NOT NULL DEFAULT 0,
  n_accepted     INTEGER NOT NULL DEFAULT 0,
  active         INTEGER NOT NULL DEFAULT 1,  -- boolean
  created_at     INTEGER NOT NULL,
  last_fired     INTEGER
);
```

#### `tasks` (background agent queue)
```sql
CREATE TABLE tasks (
  id            TEXT PRIMARY KEY,          -- UUID
  mission       TEXT NOT NULL,             -- user's original request
  status        TEXT NOT NULL DEFAULT 'queued', -- 'queued'|'running'|'blocked'|'done'|'error'
  created_at    INTEGER NOT NULL,
  started_at    INTEGER,
  completed_at  INTEGER,
  error_msg     TEXT,
  log_path      TEXT,                      -- path to JSONL log file
  pid           INTEGER,                   -- worker process PID
  conv_id       TEXT                       -- associated conversation
);
```

#### `installed_plugins`
```sql
CREATE TABLE installed_plugins (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  name          TEXT UNIQUE NOT NULL,
  source        TEXT NOT NULL,             -- 'oni:github' | 'https://...' | './path'
  transport     TEXT NOT NULL,             -- 'stdio' | 'sse' | 'ws'
  enabled       INTEGER NOT NULL DEFAULT 1,
  scope         TEXT NOT NULL DEFAULT 'global', -- 'global' | 'project'
  auto_surface  INTEGER NOT NULL DEFAULT 1,
  installed_at  INTEGER NOT NULL
);
```

#### `plugin_tools`
```sql
CREATE TABLE plugin_tools (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  plugin_id     INTEGER NOT NULL REFERENCES installed_plugins(id),
  tool_name     TEXT NOT NULL,             -- as declared by MCP server
  namespaced    TEXT NOT NULL,             -- 'plugin:tool_name'
  description   TEXT NOT NULL,
  input_schema  TEXT NOT NULL,            -- JSON Schema string
  auto_surface  INTEGER NOT NULL DEFAULT 1
);
```

#### `plugin_auth`
```sql
CREATE TABLE plugin_auth (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  plugin_id     INTEGER NOT NULL REFERENCES installed_plugins(id),
  auth_type     TEXT NOT NULL,            -- 'env_var' | 'keychain' | 'oauth'
  env_var       TEXT,                      -- e.g. 'LINEAR_API_KEY'
  keychain_key  TEXT,                      -- keytar service key
  oauth_client  TEXT                       -- OAuth client ID for plugin OAuth flows
);
```

#### `sync_log`
```sql
CREATE TABLE sync_log (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  conv_id       TEXT NOT NULL,
  direction     TEXT NOT NULL,            -- 'pull' | 'push'
  msg_count     INTEGER NOT NULL DEFAULT 0,
  lag_ms        INTEGER,
  error         TEXT,
  ts            INTEGER NOT NULL
);
```

---

### Index DB: `~/.local/share/oni/index.db`

#### `files`
```sql
CREATE TABLE files (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  path          TEXT UNIQUE NOT NULL,     -- absolute path
  lang          TEXT NOT NULL,            -- 'typescript' | 'python' | ...
  last_indexed  INTEGER NOT NULL,         -- unix timestamp ms
  last_edited   INTEGER,                  -- mtime from fs
  token_count   INTEGER,                  -- estimated tokens
  hash          TEXT NOT NULL             -- file content hash (change detection)
);
```

#### `symbols`
```sql
CREATE TABLE symbols (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  name          TEXT NOT NULL,
  kind          TEXT NOT NULL,            -- 'function'|'class'|'export'|'type'|'interface'
  file_id       INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  start_line    INTEGER NOT NULL,
  end_line      INTEGER NOT NULL,
  exported      INTEGER NOT NULL DEFAULT 0,
  signature     TEXT                      -- function signature or type definition
);
CREATE INDEX symbols_name ON symbols(name);
CREATE INDEX symbols_file ON symbols(file_id);
```

#### `import_edges`
```sql
CREATE TABLE import_edges (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  from_file     INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  to_file       INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
  specifiers    TEXT NOT NULL             -- JSON array: ['AuthService', 'TokenType']
);
CREATE INDEX import_edges_from ON import_edges(from_file);
CREATE INDEX import_edges_to ON import_edges(to_file);
```

#### `chunks_fts` (FTS5 virtual table)
```sql
CREATE VIRTUAL TABLE chunks_fts USING fts5(
  content,
  file_id UNINDEXED,
  start_line UNINDEXED,
  end_line UNINDEXED,
  symbol_name UNINDEXED,
  tokenize = 'unicode61'
);
```

---

## NDJSON Event Stream Format (`--json` flag)

All events are newline-delimited JSON. Consumers should handle unknown `type` values gracefully.

```typescript
type ONIEvent =
  | { type: 'session_start'; conv_id: string; model: string; ts: number }
  | { type: 'thinking'; agent: 'planner'|'executor'|'critic'; content: string; ts: number }
  | { type: 'tool_call'; tool: string; args: Record<string, unknown>; ts: number }
  | { type: 'tool_result'; tool: string; success: boolean; latency_ms: number; ts: number }
  | { type: 'text'; content: string; ts: number }
  | { type: 'proposal'; tools: ProposedTool[]; ts: number }
  | { type: 'proposal_response'; accepted: string[]; rejected: string[]; ts: number }
  | { type: 'critic_verdict'; verdict: 'accept'|'reject'; reason: string; ts: number }
  | { type: 'blocked'; reason: string; ts: number }
  | { type: 'done'; tokens_used: number; duration_ms: number; ts: number }
  | { type: 'error'; message: string; code: string; ts: number }

interface ProposedTool {
  index: number
  tool: string
  rationale: string
}
```

---

## Internal Module Interfaces

### ContextEngine
```typescript
interface ContextEngine {
  init(dir: string): Promise<void>
  update(file: string): Promise<void>
  query(input: string, budget: number): Promise<ContextPack>
  stats(): Promise<IndexStats>
}

interface ContextPack {
  chunks: CodeChunk[]
  totalTokens: number
  retrievalMs: number
}

interface CodeChunk {
  file: string
  startLine: number
  endLine: number
  content: string
  tokens: number
  score: number
  symbol?: string
}

interface IndexStats {
  filesIndexed: number
  symbolCount: number
  edgeCount: number
  indexSizeBytes: number
  lastFullIndex: number
  staleFiles: number
}
```

### PreferenceEngine
```typescript
interface PreferenceEngine {
  score(tool: string, intent: string): number
  record(event: ToolEvent): Promise<void>
  crystallise(): Promise<LearnedRule[]>
  activeRules(): LearnedRule[]
  reset(tool?: string): Promise<void>
  export(): Promise<PreferenceExport>
  import(data: PreferenceExport): Promise<void>
}

interface ToolEvent {
  tool: string
  intent: string
  outcome: 'accepted' | 'rejected' | 'modified' | 'auto'
  response?: string
  latencyMs?: number
}

interface LearnedRule {
  condition: { intent: string; context?: string; tool?: string }
  action: string
  confidence: number
  nObs: number
}
```

### ToolBroker
```typescript
interface ToolBroker {
  loadPlugin(plugin: InstalledPlugin): Promise<void>
  unloadPlugin(name: string): void
  allTools(): ClaudeToolDefinition[]
  execute(toolName: string, args: unknown): Promise<ToolResult>
}

interface ToolResult {
  success: boolean
  output: string
  latencyMs: number
  error?: string
}
```

### SyncDaemon
```typescript
interface SyncDaemon {
  attach(convId: string): Promise<void>
  detach(): void
  status(): SyncStatus
  pushTurn(message: Message): Promise<void>
}

type SyncStatus = 'live' | 'stale' | 'error' | 'detached'
```

### AgentCore (LangGraph)
```typescript
interface AgentCore {
  run(mission: string, options: RunOptions): Promise<AgentResult>
  stream(mission: string, options: RunOptions): AsyncIterable<ONIEvent>
  cancel(): void
}

interface RunOptions {
  cwd: string
  convId?: string
  toolBudget?: number
  dryRun?: boolean
  allowWrite?: boolean
  allowExec?: boolean
  verbosity?: 'silent' | 'normal' | 'verbose'
}

interface AgentResult {
  output: string
  toolCalls: ToolCall[]
  tokensUsed: number
  durationMs: number
  verdict: 'done' | 'blocked' | 'cancelled'
}
```

---

## Config Schema

`~/.config/oni/config.json`

```typescript
interface OniConfig {
  model: string                        // default: 'claude-sonnet-4-6'
  contextBudget: number               // default: 80000 tokens for retrieved code
  autoThreshold: number               // default: 0.85
  proposeThreshold: number            // default: 0.50
  maxReplans: number                  // default: 2
  verbosity: 'silent' | 'normal' | 'verbose'  // default: 'normal'
  syncEnabled: boolean                // default: true
  syncPollInterval: number            // default: 2000ms
  dryRunDefault: boolean              // default: true
  colors: boolean                     // default: true (auto-detect terminal support)
  notifications: boolean              // default: true
}
```

`.oni/config.json` (per-project override, same schema, partial):
```json
{
  "contextBudget": 40000,
  "model": "claude-opus-4-6"
}
```

---

## Plugin Manifest Schema

`.oni/plugins.json`

```typescript
interface PluginsManifest {
  version: '1'
  plugins: PluginEntry[]
}

interface PluginEntry {
  name: string
  source: string                      // 'oni:github' | 'https://...' | './path'
  transport: 'stdio' | 'sse' | 'ws'
  enabled: boolean
  tools: string[] | '*'              // '*' = all tools from server
  autoSurface: boolean               // include in tool proposals
  auth?: {
    type: 'env_var' | 'keychain' | 'oauth'
    envVar?: string                   // for env_var type
    keychainKey?: string              // for keychain type
  }
}
```
