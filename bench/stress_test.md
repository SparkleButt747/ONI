# ONI Stress Test — Terminal-Bench 2.0 Adapted Suite

**Date:** 2026-03-18
**Purpose:** Validate ONI's full pipeline (MIMIR/FENRIR/SKULD orchestration, tools, context, personality) against 27 adapted TermBench 2.0 tasks across all difficulty levels. Find bugs, identify model routing improvements, document weaknesses.

## Methodology
- Sequential execution (one task at a time — local model constraint)
- Headless debug mode (`oni run`) for full visibility into agent internals
- Each task: set up environment → run ONI → verify result → record outcome
- Fix-later approach: collect all findings, fix in bulk

## Model Ensemble Under Test

### Configuration A (Current Default)
| Tier | Model | Role |
|------|-------|------|
| Heavy | qwen3.5:35b | MIMIR (planning) |
| Code | qwen3-coder:30b | FENRIR (execution) |
| General | glm-4.7-flash:q4_k_m | SKULD (critic) |

### Configuration B (Alternative — test later)
| Tier | Model | Role |
|------|-------|------|
| Heavy | qwen3-coder:30b | MIMIR (planning) |
| Code | qwen3.5:35b | FENRIR (execution) |
| General | glm-4.7-flash:q4_k_m | SKULD (critic) |

### Configuration C (Speed-optimised — test later)
| Tier | Model | Role |
|------|-------|------|
| Heavy | qwen3.5:9b | MIMIR (planning) |
| Code | qwen3-coder:30b | FENRIR (execution) |
| General | qwen3.5:9b | SKULD (critic) |

## Task Selection (27 tasks, ~30% of 89)

### Easy (3)
| # | Task | Category | Expected Tools |
|---|------|----------|---------------|
| E1 | fix-git | Software Engineering | bash (git) |
| E2 | cobol-modernization | Software Engineering | read_file, write_file |
| E3 | zigzag-pattern | Software Engineering | write_file, bash |

### Medium (15)
| # | Task | Category | Expected Tools |
|---|------|----------|---------------|
| M1 | regex-log | Data Processing | write_file, bash |
| M2 | multi-source-data-merger | Data Processing | read_file, write_file, bash |
| M3 | git-leak-recovery | Software Engineering | bash (git) |
| M4 | build-cython-ext | Debugging | bash, read_file |
| M5 | polyglot-c-py | Software Engineering | write_file, bash |
| M6 | headless-terminal | Software Engineering | write_file, bash |
| M7 | constraints-scheduling | Personal Assistant | write_file, bash |
| M8 | financial-document-processor | Data Processing | read_file, write_file, bash |
| M9 | kv-store-grpc | Software Engineering | write_file, bash |
| M10 | query-optimize | Data Science | read_file, write_file, bash |
| M11 | extract-elf | File Operations | bash, write_file |
| M12 | gcode-to-text | File Operations | read_file, write_file |
| M13 | pypi-server | Software Engineering | write_file, bash |
| M14 | openssl-selfsigned-cert | Security | bash |
| M15 | sqlite-db-truncate | Debugging | bash, write_file |

### Hard (9)
| # | Task | Category | Expected Tools |
|---|------|----------|---------------|
| H1 | cancel-async-tasks | Software Engineering | write_file, bash |
| H2 | fix-code-vulnerability | Security | read_file, edit_file, bash |
| H3 | make-mips-interpreter | Software Engineering | write_file, bash |
| H4 | feal-differential-cryptanalysis | Mathematics | write_file, bash |
| H5 | circuit-fibsqrt | Software Engineering | write_file, bash |
| H6 | regex-chess | Software Engineering | write_file, bash |
| H7 | xz-exploit | Security | read_file, bash, write_file |
| H8 | gpt2-codegolf | Software Engineering | write_file, bash |
| H9 | fix-ocaml-gc | Software Engineering | read_file, edit_file, bash |

## Results Template

| # | Task | Difficulty | Result | Time | Tokens | Tools Used | Issues |
|---|------|-----------|--------|------|--------|------------|--------|
| E1 | fix-git | Easy | | | | | |
| ... | | | | | | | |
