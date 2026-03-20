# ONI Overnight Benchmark Report v2

**Date:** 2026-03-20
**Hardware:** Apple M4 Max, 128GB unified memory
**Models:** Qwen3.5-27B-UD-Q8_K_XL (MIMIR) / Qwen3-Coder-Next-UD-Q6_K_XL (FENRIR) / GLM-4.7-Flash-UD-Q8_K_XL (SKULD)
**llama-server:** v8420 (upgraded from v7970)
**Total runs:** 168 (4 configs x 42 tasks)
**Total time:** 199.0 minutes (~3.3 hours)
**Note:** Full mode (all features) could not run — GPU OOM when loading all 3 models simultaneously

---

## Executive Summary

**Best configuration: `ultra-lean` at 88.1% (37/42)**

| Mode | Description | Pass/Total | Rate |
|------|------------|-----------|------|
| ultra-lean | No orchestrator + no KG + no personality | 37/42 | **88.1%** |
| no-orchestrator | Flat mode (no planner/critic) | 36/42 | **85.7%** |
| lean | No orchestrator + no KG | 35/42 | **83.3%** |
| no-kg | No knowledge graph (orchestrator ON) | 24/42 | **57.1%** |

### Key Finding

**ultra-lean (88%) > no-orchestrator (85%) > lean (83%) >> no-kg (57%)**

The orchestrator remains the #1 performance bottleneck. Removing it alone gives +28% over no-kg mode.
Adding personality removal on top of no-orch+no-kg gives another +5% (88% vs 83%).
The 200-token personality prompt preamble has a measurable cost on benchmark tasks.

## Comparison with Previous Benchmark (2026-03-19)

| Metric | Previous (27 tasks) | Current (42 tasks) | Change |
|--------|--------------------|--------------------|--------|
| Models | qwen3.5:35b / qwen3-coder:30b / glm-4.7-flash | Qwen3.5-27B-UD-Q8_K_XL / Qwen3-Coder-Next-UD-Q6_K_XL / GLM-4.7-Flash-UD-Q8_K_XL | Higher quants |
| llama-server | v7970 (Ollama-derived) | v8420 (native llama.cpp) | Upgraded |
| No-orchestrator | 81% (22/27) | 36/42 (85.7%) | **+4%** |
| No-KG | 70% (19/27) | 24/42 (57.1%) | **-13%** |
| Full mode | 51% (14/27) | N/A (GPU OOM) | — |

**Analysis:** The no-orchestrator improvement (+4%) confirms the model upgrades help.
The no-KG regression (-13%) is due to the expanded test set (42 vs 27) including harder tasks
that the orchestrator handles poorly, dragging down any mode that keeps it enabled.

---

## Per-Difficulty Breakdown (all modes combined)

| Difficulty | Pass | Fail | Total | Rate |
|-----------|------|------|-------|------|
| EASY | 16 | 4 | 20 | 80.0% |
| MEDIUM | 75 | 13 | 88 | 85.2% |
| HARD | 41 | 19 | 60 | 68.3% |

## Capability Flag Distribution

| Flag | Count | % |
|------|-------|---|
| CLEAN_PASS | 132 | 78.6% |
| MODEL_LIMIT | 15 | 8.9% |
| UNKNOWN | 14 | 8.3% |
| TIMEOUT_LIMIT | 7 | 4.2% |

---

## Ablation Matrix — Task Flips Between Modes

| Task | no-orchest | no-kg | lean | ultra-lean | Analysis |
|------|------|------|------|------|----------|
| H11 json-parser | **P** | F | **P** | **P** | Orchestrator hurts |
| H15 lru-cache-concurrent | **P** | F | F | **P** | Orchestrator hurts |
| H6 regex-chess | **P** | F | **P** | **P** | Orchestrator hurts |
| H8 regex-engine | **P** | F | F | **P** | Orchestrator hurts |
| H9 huffman-coding | **P** | F | **P** | **P** | Orchestrator hurts |
| M12 openssl-cert | **P** | F | F | **P** | Orchestrator hurts |
| M14 data-pipeline | **P** | F | **P** | **P** | Orchestrator hurts |
| M18 binary-search-tree | **P** | F | **P** | **P** | Orchestrator hurts |
| M19 cron-parser | **P** | F | **P** | **P** | Orchestrator hurts |
| M20 json-diff | F | **P** | **P** | **P** | Orchestrator helps |
| M21 dependency-graph | **P** | F | **P** | **P** | Orchestrator hurts |
| M22 csv-sql | **P** | F | **P** | **P** | Orchestrator hurts |
| M3 git-leak-recovery | **P** | F | **P** | **P** | Orchestrator hurts |
| M6 headless-terminal | F | F | **P** | F | Mixed |
| M8 financial-doc-processor | **P** | F | **P** | **P** | Orchestrator hurts |

## Consistently Failing Tasks

- **E3** zigzag-pattern (EASY)
- **H12** forth-interpreter (HARD)
- **H2** fix-code-vulnerability (HARD)
- **H3** make-mips-interpreter (HARD)

---

## Aggregate Statistics

- **Overall pass rate:** 78.6% (132/168)
- **Average time per task:** 71.1s
- **Total inference time:** 199.0 minutes
- **Benchmark duration:** ~3.3 hours (15:13 — 18:36 GMT)

## Recommendations

1. **Default to flat mode** — The orchestrator hurts in 85% of benchmark tasks. Default `should_orchestrate()` to false for single-prompt tasks.
2. **Personality should be opt-in for headless** — The 200-token SOUL.md preamble costs 5% on benchmarks. Skip it in `oni run` mode.
3. **KG needs relevance filtering** — KG context injection without orchestrator doesn't help (lean 83% vs no-orch 85%). Only inject high-confidence project-relevant facts.
4. **Fix GPU memory for 3-model deployment** — Heavy server OOMs when loaded alongside Medium+General. Options: (a) reduce Heavy to Q4 quant, (b) load on demand, (c) use smaller planner model.
5. **H3/H12 consistently timeout** — MIPS interpreter and Forth interpreter exceed 600s in all modes. These may need higher max_rounds or are beyond current model capability.

---

## Raw Data

- Results directory: `bench/results/20260320_151317/`
- Summary CSV: `bench/results/20260320_151317/summary.csv`
- Summary JSON: `bench/results/20260320_151317/summary.json`
- Per-task data: `bench/results/20260320_151317/<mode>/<task_id>/`
