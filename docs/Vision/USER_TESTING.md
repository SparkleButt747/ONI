# ONI — User Testing Plan

Testing the UX of a CLI agent is different from testing a GUI. The "interface" is the terminal, the personality, and the interaction patterns — not visual layouts. This document covers how to test these.

---

## Goals

1. Validate that each interaction mode (`:` prefix, pipe, REPL) is learnable in <5 minutes
2. Validate that the adaptive tool proposal system does not feel invasive
3. Validate that Mission Control is comprehensible without documentation
4. Validate that the ONI personality (Graphic Realism voice) lands as intended — direct, not rude
5. Identify friction points that reduce the "zero friction" goal

---

## Participant Profile

### Phase 1 testing (internal, Phases 1–2)
- 3–5 developers who work daily in the terminal
- Must be comfortable with: ZSH/Bash, npm, git
- Prior AI coding tool experience: at least one of (Claude Code, Cursor, GitHub Copilot)

### Phase 2 testing (external, Phase 3+)
- 8–12 developers, mixed experience
- Segment A (4): Senior engineers, heavy terminal users, opinionated about tooling
- Segment B (4): Mid-level engineers, mix of GUI and terminal tools
- Segment C (2–4): Open-source contributors unfamiliar with AI agents

### Exclusion criteria
- Non-developers
- Users who have not used a terminal in the past month

---

## Test Sessions

### Session structure
- 60 minutes per participant
- 10 min: background and setup
- 40 min: task-based testing (guided tasks + think-aloud)
- 10 min: debrief interview

### Think-aloud protocol
Participants narrate what they expect to happen before each action and what they observe after. Facilitator does not prompt or explain — only records. Questions only for clarification of what participant said.

---

## Task Scripts

### Module 1: First contact (10 min)

**Task 1.1 — Install and login**
```
"Install ONI and log in. We'll help if you get stuck, but try on your own first."
```
Observe: how long does `oni login` take? Do they understand the OAuth flow? Is the browser open/close cycle confusing?

**Metrics:**
- Time to completed login (target: <3 min)
- Number of help requests (target: 0)
- Any confusion about what the browser is doing

**Task 1.2 — First message**
```
"Send ONI a message asking it to explain what's in this project directory."
```
(Project: a small TypeScript Express API, ~500 LOC, unfamiliar to participant)

Observe: do they know to use `oni chat`? Do they reach for the `:` trigger? What do they type?

---

### Module 2: Inline shell trigger (10 min)

**Task 2.1 — Basic inline**
```
"Without opening a new program, ask ONI why the last command failed."
```
Participant is at a terminal prompt. Last command was a failing `npm test`.

Observe: do they discover the `:` prefix? Do they try `oni ask` directly? Do they copy-paste the error into `oni chat`?

**Task 2.2 — Pipe usage**
```
"Run the tests and pipe the output to ONI. Ask it what's wrong."
```
Hint: "You can combine shell commands."

Observe: do they reach for `|`? Do they know about `2>&1`?

**Task 2.3 — Continue a session**
```
"Ask ONI a follow-up to your last question, using the same conversation."
```
Observe: do they know about `--continue`? Do they expect continuity without it?

---

### Module 3: REPL deep work (10 min)

**Task 3.1 — Multi-turn debug session**
```
"Open a persistent chat with ONI and debug this failing function. Work through it naturally."
```
(Provide a function with a subtle off-by-one error)

Observe: how many turns does it take? Does the participant read sub-agent prefixes? Do they notice when the Critic fires?

**Task 3.2 — File write + confirm**
```
"Ask ONI to fix the function."
```
Observe: do they understand the `[y/n/diff]` prompt? Do they use `diff` before accepting? Do they feel in control?

**Task 3.3 — Keybinding discovery**
```
"Use Mission Control to see what ONI has been doing."
```
Observe: do they know `:mc`? Do they guess it? Does Mission Control make sense without explanation?

---

### Module 4: Mission Control (5 min)

**Task 4.1 — Read the dashboard**
```
"Look at Mission Control and tell me: what is ONI currently doing? How many tokens has it used? Is the sync connected?"
```
Observe: can they answer all three questions within 30 seconds? What's confusing?

**Task 4.2 — Respond to an error**
```
"One of your background tasks has errored. Find it and retry it."
```
Observe: do they see the `ERROR` status? Do they find the `oni task retry` affordance?

---

### Module 5: Tool proposals (5 min)

**Setup:** Reset participant's preferences so all tools are in "propose" range.

**Task 5.1 — Accept a proposal**
```
"Ask ONI to fix the failing CI build."
```
ONI will propose: `[1] github:get_run_logs [2] read_file [3] bash`. Observe: does the participant understand the proposal? Do they feel in control?

**Task 5.2 — Skip and always-auto**
```
"ONI wants to search the web. You never want it to do that automatically. What do you do?"
```
Observe: do they find the `[s]` skip and `[a]` always options? Is the prompt readable?

---

## Debrief Interview

Conducted after tasks. Semi-structured — not scripted verbatim.

**Opening:**
- What was your overall impression?
- What was the most surprising thing?

**Probe areas:**
- "When ONI proposed tools before using them — how did that feel?"
- "Did the sub-agent labels ([Σ], [⚡], [⊘]) make sense to you? Did you pay attention to them?"
- "How would you describe ONI's 'personality' to a colleague?"
- "Was there a moment when you felt out of control or confused about what ONI was doing?"
- "Compare this to [Claude Code / Cursor / Copilot] — what's different?"
- "Would you use this in your daily workflow? What would have to change?"

**Closing:**
- On a scale of 1–10: ease of use / sense of control / trust in the agent
- What's the one thing you'd change?

---

## Metrics

### Quantitative
| Metric | Target | How measured |
|---|---|---|
| Time to first successful `oni chat` response | <3 min | Facilitator timer |
| `:` prefix discovered without hint | ≥60% of participants | Observation |
| `[y/n/diff]` prompt correctly understood | ≥90% | Observation |
| Mission Control 3-question task <30s | ≥70% | Facilitator timer |
| Tool proposal rejected without understanding | <10% | Observation |
| Session SUS (System Usability Scale) score | ≥70 | Post-session survey |

### Qualitative signals
- "Direct / confident" as personality description → positive
- "Rude / abrupt" as personality description → adjust tone calibration
- "I didn't know it was doing that" → tool visibility issue
- "I felt like it could delete everything" → permission model needs work
- "I want to see more / less of the sub-agent stuff" → verbosity calibration

---

## SUS Survey (Post-session)

Standard 10-item System Usability Scale adapted for CLI context:

1. I think I would like to use ONI frequently.
2. I found ONI unnecessarily complex.
3. I thought ONI was easy to use.
4. I think I would need support to use ONI.
5. I found the various features well integrated.
6. There was too much inconsistency in ONI.
7. Most people would learn to use ONI quickly.
8. I found ONI cumbersome to use.
9. I felt confident using ONI.
10. I needed to learn a lot before I could use ONI.

Scale: 1 (strongly disagree) to 5 (strongly agree). Score computed via standard SUS formula. Target: ≥70 (Good).

**Additional ONI-specific items:**
11. I felt in control of what ONI was doing at all times.
12. ONI's personality felt appropriate for a coding tool.
13. The sub-agent labels (Planner/Executor/Critic) helped me understand what ONI was doing.
14. The tool proposals felt helpful rather than intrusive.
15. I trust ONI to modify files on my machine.

Scale: 1–5. These are analysed separately, not added to SUS score.

---

## Iteration Triggers

| Finding | Action |
|---|---|
| SUS <70 | Emergency UX review before Phase 4 |
| >30% say "rude/abrupt" | Adjust Executor tone — add one sentence of context |
| >20% confused by `[y/n/diff]` | Redesign confirm prompt — add key legend |
| <50% discover `:` prefix | Add first-run hint and `oni help inline` |
| Mission Control 3-question task fails >40% | Reorganise panel layout |
| >25% say "I felt out of control" | Add `--dry-run` as default for Phase 1, add undo |
| Item 15 ("trust") mean <3.0 | Review permission model and confirmation flows |

---

## Session Logistics

**Equipment:**
- Fresh macOS or Ubuntu VM per participant (clean install)
- Terminal: default system terminal (not iTerm/Alacritty) to test without assumptions
- ONI installed and pre-authenticated (saves login task for later sessions)
- Screen + audio recording (with consent)
- Facilitator + 1 note-taker

**Consent:**
- Recording consent form
- Explicit statement: "We are testing ONI, not you. There are no wrong answers."
- Right to stop at any time

**Compensation:**
- $75 gift card per 60-minute session
- Or: contributor credit in ONI README (for open-source participants)

---

## Schedule

| Phase | When | Participants | Focus |
|---|---|---|---|
| Pilot test | End of Phase 1 | 2 internal | Catch broken tasks, calibrate timing |
| Round 1 | End of Phase 2 | 5 external | Core interaction modes |
| Round 2 | End of Phase 3 | 8 external | Mission Control, sub-agents, sync |
| Round 3 | Mid Phase 4 | 6 external | Tool proposals, plugin system |
| Accessibility review | Phase 5 | TBD | Screen reader, colour contrast, keyboard nav |
