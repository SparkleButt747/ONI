# ONI Complete Implementation Plan

> **For agentic workers:** Executing now via subagent-driven-development. User is asleep — no stops between phases.

**Goal:** Implement all features from the Vision docs that apply to the Ollama-local architecture.

**Architecture:** Three-agent system (Planner=qwen3.5:35b, Executor=qwen3-coder:30b, Critic=glm-4.7-flash), adaptive preference learning with DB persistence, expanded tool set, safety constraints, proper test suite.

**Decisions (from user):**
- SKIP: Cloud sync, OAuth/login, MCP plugins, GitHub integration
- BUILD: Everything else from the Vision docs
- Agent roles: Heavy=Planner, Code=Executor, General=Critic
- Tools: Hardcoded (search_files, edit_file, get_url) — no plugin system yet

---

## Execution Order (by impact)

### Phase 1: Three-Agent Architecture
- Planner agent (decomposes tasks into steps)
- Executor agent (executes tools per step)
- Critic agent (reviews results, can veto/replan)
- Agent state machine (idle/planning/executing/reviewing/blocked/done)
- Sub-agent prefix rendering in TUI

### Phase 2: Expanded Tools
- search_files (ripgrep wrapper)
- edit_file (patch-based, not full rewrite)
- get_url (HTTP fetch for web content)

### Phase 3: Safety & Security
- Always-show-diff before write_file
- Bash blocklist (rm -rf /, sudo, etc.)
- CWD constraints (no writes outside project root)

### Phase 4: Adaptive Preference Learning
- Signal capture (accept/reject/edit events)
- Preference scoring with decay
- Rule crystallisation to DB
- Injection into system prompt

### Phase 5: Context Engine Improvements
- Bump num_ctx to 32768
- Context compaction for large files
- .oniignore support

### Phase 6: CLI Commands
- oni ask --json (NDJSON event stream)
- oni ask stdin pipe
- oni sweep (multi-step autonomous tasks)
- oni config set/get
- oni prefs show/reset/export
- oni index stats/rebuild

### Phase 7: REPL Improvements
- Ctrl+R fuzzy history search
- Up/Down arrow command history
- : prefix inline shell

### Phase 8: Test Suite
- Unit tests for all crates
- Integration tests
- Benchmark harness

### Phase 9: TerminalBench 2.0
- Run official problems
- Write report
