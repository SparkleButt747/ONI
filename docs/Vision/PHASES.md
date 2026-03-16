# ONI — Build Phases

Each phase has clear exit criteria. Do not start the next phase until all exit criteria for the current phase are met.

---

## Phase 1 — Foundation (Weeks 1–4)

**Goal:** A working CLI agent with OAuth auth, basic tool use, and streaming chat. No fancy features — just the core loop working reliably.

### Deliverables

- [ ] `oni login` / `oni logout` — full OAuth PKCE flow, keytar storage
- [ ] `oni chat` — basic ink REPL (no Mission Control, no sub-agents yet)
- [ ] Core tool set: `read_file`, `write_file`, `bash`, `list_directory`
- [ ] Streaming SSE response rendering in REPL
- [ ] `--dry-run` default, `--write` and `--exec` flags for permissions
- [ ] `oni ask` — pipe/stdin mode, stdout streaming
- [ ] Config: `~/.config/oni/config.json` — model, thresholds
- [ ] SQLite schema: conversations, messages, tool_events
- [ ] Biome lint + Vitest scaffolding
- [ ] `npm install -g oni-cli` installable

### Exit criteria
- `oni login` completes on macOS, Ubuntu, Windows WSL2
- `oni chat` can: read a file, explain it, write a fix, run tests — in one session
- `oni ask` works in a Unix pipeline
- Tokens stored in OS keychain (verify with `security find-generic-password -s oni-cli`)
- All unit tests pass (`npm test`)
- No plaintext secrets anywhere on disk

### Known limitations at end of Phase 1
- No sub-agent prefixes (single flat Claude call)
- No context engine (no project indexing)
- No Mission Control
- No preference learning
- No sync

---

## Phase 2 — Context Engine + Inline Shell (Weeks 5–8)

**Goal:** ONI understands large codebases. Inline shell trigger works in ZSH/Bash.

### Deliverables

- [ ] `oni init` — full tree-sitter index of project
- [ ] Incremental re-index via chokidar
- [ ] Multi-signal retrieval: symbol lookup + import graph + BM25 (ripgrep)
- [ ] Context packer — assembles prompt within token budget
- [ ] `.oniignore` support
- [ ] `oni pin <path>` — scope restriction
- [ ] `oni index stats` / `oni index rebuild`
- [ ] ZSH hook (`:` prefix trigger) — auto-installed by `oni login`
- [ ] Bash hook (DEBUG trap)
- [ ] `--continue` flag for inline trigger
- [ ] `CLAUDE.md` / `.oni-context` auto-injection
- [ ] SQLite index schema: files, symbols, import_edges, chunks_fts

### Exit criteria
- `oni init` on a 50k LOC TypeScript project in <45s
- Symbol lookup returns correct file within 5ms
- Context packer never exceeds configured token budget
- `: explain this function` works in ZSH on a real project
- File change triggers re-index within 500ms
- Retrieval quality test: 20 hand-crafted queries, ≥16 return the correct file in top-3

### Known limitations at end of Phase 2
- No sub-agent architecture (still flat Claude call)
- No Mission Control
- No preference learning
- No sync

---

## Phase 3 — Mission Control + Sub-Agents + Sync (Weeks 9–14)

**Goal:** The ONI personality layer is live. Mission Control is the operational hub. claude.ai sync works.

### Deliverables

**Sub-agents:**
- [ ] LangGraph state machine: Planner → Executor → Critic
- [ ] Sub-agent system prompt with `[Σ]` / `[⚡]` / `[⊘]` prefix enforcement
- [ ] Critic veto loop with `replanCount` cap (max 2)
- [ ] Blocked state escalation to user
- [ ] ONI personality: terse, direct, no apology, no filler

**Mission Control:**
- [ ] ink dashboard: stat bar, task queue, tool log, active diff, sub-agent status
- [ ] `oni mc` command
- [ ] `:mc` keybinding in REPL
- [ ] SQLite polling (500ms interval)
- [ ] `oni diff accept` / `oni diff reject`

**Sync daemon:**
- [ ] Background poll process (launchd/systemd unit)
- [ ] `oni sync <conv_id>` attach command
- [ ] Pull: new web messages → SQLite
- [ ] Push: terminal turns → claude.ai conversation endpoint
- [ ] Sync status panel in Mission Control
- [ ] Graceful degradation on API failure

**Context compaction:**
- [ ] Trigger at 60% context budget consumed
- [ ] Critic-driven compaction digest
- [ ] History pruned, digest retained

### Exit criteria
- Sub-agent prefixes visible in REPL output
- Critic correctly rejects a deliberately bad output (test fixture)
- Replan loop terminates at max 2 iterations
- Mission Control renders all panels with live data
- `oni sync <id>` successfully mirrors a claude.ai conversation into REPL
- Compaction reduces context by ≥40% on a long session
- Sync daemon restarts automatically after kill (systemd test)

---

## Phase 4 — Adaptive Layer + MCP Plugins + CI Integration (Weeks 15–20)

**Goal:** ONI learns user preferences. Plugin ecosystem is live. CI self-healing works.

### Deliverables

**Preference learning:**
- [ ] Tool proposal UI (numbered list, accept/reject/skip/always)
- [ ] Signal capture → `tool_events` table
- [ ] Preference weight calculation with decay
- [ ] Rule crystallisation background job
- [ ] `oni prefs` commands (list, reset, export, import)
- [ ] Learned rules injected into system prompt

**MCP plugin system:**
- [ ] `@modelcontextprotocol/sdk` integration
- [ ] stdio + SSE transports
- [ ] `oni plugin add/rm/list/enable/disable`
- [ ] Built-in registry: github, npm, postgres, sentry, docker
- [ ] Tool broker (merge built-ins + plugin tools)
- [ ] `.oni/plugins.json` per-project manifest
- [ ] Auth: env var + keytar + OAuth per-plugin

**CI/CD:**
- [ ] `oni watch-ci` — GitHub Actions polling
- [ ] Failure log fetch + analysis
- [ ] Fix proposal + re-push flow
- [ ] `oni pr` — branch, commit, push, gh pr create

**Background agents:**
- [ ] `oni run --background` worker process
- [ ] `oni sweep` with checkpointing
- [ ] `oni run --list/--kill/--logs`

**JSON event stream:**
- [ ] `--json` flag on all commands (NDJSON output)

### Exit criteria
- After 10 accept/reject signals, auto-threshold correctly bypasses proposals for accepted tools
- Rule crystallised after 10+ observations with >85% confidence
- `oni plugin add github` installs and tools appear in next turn
- Plugin crash does not crash ONI main process
- `oni watch-ci` correctly diagnoses a Node version mismatch failure (test fixture)
- `oni sweep` is resumable after simulated crash
- `--json` output is valid NDJSON parseable by `jq`

---

## Phase 5 — Polish + Distribution (Weeks 21–26)

**Goal:** Ship-ready. Anyone can install ONI in 30 seconds. Docs are complete. Performance is solid.

### Deliverables

**Distribution:**
- [ ] `npm install -g oni-cli` — published to npm
- [ ] Homebrew tap: `brew install oni-ai/tap/oni`
- [ ] curl installer: `curl -fsSL https://get.oni.ai | sh`
- [ ] Standalone binary (pkg) for no-Node environments
- [ ] GitHub Actions release pipeline (tag → publish)

**Performance:**
- [ ] Cold start: `oni chat` ready in <500ms
- [ ] Context retrieval: <100ms for 100k LOC project
- [ ] Mission Control render: <200ms
- [ ] Memory: <150MB RSS for typical session

**UX polish:**
- [ ] First-run onboarding (`oni init` wizard)
- [ ] Error messages are human-readable (no stack traces in production)
- [ ] `oni help` covers all commands with examples
- [ ] `oni update` checks for and applies updates
- [ ] `oni doctor` — diagnoses common setup issues

**Security audit:**
- [ ] All tokens stored in keychain verified (no disk leakage)
- [ ] `--exec` flag required for all bash tool calls
- [ ] No eval or dynamic code execution in ONI core
- [ ] Dependency audit clean (no known CVEs)

**Documentation:**
- [ ] All `.md` files in this repo complete and accurate
- [ ] `CONTRIBUTING.md` enables a new contributor to run tests in <15 min
- [ ] Screencasts for: login, chat, mission control, plugin install

### Exit criteria
- Install + first conversation in <60 seconds on clean macOS and Ubuntu
- All Phase 1–4 acceptance criteria still pass
- 0 open P0 bugs
- Security audit complete with no critical findings
- Published to npm with correct semver
- Homebrew tap formula passing CI

---

## Dependency Graph

```
Phase 1 (Foundation)
    ↓
Phase 2 (Context Engine)     ← can start in parallel with Phase 1 week 3+
    ↓
Phase 3 (Mission Control + Sub-agents + Sync)
    ↓
Phase 4 (Adaptive + Plugins + CI)
    ↓
Phase 5 (Polish + Distribution)
```

Phase 2 context engine work can begin as soon as Phase 1's SQLite schema is stable (week 3).

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Anthropic breaks claude.ai sync API | High | Medium | Ship export/import sync as fallback. Don't block other features on sync. |
| OAuth scope insufficient | Medium | High | Test PKCE flow against claude.ai early in Phase 1. |
| tree-sitter bindings unstable on Windows | Medium | Low | Fallback to ripgrep-only on Windows. |
| Context window budget too restrictive for large projects | Medium | Medium | Make token budget configurable. Allow per-project override in `.oni/config.json`. |
| MCP SDK breaking changes | Low | Medium | Pin MCP SDK version. Update on minor releases only. |
| keytar fails on headless Linux | Medium | Medium | Fallback to encrypted file storage with user-provided passphrase. |
