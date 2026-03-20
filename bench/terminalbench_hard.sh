#!/bin/bash
# ONI TerminalBench 2.0 — Adapted medium/hard problems from official terminal-bench
# These are local adaptations that don't require Docker
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
ONI=/Users/brndy.747/.cargo/bin/oni
RESULTS_DIR="/tmp/oni_tbench_results"
mkdir -p "$RESULTS_DIR"
PASS=0
FAIL=0
TOTAL=0

run_problem() {
    local name="$1"
    local setup_cmd="$2"
    local prompt="$3"
    local check_cmd="$4"
    local tier="${5:-code}"
    local max_rounds="${6:-15}"

    TOTAL=$((TOTAL + 1))
    echo ""
    echo "═══════════════════════════════════════════"
    echo "  [$TOTAL] $name"
    echo "═══════════════════════════════════════════"

    # Setup
    eval "$setup_cmd" 2>/dev/null || true

    # Run ONI
    local output
    output=$($ONI run --tier "$tier" --max-rounds "$max_rounds" $prompt 2>"$RESULTS_DIR/${name}.debug.log") || true
    echo "$output" > "$RESULTS_DIR/${name}.output"

    # Check
    if eval "$check_cmd" 2>/dev/null; then
        echo "  ✓ PASS"
        PASS=$((PASS + 1))
    else
        echo "  ✗ FAIL"
        FAIL=$((FAIL + 1))
        echo "  DEBUG: see $RESULTS_DIR/${name}.debug.log"
    fi
}

echo "╔══════════════════════════════════════════╗"
echo "║  ONI × TERMINALBENCH 2.0 (ADAPTED)      ║"
echo "║  Model: qwen3-coder:30b                 ║"
echo "║  Max rounds: 15                         ║"
echo "╚══════════════════════════════════════════╝"

# ─── TB-1: Fibonacci Server (from fibonacci-server) ──────────
# Create and test a fibonacci HTTP endpoint
run_problem "fibonacci-server" \
    "mkdir -p /tmp/tb_fib && cd /tmp/tb_fib" \
    "Create a Python HTTP server file called server.py that runs on port 3456 with a single GET endpoint /fib that takes query param n and returns JSON with key result containing the nth fibonacci number. If n is missing return 400. Use only stdlib http.server. Do NOT start the server, just write the file." \
    "cd /tmp/tb_fib && python3 -c \"
import importlib.util, json, io, urllib.parse
spec = importlib.util.spec_from_file_location('srv', 'server.py')
mod = importlib.util.module_from_spec(spec)
# Just verify the file parses and has the right structure
with open('server.py') as f:
    code = f.read()
assert 'fib' in code.lower()
assert '3456' in code or 'PORT' in code
assert 'result' in code
print('OK')
\" 2>&1 | grep -q OK" \
    "code" "8"

# ─── TB-2: Regex Log Parsing (from regex-log) ──────────────
run_problem "regex-log" \
    "mkdir -p /tmp/tb_regex && cd /tmp/tb_regex && cat > access.log << 'LOGEOF'
2024-01-15 192.168.1.100 GET /api/users 200 2024-01-15
2024-02-20 10.0.0.1 POST /api/login 401 2024-02-21
No IP here 2024-03-01
192.168.1.50 GET /health 200 2024-04-10 extra 2024-04-11
2024-05-01 just text no ip
LOGEOF" \
    "Read access.log. Write a Python script called extract.py that uses regex to find lines containing an IPv4 address, and from those lines extracts the LAST date in YYYY-MM-DD format. Print each extracted date on its own line." \
    "cd /tmp/tb_regex && python3 extract.py 2>&1 | wc -l | tr -d ' ' | grep -q '^3$'" \
    "code" "8"

# ─── TB-3: Bank Transaction Filter (from bank-trans-filter) ──
run_problem "bank-trans-filter" \
    "mkdir -p /tmp/tb_bank && cd /tmp/tb_bank && cat > transactions.csv << 'CSVEOF'
id,date,company,account,amount,type
1,2024-01-15,North West Capital,ACC001,5000.00,credit
2,2024-01-16,South East Trading,ACC002,3200.00,debit
3,2024-01-17,Northwest Capital,ACC001,1500.00,credit
4,2024-01-18,North East Ventures,ACC003,8000.00,credit
5,2024-01-19,North West Capital,ACC001,2000.00,debit
6,2024-01-20,NW Capital,ACC001,750.00,credit
7,2024-01-21,South West Holdings,ACC004,4500.00,debit
8,2024-01-22,North West Capitall,ACC001,3000.00,credit
CSVEOF" \
    "Read transactions.csv. Filter to only transactions for North West Capital. The company may have typos but uses the same account number ACC001. Write a Python script called filter.py that outputs matching transactions as a JSON list to stdout." \
    "cd /tmp/tb_bank && python3 filter.py 2>&1 | python3 -c \"import json,sys; d=json.load(sys.stdin); assert len(d)==5; print('OK')\" 2>&1 | grep -q OK" \
    "code" "8"

# ─── TB-4: Async Task Cancellation (from cancel-async-tasks) ─
run_problem "async-tasks" \
    "mkdir -p /tmp/tb_async && cd /tmp/tb_async" \
    "Create a Python file called run.py with an async function run_tasks that takes a list of async callables and max_concurrent int. It should run tasks with concurrency limit using asyncio.Semaphore. When interrupted via KeyboardInterrupt, all running tasks cleanup code should still execute. Include a test at the bottom that creates 5 tasks sleeping 0.1s each with max_concurrent=2 and prints DONE." \
    "cd /tmp/tb_async && timeout 10 python3 run.py 2>&1 | grep -q 'DONE'" \
    "code" "8"

# ─── TB-5: Multi-Source Data Merger (from multi-source-data-merger) ─
run_problem "data-merger" \
    "mkdir -p /tmp/tb_merge && cd /tmp/tb_merge && cat > users_a.json << 'JEOF'
[{\"id\":1,\"name\":\"Alice\",\"email\":\"alice@a.com\",\"age\":30},{\"id\":2,\"name\":\"Bob\",\"email\":\"bob@b.com\",\"age\":25}]
JEOF
cat > users_b.csv << 'CEOF'
user_id,full_name,email_address,department
1,Alice Smith,alice@a.com,Engineering
3,Charlie Brown,charlie@c.com,Marketing
CEOF
cat > users_c.yaml << 'YEOF'
- uid: 2
  display_name: Robert
  mail: bob@b.com
  role: admin
- uid: 4
  display_name: Diana
  mail: diana@d.com
  role: user
YEOF" \
    "Merge user data from users_a.json, users_b.csv, and users_c.yaml. Match users by email. JSON source has highest priority for name. Create merge.py that writes merged_users.json with all unique users and combined fields." \
    "cd /tmp/tb_merge && python3 merge.py 2>/dev/null && python3 -c \"import json; d=json.load(open('merged_users.json')); assert len(d)==4; emails=set(u.get('email','') or u.get('email_address','') or u.get('mail','') for u in d); assert 'alice@a.com' in emails; print('OK')\" 2>&1 | grep -q OK" \
    "code" "10"

# ─── TB-6: Blind Maze Algorithm (from blind-maze-explorer) ───
run_problem "maze-solver" \
    "mkdir -p /tmp/tb_maze && cd /tmp/tb_maze && cat > maze.txt << 'MEOF'
#######
#S    #
# ### #
# #   #
# # # #
#   #E#
#######
MEOF" \
    "Read maze.txt where S is start, E is end, # is wall, space is path. Write solve_maze.py that finds the shortest path from S to E using BFS and prints the path length and the solved maze with the path marked as dots." \
    "cd /tmp/tb_maze && python3 solve_maze.py 2>&1 | grep -qi 'path\|length\|steps\|[0-9]'" \
    "code" "8"

# ─── TB-7: Git Leak Recovery (from git-leak-recovery) ────────
run_problem "git-leak-recovery" \
    "mkdir -p /tmp/tb_git && cd /tmp/tb_git && rm -rf .git && git init -q && git config user.email t@t.com && git config user.name T && echo 'API_KEY=sk-secret-12345' > .env && echo 'app code' > main.py && git add -A && git commit -q -m 'initial with secret' && echo 'API_KEY=REDACTED' > .env && git add -A && git commit -q -m 'remove secret'" \
    "This git repo had a secret API key committed and then removed. Find the leaked secret from git history and write it to a file called found_secret.txt." \
    "cd /tmp/tb_git && cat found_secret.txt 2>&1 | grep -q 'sk-secret-12345'" \
    "code" "8"

# ─── TB-8: Seat Assignment (from assign-seats) ───────────────
run_problem "seat-assignment" \
    "mkdir -p /tmp/tb_seats && cd /tmp/tb_seats && cat > preferences.json << 'PEOF'
{\"Alice\":{\"must_sit_next_to\":[\"Bob\"],\"cannot_sit_next_to\":[\"Charlie\"]},\"Bob\":{\"must_sit_next_to\":[\"Alice\"],\"cannot_sit_next_to\":[\"David\"]},\"Charlie\":{\"must_sit_next_to\":[],\"cannot_sit_next_to\":[\"Alice\"]},\"David\":{\"must_sit_next_to\":[\"Ethan\"],\"cannot_sit_next_to\":[\"Bob\"]},\"Ethan\":{\"must_sit_next_to\":[\"David\"],\"cannot_sit_next_to\":[]},\"Frankie\":{\"must_sit_next_to\":[],\"cannot_sit_next_to\":[]}}
PEOF" \
    "Read preferences.json which contains seating preferences for 6 guests at a circular dinner table. Write solve_seats.py that finds a valid seating arrangement satisfying all constraints. Print the arrangement as a list." \
    "cd /tmp/tb_seats && python3 solve_seats.py 2>&1 | grep -q 'Alice'" \
    "code" "10"

# ─── TB-9: Cython Extension Build (from build-cython-ext) ────
run_problem "cython-build" \
    "mkdir -p /tmp/tb_cython && cd /tmp/tb_cython" \
    "Create a simple Python extension using ctypes and C. Write a C file called fast_sum.c with a function that sums an array of ints. Write a Python file called use_fast.py that compiles it with gcc, loads it with ctypes, and demonstrates it summing the list 1 to 1000000. Print the result." \
    "cd /tmp/tb_cython && python3 use_fast.py 2>&1 | grep -q '500000500000'" \
    "code" "10"

# ─── TB-10: Complex Data Pipeline ────────────────────────────
run_problem "data-pipeline" \
    "mkdir -p /tmp/tb_pipe && cd /tmp/tb_pipe && python3 -c \"
import json, random; random.seed(42)
data = []
for i in range(1000):
    data.append({'id': i, 'value': random.gauss(100, 15), 'category': random.choice(['A','B','C']), 'valid': random.random() > 0.1})
json.dump(data, open('raw_data.json','w'))
\"" \
    "Read raw_data.json containing 1000 records. Create pipeline.py that: 1) filters to only valid=true records, 2) groups by category, 3) computes mean and std of value per category, 4) finds outliers more than 2 std from mean, 5) writes summary.json with stats per category and outlier count." \
    "cd /tmp/tb_pipe && python3 pipeline.py 2>/dev/null && python3 -c \"import json; d=json.load(open('summary.json')); cats=list(d.keys()); assert len(cats)==3; assert all('mean' in d[c] for c in cats); print('OK')\" 2>&1 | grep -q OK" \
    "code" "10"

# ─── RESULTS ────────────────────────────────────
echo ""
echo "╔══════════════════════════════════════════╗"
echo "║  RESULTS: $PASS/$TOTAL PASSED             "
echo "║  SCORE: $(( PASS * 100 / TOTAL ))%                        "
echo "║  FAILS: $FAIL                             "
echo "╚══════════════════════════════════════════╝"
echo ""
echo "Debug logs: $RESULTS_DIR/"
