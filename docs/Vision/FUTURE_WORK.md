# ONI — Future Work & Deferred Features

Features researched and validated but deferred due to current model capability limitations, infrastructure requirements, or scope. These are NOT abandoned — they represent the next evolution of ONI once the foundations are solid.

---

## Tier 1 — Blocked on Model Capability

These features require more capable local models than currently available via Ollama. Revisit when 70B+ models run at acceptable speed on Apple Silicon, or when smaller models improve at code generation reliability.

### Full Self-Programming (OpenSage-style)
- **Source:** OpenSage whitepaper (arXiv:2602.16891), SageAgent reference implementation
- **What:** Agents dynamically generate complete Python/Rust tool implementations at runtime, validate syntax, register in tool registry, and execute
- **Current state:** We implement a lighter version — `forge_tool` generates bash scripts only (safer, simpler validation)
- **Gap:** Full self-programming needs reliable code generation + sandbox validation. Current 30B models produce ~70% valid code on first attempt — not reliable enough for autonomous tool creation
- **When to revisit:** When local models achieve >95% syntax-valid code generation on HumanEval, or when we add Docker sandboxing for safe execution of generated code

### Neo4j Long-Term Knowledge Graph
- **Source:** OpenSage memory architecture
- **What:** Cross-project persistent knowledge stored in a full graph database with embedding-based + pattern-based retrieval
- **Current state:** We use SQLite FTS5 + BM25 retrieval + in-memory discovery graph
- **Gap:** Neo4j is too heavy a dependency for a local-first CLI tool. Our in-memory graph serves the same purpose for single-project work
- **When to revisit:** When ONI supports multi-project workspaces and needs to share knowledge across projects

### Agent Ensemble (N-way parallel)
- **Source:** OpenSage horizontal topology, Factory Droid multi-trajectory sampling
- **What:** Spawn N agents to tackle the same task independently, run validation on all outputs, select the best
- **Current state:** We do 2-trajectory sampling (try alternative on rejection)
- **Gap:** N-way parallel requires N× the inference cost. With local models at ~100 tok/s, 5-way parallel would take 5× as long
- **When to revisit:** When inference speed exceeds 500 tok/s on local hardware, or when we add cloud model fallback

---

## Tier 2 — Blocked on Infrastructure

These features are architecturally sound but require infrastructure we haven't built yet.

### Docker Sandboxing
- **Source:** OpenSage tool execution isolation, SageAgent container management
- **What:** Run generated/untrusted tool code inside Docker containers with resource limits
- **Current state:** We execute bash directly with a blocklist. forge_tool scripts run unsandboxed
- **Gap:** Requires Docker dependency, container lifecycle management, volume mounting for project files
- **When to revisit:** When we add the MCP plugin system (plugins should be sandboxed too)

### MCP Plugin System
- **Source:** ONI Vision docs (FEATURES.md Section 11), ForgeCode MCP support
- **What:** Model Context Protocol plugin loading — stdio + SSE transports, tool broker, plugin registry
- **Current state:** Deferred by user decision. Hardcoded tools only
- **Gap:** MCP SDK integration, plugin manifest parsing, tool namespacing
- **When to revisit:** After v0.2 stabilises. This is the extensibility story

### GitHub Integration (oni pr, oni watch-ci)
- **Source:** ONI Vision docs (FEATURES.md Sections 10, 12), Factory Droid SDLC coverage
- **What:** Create PRs, watch CI, auto-fix failures, push commits
- **Current state:** Deferred by user decision
- **Gap:** Requires GitHub API integration (likely via MCP plugin)
- **When to revisit:** After MCP plugin system is implemented — GitHub becomes the first plugin

### ZSH/Bash Shell Hook
- **What:** `:` prefix in terminal triggers ONI inline (outside the TUI), similar to GitHub Copilot CLI
- **Current state:** Not implemented. `:cmd` exists inside the TUI only
- **Gap:** Requires shell plugin (ZSH widget or Bash readline hook), IPC to running ONI daemon
- **When to revisit:** After daemon/background process story is resolved


---

## Tier 3 — Design Decisions to Revisit

### IDE Extension
- **Source:** Factory Droid (VS Code, JetBrains, Vim), ForgeCode VS Code extension
- **What:** ONI running as a language server or extension inside editors
- **Current state:** Terminal-first. No IDE integration
- **Gap:** Requires LSP implementation or extension development
- **When to revisit:** After core agent capabilities stabilise. Terminal-first is the right call for now

### Cloud Model Fallback
- **Source:** ForgeCode (300+ model support), Factory Droid (multi-provider)
- **What:** When local models fail or are too slow, optionally fall back to cloud APIs
- **Current state:** 100% Ollama local. This is a core design principle
- **Decision:** Keep local-first but consider an opt-in `--cloud-fallback` flag for users who want it
- **When to revisit:** User demand. If benchmarks show cloud models solving problems local models can't

### Full Topology Self-Assembly
- **Source:** OpenSage topology manager
- **What:** MIMIR designs the agent topology dynamically — "this task needs 3 parallel FENRIRs and 2 sequential SKULDs"
- **Current state:** We implement agent spawning (depth-limited) but the topology is still mostly static
- **Gap:** Requires the planner to output structured topology descriptions, not just step lists
- **When to revisit:** When MIMIR's planning reliability improves. Currently the step-list format is more reliable than topology JSON

### Prompt Templates (Handlebars-style)
- **Source:** ForgeCode agent definitions with `{{env.cwd}}`, `{{current_time}}` variables
- **What:** User-definable prompt templates with variable interpolation
- **Current state:** System prompts are static strings assembled in code
- **Gap:** Template engine dependency, variable resolution, error handling for missing vars
- **When to revisit:** When custom agent definitions are heavily used and users want more dynamic prompts

---

## Research References

| Source | Key Insight | Status |
|--------|------------|--------|
| [OpenSage](https://arxiv.org/html/2602.16891v1) | Self-programming agents, topology self-assembly, graph memory | Partially integrated (graph memory, agent spawning) |
| [OpenClaw](https://github.com/openclaw/openclaw) | SOUL.md personality, daily journals, emotional state, inner-life | Fully integrated |
| [Factory Droid](https://factory.ai/news/code-droid-technical-report) | HyperCode codebase graphs, multi-trajectory, DroidShield | Partially integrated (multi-trajectory, auto-lint) |
| [ForgeCode](https://github.com/antinomyhq/forge) | Named agent modes, custom definitions, undo, follow-up, compaction config | Fully integrated |
| [Stanford Generative Agents](https://arxiv.org/abs/2304.03442) | Reflection → personality evolution | Fully integrated (reflection engine) |
| [LangMem](https://langchain-ai.github.io/langmem/) | Procedural memory rewriting | Integrated (SOUL.md mutations via reflection) |
| [Mem0](https://arxiv.org/html/2504.19413v1) | Priority scoring + temporal decay | Integrated (preference signal decay) |

---

*Last updated: 2026-03-19*
