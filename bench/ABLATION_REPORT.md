# ONI Ablation Test Report

**Date:** 2026-03-19 (00:55 - 05:20, ~4.5 hours)
**Hardware:** Apple M4 Max, 128GB unified memory
**Models:** qwen3.5:35b (MIMIR) / qwen3-coder:30b (FENRIR) / glm-4.7-flash (SKULD)
**Total tasks:** 108 (27 tasks x 4 modes)

---

## Executive Summary

The orchestrator and knowledge graph are **actively hurting** benchmark performance. Disabling the orchestrator improved pass rate from 51% to 81% (+30 percentage points). This is the single biggest finding.

| Mode | Pass/27 | Rate | Delta |
|------|---------|------|-------|
| full (all features) | 14 | 51% | baseline |
| no-knowledge-graph | 19 | 70% | **+19%** |
| no-orchestrator | 22 | **81%** | **+30%** |
| no-personality | 18 | 66% | **+15%** |

---

## Key Findings

### 1. Orchestrator overhead is the #1 bottleneck

The MIMIR/FENRIR/SKULD three-agent loop adds 30-60s of overhead per task. For most TermBench tasks (single-prompt, single-file), this overhead:
- Burns token budget on planning/critiquing that the executor already handles
- Adds extra inference calls (3-5 per task) that eat into the timeout
- Forces the executor through a rigid step-by-step flow when it could solve the problem in one pass

**Evidence:** H2 (fix-code-vulnerability) took 300s and FAILED with orchestrator, but completed in 12s and PASSED without it. M7, M8 show similar patterns.

**Recommendation:** The `should_orchestrate()` heuristic is too aggressive. It triggers on keywords like "create", "fix", "write" which match almost every TermBench prompt. Either:
- Raise the threshold significantly (require 2+ keywords, or detect multi-file tasks explicitly)
- Default to flat mode and only orchestrate when the user explicitly requests it (/plan)
- Use a token-count heuristic: only orchestrate if the prompt implies >500 LOC of output

### 2. Knowledge graph adds noise, not signal

The knowledge graph injects "remembered facts" from past sessions into the system prompt. On a fresh benchmark run, these are either empty or stale, adding irrelevant context that confuses the model.

**Evidence:** 5 tasks flipped from FAIL to PASS when KG was disabled. No tasks went the other direction.

**Recommendation:**
- Don't inject KG context for headless/benchmark runs
- Add a relevance threshold: only inject if similarity score > 0.7
- Consider only injecting KG facts that were created in the current project directory

### 3. Personality has modest negative impact

SOUL.md personality prompting adds ~200 tokens of preamble to every system prompt. This is a 15% penalty — modest but real.

**Evidence:** 4 tasks flipped from FAIL to PASS. The extra prompt tokens reduce the effective context window for actual task content.

**Recommendation:**
- Skip personality prompt in headless/benchmark mode
- For interactive use, keep it but measure ongoing impact
- Consider injecting personality only on first turn, not every turn

### 4. CapabilityFlag classification works well

| Flag | Count | Notes |
|------|-------|-------|
| CLEAN_PASS | 73 | Correct, model solved it |
| MODEL_LIMIT | 15 | Model capability ceiling (crypto, complex algorithms) |
| FRAMEWORK_LIMIT | 13 | ONI infrastructure caused the failure |
| TIMEOUT_LIMIT | 4 | Hit the clock |
| HARNESS_ISSUE | 3 | Test harness bugs (E3 mostly) |
| UNKNOWN | 0 | Good — everything classified |

The FRAMEWORK_LIMIT count (13) correlates strongly with orchestrator overhead — when the orchestrator is disabled, most of these become CLEAN_PASS.

---

## Per-Difficulty Breakdown (across all modes)

| Difficulty | Pass | Fail | Rate |
|-----------|------|------|------|
| Easy (12 runs) | 5 | 7 | 42% |
| Medium (60 runs) | 48 | 12 | 80% |
| Hard (36 runs) | 20 | 16 | 56% |

Easy tasks have low pass rate because E1 (git merge) and E3 (zigzag) consistently fail across all modes — these are harness/model issues, not feature issues.

---

## Aggregate Stats

- **Average tokens per task:** 32,234
- **Average time per task:** 138.4s
- **Total inference time:** ~4.5 hours
- **Telemetry files generated:** 108 (all valid JSON)

---

## Per-Task Ablation Matrix

Tasks that flipped between modes (most interesting for feature impact):

| Task | full | no-kg | no-orch | no-pers | Conclusion |
|------|------|-------|---------|---------|------------|
| M7 constraints | FAIL | PASS | PASS | PASS | KG/orch/pers all hurt |
| M8 financial-doc | FAIL | PASS | PASS | PASS | Same — overhead kills it |
| H2 fix-vuln | FAIL | PASS | PASS | PASS | Orchestrator especially bad |
| H4 crypto | PASS | PASS | PASS | PASS | Consistently passes now |
| H5 circuit | FAIL | FAIL | PASS | PASS | Orchestrator/personality hurt |
| H9 OCaml GC | PASS | PASS | PASS | PASS | Consistently passes |
| M11 extract-elf | FAIL | FAIL | PASS | FAIL | Only orchestrator removal helps |

---

## Recommended Next Steps

1. **Immediate: tune `should_orchestrate()`** — raise the bar significantly. Most single-prompt tasks should use flat mode.
2. **Immediate: skip KG/personality in headless mode** — these features are for interactive UX, not benchmarks.
3. **Short-term: improve KG relevance filtering** — only inject high-confidence, project-relevant facts.
4. **Short-term: measure personality impact on interactive sessions** — it may help UX even if it hurts benchmarks.
5. **Re-run with tuned heuristics** — expect 75-85% in full mode after fixes.

---

## Raw Data

- Summary CSV: `bench/results/20260319_005510/summary.csv`
- Telemetry JSON per task: `bench/results/20260319_005510/<task_id>/telemetry.json`
- Debug logs: `bench/results/20260319_005510/<task_id>/debug.log`
- Aggregate summary: `bench/results/20260319_005510/summary.json`
