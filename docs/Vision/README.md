# ONI — Onboard Neural Intelligence

> *"Reduce friction between developer and agent to zero. Ship code 24/7, around the clock."*

ONI is an open-source CLI agent for non-commercial developers. It provides a Claude-powered coding agent via OAuth (no API key required), with a terminal-first UX, persistent session memory, adaptive tool learning, and a distinctive "Graphic Realism" aesthetic inspired by Bungie's Marathon.

---

## Document Index

| File | Contents |
|---|---|
| `README.md` | Vision, goals, non-goals (this file) |
| `TECH_STACK.md` | Full stack decisions with rationale |
| `ARCHITECTURE.md` | System architecture, data flow, module map |
| `FEATURES.md` | Complete feature specifications |
| `PHASES.md` | Build roadmap, milestones, exit criteria |
| `DESIGN_SYSTEM.md` | Visual language, colour, typography, motion |
| `TESTING.md` | Unit, integration, E2E, and eval strategy |
| `USER_TESTING.md` | UI/UX testing plan, scripts, metrics |
| `API_CONTRACTS.md` | Internal interfaces, message formats, schemas |
| `SECURITY.md` | Auth model, permissions, threat model |
| `CONTRIBUTING.md` | Dev setup, conventions, PR process |

---

## Vision

Most AI coding tools have the same failure mode: they optimise for the demo, not the workflow. They require API keys that cost money. They interrupt the developer's mental model with context switches. They forget everything between sessions. They never learn your preferences.

ONI is built on a different premise:

**An agent should feel like a member of your team, not a chatbot you query.**

It should know your codebase, remember decisions you've made together, learn how you like to work, surface tools proactively, and operate in the background while you think. It should be opinionated, direct, and honest — not reassuring and verbose.

---

## Core Principles

### 1. OAuth-first, no API key
Non-commercial users access Claude via `oni login` — a standard OAuth 2.0 PKCE flow against claude.ai. No credit card, no Anthropic console, no API key. This is the primary access model for ONI. Commercial users can supply an API key as an override.

### 2. Terminal-native
ONI lives in the terminal. It does not require a browser, a GUI, or a VS Code extension. Three interaction modes:
- `:` prefix inline in any ZSH/Bash shell
- `oni ask` piped from stdin
- `oni chat` full REPL

### 3. Adaptive and learning
ONI learns user preferences on tool use from every session. It proposes tools before using them, captures accept/reject signals, and crystallises learned behaviour into persistent rules. Over time it stops asking and just knows.

### 4. Three sub-agents, one voice
ONI's internal architecture uses three named sub-agents operating within a single Claude context:
- **Σ Planner** — decomposes missions, sets scope, flags ambiguity before execution
- **⚡ Executor** — runs the plan, calls tools, writes files, reports completed actions only
- **⊘ Critic** — reviews output post-task, has veto power, can requeue to Planner

All three present through a single terse, direct personality. Not an assistant. An operator.

### 5. Mission Control
A persistent ink-rendered TUI dashboard showing: active task queue, token burn rate, tool call log, active file diff, claude.ai sync status, sub-agent states. Invoked via `oni mc`.

---

## Non-Goals

- **Not a team tool.** No Slack integration, no org-wide deployment, no multi-user features in v1.
- **Not a VS Code extension.** Terminal-first. Editor integrations are post-v1.
- **Not a model-agnostic tool.** ONI is built for Claude specifically. Model switching is not a design goal.
- **Not an autonomous agent.** ONI does not act without user-initiated sessions. It has no cron jobs or always-on background processes beyond the sync daemon.
- **Not for commercial use without API key.** OAuth access is explicitly for personal, non-commercial use per Anthropic ToS.

---

## Inspiration and References

| Reference | What we take |
|---|---|
| **Claude Code** | Core agentic loop, tool design, permission model |
| **ForgeCode** | ZSH/Bash inline `:` prefix trigger, codebase context engine |
| **Droid (Factory)** | Background async tasks, CI/CD self-healing, JSON event streams, fail-safe permissions |
| **OpenClaw** | Personality system, named sub-agents, friction-reduction philosophy |
| **OpenBlock** | MCP plugin architecture pattern |
| **Marathon (Bungie)** | Visual design language ("Graphic Realism") |

---

## Tagline

**ONI. Onboard Neural Intelligence. Ship or die.**
