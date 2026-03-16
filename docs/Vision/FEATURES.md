# ONI — Feature Specifications

Complete specification for every ONI feature. Each section covers: behaviour, UX, implementation notes, and acceptance criteria.

---

## 1. Authentication (`oni login` / `oni logout`)

### Behaviour
- `oni login` initiates OAuth 2.0 PKCE flow
- Generates `code_verifier` (43–128 char random string) and `code_challenge` (SHA-256 hash, base64url encoded)
- Opens browser to `https://claude.ai/oauth?client_id=oni&redirect_uri=http://localhost:3841/callback&code_challenge=...&code_challenge_method=S256`
- Starts localhost HTTP server on port 3841
- Receives authorisation code in redirect
- Exchanges code + verifier for access/refresh token pair
- Stores tokens in OS keychain via `keytar` under service `oni-cli`
- Prints `Authenticated. Welcome.` — no verbose success ceremony

### Error handling
- Port 3841 in use → try 3842, 3843; fail with message if all busy
- Browser fails to open → print URL for manual visit
- Timeout after 120s → fail with `Login timed out. Run oni login to retry.`
- Invalid code → `Authentication failed. Run oni login.`

### `oni logout`
- Deletes tokens from keychain
- Optionally: `--all` clears all stored preferences and learned rules

### Acceptance criteria
- [ ] `oni login` completes in <5s on fast connection
- [ ] Tokens stored in OS keychain, not in any config file
- [ ] `oni logout` removes all tokens
- [ ] Re-running `oni login` when already authenticated: warns and re-auths

---

## 2. Inline Shell Trigger (`:` prefix)

### Behaviour
- ZSH: `preexec` hook intercepts commands starting with `:`
- Bash: `DEBUG` trap intercepts before execution
- Hook installed automatically by `oni login` to detected shell rc file
- Strips leading `:` and passes remainder to `oni ask`
- Passes implicit context: `$PWD`, `$?` (last exit code), `$HISTORY_LAST_COMMAND`
- Response streams to stdout inline
- Confirm-before-write prompt for any file mutations: `[y/n/diff]`

### Shell hook (ZSH)
```bash
# Written to ~/.zshrc by oni login
oni_preexec() {
  if [[ "$1" == :* ]]; then
    oni ask "${1:1}" --cwd "$PWD" --last-exit "$?" 
    return 1
  fi
}
autoload -Uz add-zsh-hook
add-zsh-hook preexec oni_preexec
```

### Behaviour notes
- Does NOT start a persistent session — each `:` invocation is stateless by default
- Use `--continue` flag to attach to last conv: `:fix this --continue`
- Tool calls rendered inline as `[tool] tool_name arg`
- No TUI — pure stdout/stderr

### Acceptance criteria
- [ ] `: hello` triggers ONI, not ZSH `null` command
- [ ] Hook survives `source ~/.zshrc`
- [ ] Works in non-interactive shells (scripts)
- [ ] `--continue` attaches to last active conv_id

---

## 3. Pipe / stdin (`oni ask`)

### Behaviour
- Reads stdin to EOF
- Appends user question as final message
- Streams response to stdout
- Supports `--json` flag for NDJSON event output
- Tool calls (read_file, etc.) still execute from piped invocations

### Usage patterns
```bash
npm test 2>&1 | oni ask "why is this failing"
git diff | oni ask "write a commit message"
cat error.log | oni ask "summarise"
docker build . 2>&1 | tail -50 | oni ask "what went wrong"
```

### `--json` output format
```jsonl
{"type":"thinking","content":"[Σ] Decomposing...","ts":1700000001}
{"type":"tool_call","tool":"read_file","args":{"path":"src/auth.ts"},"ts":1700000002}
{"type":"tool_result","tool":"read_file","latency_ms":11,"ts":1700000003}
{"type":"text","content":"Race condition in...","ts":1700000004}
{"type":"done","tokens_used":4201,"ts":1700000005}
```

### Acceptance criteria
- [ ] `echo "hello" | oni ask "what did I say"` works
- [ ] `--json` produces valid NDJSON on stdout
- [ ] Tool calls work in pipe mode (can read files, run bash)
- [ ] Ctrl+C cancels gracefully, no hanging processes

---

## 4. REPL (`oni chat`)

### Behaviour
- Full ink-rendered interactive REPL
- Persistent conversation — all turns share one conv_id
- Header: `ONI v{version} · {model} · {tokens_used}k tok · conv_{id}`
- Prompt: `you › ` (user) / `oni › ` (assistant)
- Sub-agent prefixes rendered in colour: `[Σ]` violet, `[⚡]` cyan, `[⊘]` coral

### Keybindings
| Key | Action |
|---|---|
| `Enter` | Send message |
| `Shift+Enter` | Newline (multiline input) |
| `\` + `Enter` | Continuation (alternative multiline) |
| `↑ / ↓` | History navigation |
| `Ctrl+R` | Fuzzy history search |
| `:q` | Exit REPL |
| `:mc` | Open Mission Control |
| `:diff` | Review pending file diffs |
| `:clear` | New session (new conv_id) |
| `:tools` | List available tools + plugin status |
| `:prefs` | Show learned preferences |
| `Ctrl+C` | Cancel current generation |

### Display
- Tool calls rendered inline: `[tool] tool_name arg … 12ms`
- File diffs shown inline with `+`/`-` colour coding
- Token count updated after each turn
- Burn rate shown if >1000 tok/min

### Acceptance criteria
- [ ] History persists between `oni chat` invocations
- [ ] `:mc` opens Mission Control without closing REPL
- [ ] Ctrl+C cancels streaming, does not kill process
- [ ] Works in 80-column terminal without wrapping artefacts

---

## 5. Mission Control (`oni mc`)

### Panels

**Stat bar (always visible):**
- Running tasks count (green if >0, white if 0)
- Cumulative token usage this session
- Current burn rate (tok/min) — amber if >2000, coral if >5000
- Total tool calls

**Task queue:**
- All tasks with status: `RUNNING` / `BLOCKED` / `ERROR` / `DONE`
- Blocked tasks show blocker reason inline
- `oni task kill <id>` — terminate running task
- `oni task retry <id>` — requeue failed task

**Tool call log:**
- Chronological monospace log: `HH:MM:SS  tool_name  arg  latency_ms`
- Scrollable (↑/↓ in Mission Control)
- Last 100 entries kept in view

**Active diff:**
- Live unified diff of file currently being written
- Updates as executor streams writes
- `oni diff accept` — apply all pending changes
- `oni diff reject` — discard all pending changes
- `oni diff accept <file>` — apply single file

**Sub-agent status:**
- Planner / Executor / Critic: `active` / `idle` / `reviewing`

**claude.ai sync:**
- Connection status: `LIVE` (green pulse) / `STALE` (amber) / `ERROR` (coral)
- Last sync timestamp
- Conv ID

**Context window:**
- Dual progress bar: absolute token count + burn rate
- Visual warning at 60% (amber) and 80% (coral)

### Acceptance criteria
- [ ] `oni mc` renders in <200ms
- [ ] All panels update in real time (SQLite poll every 500ms)
- [ ] Keyboard navigation between panels
- [ ] Works in 80×24 terminal minimum

---

## 6. Adaptive Tool Proposals + Preference Learning

### Proposal UX
When `score < 0.85`:
```
oni › I can use a few tools here. Proposing:
  [1] read_file   Dockerfile, docker-compose.yml
  [2] bash        docker build . --no-cache 2>&1 | tail -40
  [3] web_search  "docker layer cache CI" — optional

Use all? [enter] · pick [1/2/3] · skip [s] · always auto [a]
```

When `score ≥ 0.85`: tool runs silently with inline `[tool]` log.

### Scoring function
```
score(tool, intent) =
  base_prior[tool]
  × intent_match(tool, intent_vec)    // cosine sim, intent → tool
  × preference_weight(tool, intent)   // from SQLite preferences table
  × recency_decay(last_updated)       // 0.97^days_elapsed
```

Thresholds (configurable via `oni config`):
- `≥ 0.85` → auto-use
- `0.50 – 0.85` → propose
- `< 0.50` → omit

### Signal capture
| Response | Weight delta |
|---|---|
| Accept all (`enter`/`y`) | +1.0 each tool |
| Accept subset (`1,2`) | +1.0 selected, −1.0 skipped |
| Skip all (`s`) | −1.0 all proposed |
| Always auto (`a`) | weight → 1.0 permanently |
| Modify command | +0.5 partial, stores preferred command |

### Rule crystallisation
Background job runs on session end:
```sql
SELECT tool_name, intent_key, weight, n_obs FROM preferences
WHERE weight > 0.85 AND n_obs > 10
```
Promoted rules injected into system prompt as natural language instructions.

### `oni prefs` commands
```bash
oni prefs list                    # show all learned rules + confidence
oni prefs show <tool>             # detail for specific tool
oni prefs reset                   # wipe all preferences
oni prefs export > prefs.jsonl    # export for backup/transfer
oni prefs import prefs.jsonl      # import preferences
oni prefs forget web_search       # reset single tool preferences
```

### Acceptance criteria
- [ ] Auto-threshold correctly bypasses proposal at ≥0.85
- [ ] Accept/reject signals correctly update SQLite weights
- [ ] Decay applied on read (not write)
- [ ] Crystallised rules appear in system prompt next session
- [ ] `oni prefs reset` works cleanly

---

## 7. Context Engine

### Index initialisation
```bash
oni init    # full index of current directory
```
- Detects language per file (`.ts` → TypeScript grammar, `.py` → Python, etc.)
- tree-sitter parses each file → extracts: function names, class names, exports, imports, type defs
- Writes to SQLite: `symbols`, `import_edges`, `chunks_fts`
- Excludes: `node_modules`, `.git`, `dist`, `build`, patterns in `.oniignore`
- Progress shown in TUI
- 100k LOC ≈ 30s first index

### Incremental updates
- chokidar watches for file saves
- Debounced at 200ms
- Only changed file re-parsed
- Import edges re-computed for changed file

### Retrieval (per query)
1. Symbol lookup — FTS5 match on function/class names in query
2. Import graph traversal — expand from matched file (depth ≤ 3, configurable)
3. BM25 fulltext — ripgrep for unmatched patterns
4. Re-rank — `score = BM25 × exp(-elapsed_min/30) × graph_centrality`
5. Pack — top-N chunks up to token budget

### Scope control
```bash
oni pin src/auth/          # restrict retrieval to subtree
oni pin --reset            # remove pin
oni ignore dist/ vendor/   # add to .oniignore
```

### `oni index` commands
```bash
oni index stats            # files, symbols, edges, index size, staleness
oni index rebuild          # force full re-index
oni index watch            # start file watcher (if not already running)
```

### Acceptance criteria
- [ ] `oni init` completes on 100k LOC in <60s
- [ ] File change triggers re-index within 500ms
- [ ] Symbol lookup returns correct file within 5ms
- [ ] Token budget never exceeded in assembled context

---

## 8. claude.ai Sync

### Attach flow
```bash
oni sync abc123            # attach to conv ID from claude.ai URL
oni sync --latest          # attach to most recent claude.ai conversation
oni sync --detach          # stop syncing, go local-only
```

### Sync daemon
- Spawned by `oni login`, runs as background process
- Registered with launchd (macOS) or systemd --user (Linux)
- Polls `GET /api/organizations/:org/chat_conversations/:conv_id` every 2s when active
- Writes new messages to SQLite with `origin='web'`
- On ONI terminal turn: POSTs assistant message back to conversation
- Status visible in Mission Control sync panel

### Conflict resolution
- `msg_id` deduplication prevents double-insert
- Simultaneous writes (race condition): last-write-wins
- Timestamp ordering preserved

### Failure modes and degradation
| Failure | Behaviour |
|---|---|
| API endpoint breaks | Local-only mode. Sync panel shows `ERROR`. |
| Daemon crash | Auto-restart. Log to `~/.local/share/oni/sync.log`. |
| Token expiry | Re-auth flow triggered. |
| Network offline | Sync paused. Resumes on reconnect. |

### Acceptance criteria
- [ ] `oni sync <id>` attaches in <2s
- [ ] Web messages appear in REPL within 3s of being sent
- [ ] Terminal messages appear on claude.ai within 5s
- [ ] Sync panel accurately reflects connection state

---

## 9. Background Agents (`oni run`)

### Usage
```bash
oni run "add unit tests to all service files"              # background task
oni run "migrate all fetch() calls to ky" --glob "src/**/*.ts"   # sweep
oni run --list             # list all background tasks
oni run --kill <id>        # terminate task
oni run --logs <id>        # stream task logs
```

### Behaviour
- Spawns worker process via `child_process.fork()`
- Worker writes progress to SQLite `tasks` table
- Mission Control shows all running tasks
- On completion: terminal notification (via `notify-send` / macOS notifications)
- On error: task marked `ERROR` in queue, logs preserved

### `oni sweep` (codebase-wide operations)
- Sequential execution (one file at a time)
- Checkpointed — resume after crash
- Progress bar showing files processed / remaining
- Dry-run by default (`--write` to apply)

```bash
oni sweep "add JSDoc to all exported functions" --glob "src/**/*.ts" --dry-run
oni sweep "add JSDoc to all exported functions" --glob "src/**/*.ts" --write
```

### Acceptance criteria
- [ ] `oni run` returns immediately, task runs in background
- [ ] Progress visible in Mission Control
- [ ] Task survives terminal close (daemon continues)
- [ ] `oni sweep --write` only modifies matched files
- [ ] Checkpoints allow resume after crash

---

## 10. CI Self-Healing (`oni watch-ci`)

### Usage
```bash
oni watch-ci                           # watch current repo, current branch
oni watch-ci --repo owner/repo        # explicit repo
oni watch-ci --branch feature/auth    # specific branch
```

### Behaviour
1. Polls GitHub Actions / GitLab CI via API every 30s
2. On failure: fetches logs via `gh run view --log-failed`
3. Σ Planner analyses failure, identifies root cause
4. Proposes fix: `Fix and re-push? [y/diff/n]`
5. If accepted: writes fix, commits, pushes, triggers re-run
6. Waits for new run, reports result

### Requirements
- GitHub: `gh` CLI must be installed and authenticated
- GitLab: `glab` CLI or `GITLAB_TOKEN` env var
- ONI only touches files directly implicated in the failure

### Acceptance criteria
- [ ] Correctly identifies common failure types (missing mock, wrong import, version mismatch)
- [ ] Never commits to protected branches without explicit confirmation
- [ ] `[n]` at prompt does nothing — no writes, no push

---

## 11. MCP Plugin System

### Install
```bash
oni plugin add github              # from ONI registry
oni plugin add https://mcp.co/sse  # arbitrary SSE server
oni plugin add ./local-mcp/        # local stdio server
```

### Lifecycle
1. Fetch manifest from registry or URL
2. Show: tools, scopes, auth required — `Install? [y/n]`
3. Write to `.oni/plugins.json`
4. Connect MCP server, run `tools/list`
5. Merge tools into broker
6. Available on next turn

### Built-in registry
| Plugin | Transport | Key tools |
|---|---|---|
| `github` | stdio | `create_pr`, `list_issues`, `get_run_logs`, `merge_pr` |
| `linear` | SSE | `get_issue`, `update_status`, `create_issue` |
| `postgres` | stdio | `query`, `describe_table`, `explain` |
| `sentry` | SSE | `get_error`, `get_stacktrace`, `list_issues` |
| `npm` | stdio | `search`, `check_vulns`, `latest_version` |
| `docker` | stdio | `list_containers`, `get_logs`, `exec` |
| `jira` | SSE | `get_ticket`, `update_status`, `create_ticket` |
| `slack` | SSE | `send_message`, `get_thread` |

### `oni plugin` commands
```bash
oni plugin list                    # all installed plugins + status
oni plugin enable <name>          # enable disabled plugin
oni plugin disable <name>         # disable without removing
oni plugin rm <name>              # remove plugin
oni plugin update <name>          # update to latest registry version
oni plugin auth <name>            # re-run auth flow for plugin
```

### Acceptance criteria
- [ ] `oni plugin add github` completes in <10s
- [ ] Plugin tools appear in next ONI turn after install
- [ ] `auto_surface: true` plugins appear in tool proposals
- [ ] Plugin crash does not crash ONI — degrades gracefully
- [ ] `.oni/plugins.json` is committable (no secrets in file)

---

## 12. PR Creation (`oni pr`)

### Behaviour
After task completion, ONI optionally offers:
```
Create PR? [y/n]
```
If `y`, or `oni pr` run manually:
1. Creates branch: `oni/<task-slug>-<short-id>`
2. `git add -A && git commit -m "<ONI-generated message>"`
3. `git push -u origin <branch>`
4. `gh pr create --title "..." --body "..."` (requires `gh` CLI)
5. Prints PR URL

### Generated PR description
- Summary of what was changed
- Files modified list
- Test results if available
- Link to ONI session conv_id

### Acceptance criteria
- [ ] `oni pr` only runs after file changes exist
- [ ] Branch name is URL-safe
- [ ] PR body includes file list and summary
- [ ] Fails gracefully if `gh` not installed
