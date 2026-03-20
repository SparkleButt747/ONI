#!/bin/bash
# ONI Overnight Benchmark v2 — Expanded Terminal-Bench 2.0 Suite
# 42 tasks x 5 feature configurations = 210 runs
# Expected runtime: ~7-9 hours on M4 Max 128GB
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"
ONI="$HOME/.cargo/bin/oni"

# macOS timeout
if command -v gtimeout &>/dev/null; then
    TIMEOUT_CMD="gtimeout"
elif command -v timeout &>/dev/null; then
    TIMEOUT_CMD="timeout"
else
    echo "ERROR: 'timeout' or 'gtimeout' (from coreutils) is required."
    echo "  Install: brew install coreutils"
    exit 1
fi

# ═══════════════════════════════════════════════════════════════
# CONFIGURATION
# ═══════════════════════════════════════════════════════════════

TIMEOUT_EASY=120
TIMEOUT_MEDIUM=300
TIMEOUT_HARD=600

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
RESULTS_DIR="${SCRIPT_DIR}/results/${TIMESTAMP}"
mkdir -p "$RESULTS_DIR"

# Feature configurations to test
MODES=(
    "full"
    "no-orchestrator"
    "no-kg"
    "lean"
    "ultra-lean"
)

# ═══════════════════════════════════════════════════════════════
# HELPERS
# ═══════════════════════════════════════════════════════════════

log_system() {
    local name="$1"
    vm_stat 2>/dev/null | head -5 > "$RESULTS_DIR/${name}.sysinfo" 2>/dev/null || true
    ps aux | grep llama-server | grep -v grep >> "$RESULTS_DIR/${name}.sysinfo" 2>/dev/null || true
}

build_flags() {
    local mode="$1"
    case "$mode" in
        full)            echo "" ;;
        no-orchestrator) echo "--no-orchestrator" ;;
        no-kg)           echo "--no-knowledge-graph" ;;
        no-personality)  echo "--no-personality" ;;
        lean)            echo "--no-orchestrator --no-knowledge-graph" ;;
        ultra-lean)      echo "--no-orchestrator --no-knowledge-graph --no-personality" ;;
    esac
}

classify_result() {
    local id="$1"
    local passed="$2"
    local elapsed="$3"
    local timeout_s="$4"
    local debug_log="$5"

    if [[ "$passed" == "PASS" ]]; then
        echo "CLEAN_PASS"
        return
    fi

    if [[ "$elapsed" -ge "$timeout_s" ]]; then
        echo "TIMEOUT_LIMIT"
        return
    fi

    if grep -qiE 'rate.limit|connection.refused|model.not.found|rpc.error|server.*error' "$debug_log" 2>/dev/null; then
        echo "FRAMEWORK_LIMIT"
        return
    fi

    if grep -qiE 'completed|finished|done|tool_rounds' "$debug_log" 2>/dev/null; then
        echo "MODEL_LIMIT"
        return
    fi

    echo "UNKNOWN"
}

# ═══════════════════════════════════════════════════════════════
# CORE TASK RUNNER
# ═══════════════════════════════════════════════════════════════

run_task() {
    local id="$1"
    local name="$2"
    local difficulty="$3"
    local prompt="$4"
    local check_cmd="$5"
    local tier="${6:-medium}"
    local max_rounds="${7:-15}"
    local run_mode="${8:-full}"

    TOTAL=$((TOTAL + 1))
    local task_num="$TOTAL"

    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  [$id] $name ($difficulty) [mode: $run_mode] [${task_num}/${SUITE_SIZE}]"
    echo "  $(date '+%H:%M:%S') | TIER: $tier | MAX_ROUNDS: $max_rounds"
    echo "═══════════════════════════════════════════════════════════════"

    log_system "$id"

    local task_timeout
    case "$difficulty" in
        EASY)   task_timeout="$TIMEOUT_EASY" ;;
        MEDIUM) task_timeout="$TIMEOUT_MEDIUM" ;;
        HARD)   task_timeout="$TIMEOUT_HARD" ;;
        *)      task_timeout="$TIMEOUT_MEDIUM" ;;
    esac

    local task_results_dir="$RESULTS_DIR/${run_mode}/${id}"
    mkdir -p "$task_results_dir"

    local debug_log="${task_results_dir}/debug.log"
    local output_file="${task_results_dir}/output.txt"
    local telemetry_file="${task_results_dir}/telemetry.json"

    local feature_flags
    feature_flags="$(build_flags "$run_mode")"

    local start_time
    start_time=$(date +%s)

    local output
    # shellcheck disable=SC2086
    output=$($TIMEOUT_CMD "$task_timeout" $ONI run \
        --tier "$tier" \
        --max-rounds "$max_rounds" \
        --telemetry \
        --telemetry-out "$telemetry_file" \
        $feature_flags \
        "$prompt" 2>"$debug_log") || true
    echo "$output" > "$output_file"

    local end_time
    end_time=$(date +%s)
    local elapsed=$((end_time - start_time))

    local tokens
    tokens=$(grep -o 'tokens.*' "$debug_log" 2>/dev/null | tail -1 || echo "?")

    local result_label
    # Wrap check_cmd in a 30s timeout to prevent hangs from infinite loops in generated code
    if $TIMEOUT_CMD 30 bash -c "$check_cmd" 2>/dev/null; then
        result_label="PASS"
        PASS=$((PASS + 1))
        echo "  ✓ PASS (${elapsed}s)"
    else
        result_label="FAIL"
        FAIL=$((FAIL + 1))
        echo "  ✗ FAIL (${elapsed}s)"
    fi

    local cap_flag
    cap_flag="$(classify_result "$id" "$result_label" "$elapsed" "$task_timeout" "$debug_log")"
    echo "  CAP_FLAG: $cap_flag"

    echo "$id|$name|$difficulty|$result_label|${elapsed}s|$tokens|$cap_flag|$run_mode|$tier" >> "$RESULTS_DIR/summary.csv"

    log_system "${id}_after"
}

# ═══════════════════════════════════════════════════════════════
# TASK DEFINITIONS — 42 tasks (5 easy, 22 medium, 15 hard)
# ═══════════════════════════════════════════════════════════════

SUITE_SIZE=42

run_suite() {
    local suite_mode="${1:-full}"

    PASS=0
    FAIL=0
    TOTAL=0

    echo ""
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║  SUITE: $suite_mode                                         "
    echo "║  Tasks: $SUITE_SIZE | Started: $(date)                      "
    echo "╚══════════════════════════════════════════════════════════════╝"

    # ───────────────────────────────────────────────────────────────
    # EASY TASKS (5)
    # ───────────────────────────────────────────────────────────────

    # E1: fix-git — Merge branches with conflict resolution
    cd /tmp && rm -rf oni_v2_e1 && mkdir oni_v2_e1 && cd oni_v2_e1
    git init -q && git config user.email "t@t.com" && git config user.name "T"
    echo "initial" > readme.md && git add . && git commit -q -m "initial"
    git checkout -q -b feature
    echo "feature work" > feature.txt && git add . && git commit -q -m "add feature"
    git checkout -q main 2>/dev/null || git checkout -q master
    echo "conflicting" > readme.md && git add . && git commit -q -m "master update"
    run_task "E1" "fix-git" "EASY" \
        "Merge the 'feature' branch into master in this git repo. Resolve any conflicts. Keep both changes." \
        "cd /tmp/oni_v2_e1 && git log --oneline | grep -q 'feature' || git log --oneline | grep -q 'Merge'" \
        "medium" "8" "$suite_mode"

    # E2: cobol-modernization — Convert COBOL to Python
    cd /tmp && rm -rf oni_v2_e2 && mkdir oni_v2_e2 && cd oni_v2_e2
    cat > program.cob << 'COBEOF'
       IDENTIFICATION DIVISION.
       PROGRAM-ID. HELLO-WORLD.
       DATA DIVISION.
       WORKING-STORAGE SECTION.
       01 WS-COUNT PIC 9(2) VALUE 0.
       01 WS-TOTAL PIC 9(4) VALUE 0.
       PROCEDURE DIVISION.
           PERFORM VARYING WS-COUNT FROM 1 BY 1 UNTIL WS-COUNT > 10
               ADD WS-COUNT TO WS-TOTAL
           END-PERFORM.
           DISPLAY "Total: " WS-TOTAL.
           STOP RUN.
COBEOF
    run_task "E2" "cobol-modernization" "EASY" \
        "Read program.cob and convert it to Python. The Python version must produce identical output. Write it to program.py." \
        "cd /tmp/oni_v2_e2 && python3 program.py 2>&1 | grep -qE 'Total: (55|0055)'" \
        "medium" "5" "$suite_mode"

    # E3: zigzag-pattern — String zigzag encoding
    cd /tmp && rm -rf oni_v2_e3 && mkdir oni_v2_e3 && cd oni_v2_e3
    run_task "E3" "zigzag-pattern" "EASY" \
        "Create zigzag.py that takes a string and number of rows, outputs the zigzag pattern reading. Example: zigzag('PAYPALISHIRING', 3) should output 'PAHNAPLSIIGYIR'. Include a test that verifies this example." \
        "cd /tmp/oni_v2_e3 && python3 zigzag.py 2>&1 | grep -qE 'passed|PAHNAPLSIIGYIR'" \
        "medium" "5" "$suite_mode"

    # E4: reverse-words — Reverse word order in sentences
    cd /tmp && rm -rf oni_v2_e4 && mkdir oni_v2_e4 && cd oni_v2_e4
    cat > input.txt << 'EOF'
hello world foo bar
the quick brown fox
EOF
    run_task "E4" "reverse-words" "EASY" \
        "Read input.txt. Create reverse_words.py that reads the file and prints each line with words reversed. 'hello world foo bar' becomes 'bar foo world hello'." \
        "cd /tmp/oni_v2_e4 && python3 reverse_words.py 2>&1 | head -1 | grep -q 'bar foo world hello'" \
        "medium" "5" "$suite_mode"

    # E5: word-frequency — Count word frequencies in a text file
    cd /tmp && rm -rf oni_v2_e5 && mkdir oni_v2_e5 && cd oni_v2_e5
    cat > text.txt << 'EOF'
the cat sat on the mat
the cat ate the rat
the rat ran from the cat
EOF
    run_task "E5" "word-frequency" "EASY" \
        "Read text.txt. Create word_freq.py that counts word frequencies and prints the top 3 most common words with their counts, one per line, format: 'word: N', sorted descending." \
        "cd /tmp/oni_v2_e5 && python3 word_freq.py 2>&1 | head -1 | grep -q 'the'" \
        "medium" "5" "$suite_mode"

    # ───────────────────────────────────────────────────────────────
    # MEDIUM TASKS (22)
    # ───────────────────────────────────────────────────────────────

    # M1: regex-log — Extract dates from log lines with IPv4
    cd /tmp && rm -rf oni_v2_m1 && mkdir oni_v2_m1 && cd oni_v2_m1
    cat > access.log << 'LOGEOF'
2024-01-15 10:23:45 192.168.1.100 GET /api/users 200
2024-01-15 10:24:01 10.0.0.1 POST /api/login 401
2024-01-16 08:00:00 no-ip-here GET /health 200
2024-01-16 09:15:22 172.16.0.50 DELETE /api/users/5 403
2024-01-17 14:30:00 invalid GET /api/data 500
LOGEOF
    run_task "M1" "regex-log" "MEDIUM" \
        "Read access.log. Write extract_dates.py that uses regex to extract the date from every line that contains a valid IPv4 address. Print dates one per line. Lines without valid IPs should be skipped." \
        "cd /tmp/oni_v2_m1 && python3 extract_dates.py | wc -l | tr -d ' ' | grep -q '^3$'" \
        "medium" "5" "$suite_mode"

    # M2: multi-source-data-merger
    cd /tmp && rm -rf oni_v2_m2 && mkdir oni_v2_m2 && cd oni_v2_m2
    cat > users.json << 'JEOF'
[{"email":"alice@example.com","name":"Alice","role":"admin"},{"email":"bob@example.com","name":"Bob","role":"user"}]
JEOF
    cat > profiles.csv << 'CEOF'
email,department,location
alice@example.com,Engineering,London
charlie@example.com,Marketing,NYC
CEOF
    cat > extra.yaml << 'YEOF'
- email: bob@example.com
  phone: "+44123456"
- email: alice@example.com
  phone: "+44789012"
YEOF
    run_task "M2" "multi-source-data-merger" "MEDIUM" \
        "Read users.json, profiles.csv, and extra.yaml. Write merge.py that merges all three sources by email, outputting a single merged.json with all fields. Handle missing fields gracefully (use null)." \
        "cd /tmp/oni_v2_m2 && python3 merge.py && python3 -c \"import json; d=json.load(open('merged.json')); assert len(d)==3; a=[x for x in d if x['email']=='alice@example.com'][0]; assert a['department']=='Engineering'; assert a['phone']=='+44789012'; print('OK')\" 2>&1 | grep -q OK" \
        "medium" "8" "$suite_mode"

    # M3: git-leak-recovery
    cd /tmp && rm -rf oni_v2_m3 && mkdir oni_v2_m3 && cd oni_v2_m3
    git init -q && git config user.email "t@t.com" && git config user.name "T"
    echo "config = {}" > config.py && git add . && git commit -q -m "init"
    echo 'SECRET_KEY = "sk_live_SUPER_SECRET_12345"' >> config.py && git add . && git commit -q -m "add config"
    echo "config = {}" > config.py && git add . && git commit -q -m "remove secret"
    run_task "M3" "git-leak-recovery" "MEDIUM" \
        "There is a secret key that was committed to this repo and then removed. Find the secret key from the git history and write it to a file called secret.txt." \
        "cd /tmp/oni_v2_m3 && grep -q 'sk_live_SUPER_SECRET_12345' secret.txt" \
        "medium" "8" "$suite_mode"

    # M4: build-c-extension — C extension via ctypes
    cd /tmp && rm -rf oni_v2_m4 && mkdir oni_v2_m4 && cd oni_v2_m4
    run_task "M4" "build-c-extension" "MEDIUM" \
        "Create a C file called fast_sum.c with a function int fast_sum(int* arr, int n) that sums an array. Create use_fast.py that compiles it with cc, loads with ctypes, and sums 1..1000000. Print the result (500000500000)." \
        "cd /tmp/oni_v2_m4 && python3 use_fast.py 2>&1 | grep -q '500000500000'" \
        "medium" "10" "$suite_mode"

    # M5: maze-solver — BFS maze solving
    cd /tmp && rm -rf oni_v2_m5 && mkdir oni_v2_m5 && cd oni_v2_m5
    cat > maze.txt << 'MEOF'
#######
#S    #
# ### #
# #   #
# # # #
#   #E#
#######
MEOF
    run_task "M5" "maze-solver" "MEDIUM" \
        "Read maze.txt where S is start, E is end, # is wall, space is path. Write solve_maze.py that finds the shortest path from S to E using BFS and prints the path length and the solved maze with dots marking the path." \
        "cd /tmp/oni_v2_m5 && python3 solve_maze.py 2>&1 | grep -qi 'path\|length\|steps\|[0-9]'" \
        "medium" "8" "$suite_mode"

    # M6: headless-terminal — Basic terminal emulator
    cd /tmp && rm -rf oni_v2_m6 && mkdir oni_v2_m6 && cd oni_v2_m6
    run_task "M6" "headless-terminal" "MEDIUM" \
        "Create headless_term.py that implements a basic terminal emulator. Accept commands via stdin, execute in subprocess, capture stdout/stderr, print output. Test: echo 'echo hello' | python3 headless_term.py" \
        "cd /tmp/oni_v2_m6 && echo 'echo hello' | python3 headless_term.py 2>&1 | grep -q 'hello'" \
        "medium" "8" "$suite_mode"

    # M7: constraints-scheduling
    cd /tmp && rm -rf oni_v2_m7 && mkdir oni_v2_m7 && cd oni_v2_m7
    cat > schedules.json << 'SEOF'
{
  "alice": ["09:00-10:00", "11:00-12:00", "14:00-16:00"],
  "bob": ["10:00-11:30", "13:00-15:00"],
  "charlie": ["09:00-10:30", "14:00-15:30"]
}
SEOF
    run_task "M7" "constraints-scheduling" "MEDIUM" \
        "Read schedules.json. Each person's array shows their BUSY times. Write scheduler.py that finds all 30-minute slots between 09:00-17:00 where ALL three are free. Print slots as 'HH:MM-HH:MM', one per line." \
        "cd /tmp/oni_v2_m7 && python3 scheduler.py | head -1 | grep -qE '[0-9]{2}:[0-9]{2}-[0-9]{2}:[0-9]{2}'" \
        "medium" "8" "$suite_mode"

    # M8: financial-doc-processor
    cd /tmp && rm -rf oni_v2_m8 && mkdir oni_v2_m8 && cd oni_v2_m8
    cat > invoice1.txt << 'INV1'
INVOICE #1001
Date: 2024-03-15
Vendor: Acme Corp
Items:
  Widget A x10 @ $29.99
  Widget B x5 @ $49.99
Total: $549.85
INV1
    cat > invoice2.txt << 'INV2'
INVOICE #1002
Date: 2024-03-16
Vendor: Globex Inc
Items:
  Service Fee x1 @ $500.00
  Consulting x2 @ $150.00
Total: $800.00
INV2
    run_task "M8" "financial-doc-processor" "MEDIUM" \
        "Read invoice1.txt and invoice2.txt. Create process_invoices.py that extracts: invoice number, date, vendor, total. Output summary.csv with columns: invoice_id,date,vendor,total" \
        "cd /tmp/oni_v2_m8 && python3 process_invoices.py && python3 -c \"import csv; rows=list(csv.DictReader(open('summary.csv'))); assert len(rows)==2; assert rows[0]['vendor'].strip()=='Acme Corp'; print('OK')\" 2>&1 | grep -q OK" \
        "medium" "8" "$suite_mode"

    # M9: query-optimize — SQLite query optimization
    cd /tmp && rm -rf oni_v2_m9 && mkdir oni_v2_m9 && cd oni_v2_m9
    python3 -c "
import sqlite3
conn = sqlite3.connect('test.db')
conn.execute('CREATE TABLE IF NOT EXISTS orders (id INT, customer TEXT, amount REAL, date TEXT)')
for i in range(10000):
    conn.execute('INSERT INTO orders VALUES (?, ?, ?, ?)', (i, f'cust_{i%100}', i*1.5, f'2024-{(i%12)+1:02d}-{(i%28)+1:02d}'))
conn.commit()
conn.close()
"
    cat > slow_query.sql << 'SQLEOF'
SELECT customer, SUM(amount) as total
FROM orders
WHERE date LIKE '2024-06%'
GROUP BY customer
ORDER BY total DESC
LIMIT 10;
SQLEOF
    run_task "M9" "query-optimize" "MEDIUM" \
        "Read slow_query.sql and test.db. Create optimize.py that: 1) Adds appropriate indexes, 2) Rewrites the query to be faster, 3) Runs both versions and prints timing comparison." \
        "cd /tmp/oni_v2_m9 && python3 optimize.py 2>&1 | grep -qiE 'fast|improv|index|optimiz|time'" \
        "medium" "8" "$suite_mode"

    # M10: extract-elf — Extract strings from compiled binary
    cd /tmp && rm -rf oni_v2_m10 && mkdir oni_v2_m10 && cd oni_v2_m10
    cat > secret.c << 'CEOF'
#include <stdio.h>
static const char SECRET[] = "ONI_HIDDEN_VALUE_42";
static const int MAGIC = 0xDEADBEEF;
int main() { printf("Nothing to see here\n"); return 0; }
CEOF
    cc -o secret secret.c
    run_task "M10" "extract-elf" "MEDIUM" \
        "There is a compiled binary called 'secret'. Extract all string constants and the magic number from it. Write findings to findings.json with keys 'strings' (array) and 'magic_hex' (string)." \
        "cd /tmp/oni_v2_m10 && python3 -c \"import json; d=json.load(open('findings.json')); assert 'ONI_HIDDEN_VALUE_42' in str(d); print('OK')\" 2>&1 | grep -q OK" \
        "medium" "8" "$suite_mode"

    # M11: gcode-to-text — Decode 3D printer paths
    cd /tmp && rm -rf oni_v2_m11 && mkdir oni_v2_m11 && cd oni_v2_m11
    cat > print.gcode << 'GEOF'
G28 ; Home
G1 Z5 F500
; Letter O
G1 X10 Y10 F1000
G1 X10 Y30
G1 X30 Y30
G1 X30 Y10
G1 X10 Y10
; Letter N
G1 X40 Y10
G1 X40 Y30
G1 X60 Y10
G1 X60 Y30
; Letter I
G1 X70 Y10
G1 X70 Y30
GEOF
    run_task "M11" "gcode-to-text" "MEDIUM" \
        "Read print.gcode. Analyze the movement commands to determine what text/letters the tool path traces. Write the decoded text to decoded.txt." \
        "cd /tmp/oni_v2_m11 && grep -qi 'ONI' decoded.txt" \
        "medium" "8" "$suite_mode"

    # M12: openssl-cert — Generate self-signed TLS certificate
    cd /tmp && rm -rf oni_v2_m12 && mkdir oni_v2_m12 && cd oni_v2_m12
    run_task "M12" "openssl-cert" "MEDIUM" \
        "Generate a self-signed TLS certificate for 'oni.local' using openssl. Create both the private key (key.pem) and certificate (cert.pem). Valid for 365 days, RSA 2048-bit." \
        "cd /tmp/oni_v2_m12 && openssl x509 -in cert.pem -text -noout 2>&1 | grep -q 'oni.local'" \
        "medium" "5" "$suite_mode"

    # M13: seat-assignment — Constraint satisfaction
    cd /tmp && rm -rf oni_v2_m13 && mkdir oni_v2_m13 && cd oni_v2_m13
    cat > preferences.json << 'PEOF'
{"Alice":{"must_sit_next_to":["Bob"],"cannot_sit_next_to":["Charlie"]},"Bob":{"must_sit_next_to":["Alice"],"cannot_sit_next_to":["David"]},"Charlie":{"must_sit_next_to":[],"cannot_sit_next_to":["Alice"]},"David":{"must_sit_next_to":["Ethan"],"cannot_sit_next_to":["Bob"]},"Ethan":{"must_sit_next_to":["David"],"cannot_sit_next_to":[]},"Frankie":{"must_sit_next_to":[],"cannot_sit_next_to":[]}}
PEOF
    run_task "M13" "seat-assignment" "MEDIUM" \
        "Read preferences.json — seating constraints for 6 guests at a circular table. Write solve_seats.py that finds a valid seating arrangement satisfying all constraints. Print the arrangement." \
        "cd /tmp/oni_v2_m13 && python3 solve_seats.py 2>&1 | grep -q 'Alice'" \
        "medium" "10" "$suite_mode"

    # M14: data-pipeline — Statistical analysis pipeline
    cd /tmp && rm -rf oni_v2_m14 && mkdir oni_v2_m14 && cd oni_v2_m14
    python3 -c "
import json, random; random.seed(42)
data = []
for i in range(1000):
    data.append({'id': i, 'value': round(random.gauss(100, 15), 2), 'category': random.choice(['A','B','C']), 'valid': random.random() > 0.1})
json.dump(data, open('raw_data.json','w'))
"
    run_task "M14" "data-pipeline" "MEDIUM" \
        "Read raw_data.json (1000 records). Create pipeline.py that: 1) filters valid=true, 2) groups by category, 3) computes mean+std per category, 4) finds outliers >2 std from mean, 5) writes summary.json with stats per category and outlier count." \
        "cd /tmp/oni_v2_m14 && python3 pipeline.py 2>/dev/null && python3 -c \"import json; d=json.load(open('summary.json')); cats=list(d.keys()); assert len(cats)==3; assert all('mean' in d[c] for c in cats); print('OK')\" 2>&1 | grep -q OK" \
        "medium" "10" "$suite_mode"

    # M15: bank-transaction-filter — Fuzzy company matching
    cd /tmp && rm -rf oni_v2_m15 && mkdir oni_v2_m15 && cd oni_v2_m15
    cat > transactions.csv << 'CSVEOF'
id,date,company,account,amount,type
1,2024-01-15,North West Capital,ACC001,5000.00,credit
2,2024-01-16,South East Trading,ACC002,3200.00,debit
3,2024-01-17,Northwest Capital,ACC001,1500.00,credit
4,2024-01-18,North East Ventures,ACC003,8000.00,credit
5,2024-01-19,North West Capital,ACC001,2000.00,debit
6,2024-01-20,NW Capital,ACC001,750.00,credit
7,2024-01-21,South West Holdings,ACC004,4500.00,debit
8,2024-01-22,North West Capitall,ACC001,3000.00,credit
CSVEOF
    run_task "M15" "bank-trans-filter" "MEDIUM" \
        "Read transactions.csv. Filter to only transactions for North West Capital. The company may have typos but uses the same account number ACC001. Write filter.py that outputs matching transactions as a JSON list to stdout." \
        "cd /tmp/oni_v2_m15 && python3 filter.py 2>&1 | python3 -c \"import json,sys; d=json.load(sys.stdin); assert len(d)==5; print('OK')\" 2>&1 | grep -q OK" \
        "medium" "8" "$suite_mode"

    # M16: markdown-to-html — Simple markdown parser
    cd /tmp && rm -rf oni_v2_m16 && mkdir oni_v2_m16 && cd oni_v2_m16
    cat > input.md << 'MDEOF'
# Hello World

This is a **bold** paragraph with `inline code`.

## Features

- Item one
- Item two
- Item three

```python
print("hello")
```
MDEOF
    run_task "M16" "markdown-to-html" "MEDIUM" \
        "Read input.md. Create md2html.py that converts it to HTML without using any markdown libraries (no 'import markdown'). Handle: headers (#, ##), bold (**), inline code, unordered lists, fenced code blocks. Write output to output.html." \
        "cd /tmp/oni_v2_m16 && python3 md2html.py && grep -q '<h1>' output.html && grep -q '<strong>' output.html && grep -q '<li>' output.html" \
        "medium" "8" "$suite_mode"

    # M17: rate-limiter — Token bucket rate limiter
    cd /tmp && rm -rf oni_v2_m17 && mkdir oni_v2_m17 && cd oni_v2_m17
    run_task "M17" "rate-limiter" "MEDIUM" \
        "Create rate_limiter.py implementing a token bucket rate limiter class. Constructor takes max_tokens and refill_rate (tokens/sec). Methods: acquire(n=1) -> bool, wait_acquire(n=1) -> awaits. Include a test that allows 5 rapid requests then blocks the 6th. Use max_tokens=5, refill_rate=1." \
        "cd /tmp/oni_v2_m17 && python3 rate_limiter.py 2>&1 | grep -qiE 'pass|blocked|OK|limit'" \
        "medium" "8" "$suite_mode"

    # M18: binary-search-tree — BST implementation
    cd /tmp && rm -rf oni_v2_m18 && mkdir oni_v2_m18 && cd oni_v2_m18
    run_task "M18" "binary-search-tree" "MEDIUM" \
        "Create bst.py implementing a binary search tree with: insert, search, delete, in_order traversal. Include test: insert [5,3,7,1,4,6,8], delete 3, verify in_order is [1,4,5,6,7,8]. Print 'PASS' if correct." \
        "cd /tmp/oni_v2_m18 && python3 bst.py 2>&1 | grep -q 'PASS'" \
        "medium" "8" "$suite_mode"

    # M19: cron-parser — Parse cron expression
    cd /tmp && rm -rf oni_v2_m19 && mkdir oni_v2_m19 && cd oni_v2_m19
    run_task "M19" "cron-parser" "MEDIUM" \
        "Create cron_parser.py that parses standard 5-field cron expressions (minute hour day month weekday). Implement next_run(expression, from_time) that returns the next execution time. Test with '*/15 * * * *' from 2024-01-15 10:22:00 — next should be 10:30. Print results." \
        "cd /tmp/oni_v2_m19 && python3 cron_parser.py 2>&1 | grep -q '10:30'" \
        "medium" "10" "$suite_mode"

    # M20: json-diff — Deep JSON comparison
    cd /tmp && rm -rf oni_v2_m20 && mkdir oni_v2_m20 && cd oni_v2_m20
    cat > a.json << 'AEOF'
{"name": "Alice", "age": 30, "hobbies": ["reading", "chess"], "address": {"city": "London", "zip": "SW1"}}
AEOF
    cat > b.json << 'BEOF'
{"name": "Alice", "age": 31, "hobbies": ["reading", "hiking"], "address": {"city": "Manchester", "zip": "SW1"}, "phone": "123"}
BEOF
    run_task "M20" "json-diff" "MEDIUM" \
        "Read a.json and b.json. Create json_diff.py that computes a deep diff between them. Print changes like: 'MODIFIED age: 30 -> 31', 'ADDED phone: 123', etc. Handle nested objects and arrays." \
        "cd /tmp/oni_v2_m20 && python3 json_diff.py 2>&1 | grep -qiE 'age.*30.*31|modif.*age'" \
        "medium" "8" "$suite_mode"

    # M21: dependency-graph — Topological sort
    cd /tmp && rm -rf oni_v2_m21 && mkdir oni_v2_m21 && cd oni_v2_m21
    cat > deps.json << 'DEOF'
{"A": ["B", "C"], "B": ["D"], "C": ["D"], "D": [], "E": ["A", "C"], "F": []}
DEOF
    run_task "M21" "dependency-graph" "MEDIUM" \
        "Read deps.json where each key depends on the listed values. Create topo_sort.py that performs topological sort and prints the build order. D must come before B and C. B and C before A. A before E." \
        "cd /tmp/oni_v2_m21 && python3 topo_sort.py 2>&1 | tr -d '[:space:]' | grep -q 'D'" \
        "medium" "8" "$suite_mode"

    # M22: csv-sql — SQL-like queries on CSV
    cd /tmp && rm -rf oni_v2_m22 && mkdir oni_v2_m22 && cd oni_v2_m22
    cat > employees.csv << 'CSVEOF'
name,department,salary
Alice,Engineering,90000
Bob,Marketing,70000
Charlie,Engineering,95000
Diana,Marketing,75000
Eve,Engineering,88000
Frank,Sales,65000
CSVEOF
    run_task "M22" "csv-sql" "MEDIUM" \
        "Read employees.csv. Create csv_query.py that implements a mini SQL engine for CSV. Run: SELECT department, AVG(salary) FROM employees.csv GROUP BY department ORDER BY AVG(salary) DESC. Print results." \
        "cd /tmp/oni_v2_m22 && python3 csv_query.py 2>&1 | grep -q 'Engineering'" \
        "medium" "10" "$suite_mode"

    # ───────────────────────────────────────────────────────────────
    # HARD TASKS (15)
    # ───────────────────────────────────────────────────────────────

    # H1: cancel-async-tasks — Async task runner with cancellation
    cd /tmp && rm -rf oni_v2_h1 && mkdir oni_v2_h1 && cd oni_v2_h1
    run_task "H1" "cancel-async-tasks" "HARD" \
        "Create async_runner.py: async task runner that runs tasks concurrently, supports cancellation by ID, properly cleans up on cancellation, returns results. Test: start 5 tasks (0.1s sleep), cancel 2, verify 3 complete. Print results." \
        "cd /tmp/oni_v2_h1 && $TIMEOUT_CMD 30 python3 async_runner.py 2>&1 | grep -qiE '3 completed|3 results|passed|OK'" \
        "medium" "12" "$suite_mode"

    # H2: fix-code-vulnerability — Fix 3 CWE vulnerabilities
    cd /tmp && rm -rf oni_v2_h2 && mkdir oni_v2_h2 && cd oni_v2_h2
    cat > server.py << 'VULN'
import os, sqlite3, subprocess
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        params = parse_qs(urlparse(self.path).query)
        if 'user' in params:
            conn = sqlite3.connect('app.db')
            result = conn.execute(f"SELECT * FROM users WHERE name='{params['user'][0]}'").fetchall()
            self.send_response(200)
            self.end_headers()
            self.wfile.write(str(result).encode())
        elif 'cmd' in params:
            output = os.popen(params['cmd'][0]).read()
            self.send_response(200)
            self.end_headers()
            self.wfile.write(output.encode())
        elif 'file' in params:
            with open(params['file'][0]) as f:
                self.send_response(200)
                self.end_headers()
                self.wfile.write(f.read().encode())

if __name__ == '__main__':
    HTTPServer(('', 8080), Handler).serve_forever()
VULN
    run_task "H2" "fix-code-vulnerability" "HARD" \
        "Read server.py. Fix 3 vulnerabilities: SQL injection (CWE-89), command injection (CWE-78), path traversal (CWE-22). Write fixed version. Create vulnerabilities.txt listing findings." \
        "cd /tmp/oni_v2_h2 && ! grep -q \"f\\\"SELECT\" server.py && ! grep -q 'os.popen' server.py && grep -qE 'parameteriz|sanitiz|realpath|abspath' server.py" \
        "medium" "12" "$suite_mode"

    # H3: make-mips-interpreter
    cd /tmp && rm -rf oni_v2_h3 && mkdir oni_v2_h3 && cd oni_v2_h3
    run_task "H3" "make-mips-interpreter" "HARD" \
        "Create mips.py — a MIPS R2000 interpreter handling: ADD, SUB, AND, OR, SLT, LW, SW, BEQ, J, ADDI, LI, SYSCALL (print_int=1, exit=10). Include test program computing fibonacci(10)=55 via syscall." \
        "cd /tmp/oni_v2_h3 && python3 mips.py 2>&1 | grep -q '55'" \
        "medium" "15" "$suite_mode"

    # H4: feal-differential-cryptanalysis
    cd /tmp && rm -rf oni_v2_h4 && mkdir oni_v2_h4 && cd oni_v2_h4
    run_task "H4" "feal-differential-cryptanalysis" "HARD" \
        "Implement FEAL-4 block cipher in feal4.py. Then implement a differential cryptanalysis attack that recovers subkey bits using chosen plaintext pairs. Demonstrate distinguishing the cipher from random with ~20 plaintexts." \
        "cd /tmp/oni_v2_h4 && python3 feal4.py 2>&1 | grep -qiE 'key|recover|attack|success|subkey'" \
        "medium" "15" "$suite_mode"

    # H5: circuit-fibsqrt
    cd /tmp && rm -rf oni_v2_h5 && mkdir oni_v2_h5 && cd oni_v2_h5
    run_task "H5" "circuit-fibsqrt" "HARD" \
        "Create circuit.py: compute fib(isqrt(N)) mod 2^32. isqrt is integer square root. For N=100: isqrt=10, fib(10)=55. For N=256: isqrt=16, fib(16)=987. Test both values." \
        "cd /tmp/oni_v2_h5 && python3 circuit.py 2>&1 | grep -q '55' && python3 circuit.py 2>&1 | grep -q '987'" \
        "medium" "10" "$suite_mode"

    # H6: regex-chess — Chess notation validator
    cd /tmp && rm -rf oni_v2_h6 && mkdir oni_v2_h6 && cd oni_v2_h6
    run_task "H6" "regex-chess" "HARD" \
        "Create chess_regex.py with regex matching valid algebraic chess notation: pawn moves (e4), pieces (Nf3), captures (Bxe5), castling (O-O, O-O-O), promotions (e8=Q), check (+), checkmate (#). Test 10 valid + 5 invalid examples." \
        "cd /tmp/oni_v2_h6 && python3 chess_regex.py 2>&1 | grep -qiE 'pass|all.*valid|OK|correct'" \
        "medium" "10" "$suite_mode"

    # H7: fix-gc — Fix garbage collector mark bug
    cd /tmp && rm -rf oni_v2_h7 && mkdir oni_v2_h7 && cd oni_v2_h7
    cat > gc_sim.py << 'GCEOF'
"""Simulated mark-and-sweep garbage collector with a bug."""
class GC:
    def __init__(self):
        self.heap = {}
        self.roots = set()
        self.next_id = 0

    def alloc(self, data, refs=None):
        obj_id = self.next_id
        self.next_id += 1
        self.heap[obj_id] = (data, refs or [])
        return obj_id

    def add_root(self, obj_id):
        self.roots.add(obj_id)

    def remove_root(self, obj_id):
        self.roots.discard(obj_id)

    def mark(self):
        marked = set()
        stack = list(self.roots)
        while stack:
            obj_id = stack.pop()
            if obj_id in marked:
                continue
            marked.add(obj_id)
            if obj_id in self.heap:
                _, refs = self.heap[obj_id]
                stack.append(refs[0] if refs else -1)  # BUG: only first ref
        return marked

    def sweep(self, marked):
        to_delete = [k for k in self.heap if k not in marked]
        for k in to_delete:
            del self.heap[k]
        return len(to_delete)

    def collect(self):
        marked = self.mark()
        return self.sweep(marked)

gc = GC()
a = gc.alloc("root_obj")
b = gc.alloc("child_1")
c = gc.alloc("child_2")
d = gc.alloc("orphan")
gc.heap[a] = ("root_obj", [b, c])
gc.add_root(a)
freed = gc.collect()
assert b in gc.heap, "child_1 should survive"
assert c in gc.heap, f"child_2 should survive but was collected!"
assert d not in gc.heap, "orphan should be collected"
print(f"GC OK: freed {freed} objects")
GCEOF
    run_task "H7" "fix-gc" "HARD" \
        "Read gc_sim.py. It has a bug in mark() — only follows first reference, not all. Fix it so all reachable objects survive. The test at bottom should pass." \
        "cd /tmp/oni_v2_h7 && python3 gc_sim.py 2>&1 | grep -q 'GC OK'" \
        "medium" "10" "$suite_mode"

    # H8: regex-engine — Build a regex engine from scratch
    cd /tmp && rm -rf oni_v2_h8 && mkdir oni_v2_h8 && cd oni_v2_h8
    run_task "H8" "regex-engine" "HARD" \
        "Create regex_engine.py implementing a basic regex engine from scratch (no 're' module). Support: literal chars, . (any), * (zero or more), + (one or more), ? (zero or one), | (alternation), () grouping. Test against: 'a.b' matches 'axb', 'a*' matches 'aaa', '(ab|cd)+' matches 'ababcd'. Print PASS/FAIL for each test." \
        "cd /tmp/oni_v2_h8 && python3 regex_engine.py 2>&1 | grep -c 'PASS' | xargs test 3 -le" \
        "medium" "15" "$suite_mode"

    # H9: huffman-coding — Compress and decompress
    cd /tmp && rm -rf oni_v2_h9 && mkdir oni_v2_h9 && cd oni_v2_h9
    cat > original.txt << 'HEOF'
AAAAABBBBBBBBBCCCCCCCCCCCCDDDDDDDDDDDDDEEEEEEEEEEEEEEEEFFFFFFFFFFFFFFFFFFFFFFFFG
HEOF
    run_task "H9" "huffman-coding" "HARD" \
        "Read original.txt. Create huffman.py implementing Huffman coding: 1) Build frequency table, 2) Build Huffman tree, 3) Generate codes, 4) Encode the text to binary string, 5) Decode back to original, 6) Verify roundtrip matches. Print the codes table and compression ratio." \
        "cd /tmp/oni_v2_h9 && python3 huffman.py 2>&1 | grep -qiE 'ratio|compress|match|verified|roundtrip'" \
        "medium" "10" "$suite_mode"

    # H10: minimax-tictactoe — Unbeatable AI
    cd /tmp && rm -rf oni_v2_h10 && mkdir oni_v2_h10 && cd oni_v2_h10
    run_task "H10" "minimax-tictactoe" "HARD" \
        "Create tictactoe.py with minimax + alpha-beta pruning AI for tic-tac-toe. The AI should be unbeatable. Test by playing all 9 possible first moves against it — it should never lose (all draws or AI wins). Print results." \
        "cd /tmp/oni_v2_h10 && python3 tictactoe.py 2>&1 | grep -qiE 'unbeat|never.*los|all.*draw|PASS|perfect'" \
        "medium" "12" "$suite_mode"

    # H11: json-parser — From scratch, no json module
    cd /tmp && rm -rf oni_v2_h11 && mkdir oni_v2_h11 && cd oni_v2_h11
    run_task "H11" "json-parser" "HARD" \
        "Create json_parser.py implementing a JSON parser from scratch (no 'import json'). Support: strings, numbers, booleans, null, arrays, objects. Parse '{\"name\":\"Alice\",\"age\":30,\"scores\":[10,20,30],\"active\":true}' and print the parsed dict. Verify key types." \
        "cd /tmp/oni_v2_h11 && python3 json_parser.py 2>&1 | grep -qE 'Alice|name'" \
        "medium" "12" "$suite_mode"

    # H12: forth-interpreter — Minimal Forth interpreter
    cd /tmp && rm -rf oni_v2_h12 && mkdir oni_v2_h12 && cd oni_v2_h12
    run_task "H12" "forth-interpreter" "HARD" \
        "Create forth.py — a minimal Forth interpreter. Support: stack ops (DUP DROP SWAP OVER ROT), arithmetic (+, -, *, /), comparison (= < >), output (. CR), word definitions (: word ... ;), IF ELSE THEN conditionals. Test: ': fib DUP 2 < IF DROP 1 ELSE DUP 1 - fib SWAP 2 - fib + THEN ; 10 fib .' should print 89." \
        "cd /tmp/oni_v2_h12 && python3 forth.py 2>&1 | grep -q '89'" \
        "medium" "15" "$suite_mode"

    # H13: xz-backdoor-analysis — Technical writeup
    cd /tmp && rm -rf oni_v2_h13 && mkdir oni_v2_h13 && cd oni_v2_h13
    run_task "H13" "xz-exploit-analysis" "HARD" \
        "Write analysis.md explaining the XZ Utils backdoor (CVE-2024-3094). Cover: 1) Attack vector, 2) How backdoor was injected into build system, 3) What payload did (SSH auth bypass), 4) How discovered, 5) Affected versions. Be technically specific." \
        "cd /tmp/oni_v2_h13 && wc -w analysis.md | awk '{print \$1}' | xargs test 200 -lt" \
        "medium" "10" "$suite_mode"

    # H14: bloom-filter — Probabilistic data structure
    cd /tmp && rm -rf oni_v2_h14 && mkdir oni_v2_h14 && cd oni_v2_h14
    run_task "H14" "bloom-filter" "HARD" \
        "Create bloom_filter.py implementing a Bloom filter with configurable false positive rate. Constructor takes expected_items and fp_rate. Use multiple hash functions (k). Methods: add(item), contains(item). Test: add 1000 items, check all are found (zero false negatives), then test 1000 non-members and report the false positive rate (should be close to configured rate). Print results." \
        "cd /tmp/oni_v2_h14 && python3 bloom_filter.py 2>&1 | grep -qiE 'false.positive|rate|bloom|PASS'" \
        "medium" "12" "$suite_mode"

    # H15: lru-cache-concurrent — Thread-safe LRU cache
    cd /tmp && rm -rf oni_v2_h15 && mkdir oni_v2_h15 && cd oni_v2_h15
    run_task "H15" "lru-cache-concurrent" "HARD" \
        "Create lru_cache.py implementing a thread-safe LRU cache with fixed capacity. Use a doubly-linked list + dict (O(1) get/put). Test with 4 threads doing 1000 ops each concurrently. Verify: eviction works, thread safety holds (no crashes/data corruption), access order is maintained. Print results." \
        "cd /tmp/oni_v2_h15 && python3 lru_cache.py 2>&1 | grep -qiE 'pass|thread.*safe|OK|correct|evict'" \
        "medium" "12" "$suite_mode"

    # ───────────────────────────────────────────────────────────────
    # SUITE RESULTS
    # ───────────────────────────────────────────────────────────────
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  SUITE COMPLETE [mode: $suite_mode]"
    echo "  RESULTS: $PASS/$TOTAL PASSED ($FAIL FAILED)"
    echo "  SCORE: $(( PASS * 100 / TOTAL ))%"
    echo "  FINISHED: $(date)"
    echo "═══════════════════════════════════════════════════════════════"

    echo "${suite_mode}|${PASS}|${FAIL}|${TOTAL}" >> "$RESULTS_DIR/ablation_modes.txt"
}

# ═══════════════════════════════════════════════════════════════
# TELEMETRY SUMMARY GENERATOR
# ═══════════════════════════════════════════════════════════════

generate_summary() {
    local results_dir="$1"
    local summary_json="${results_dir}/summary.json"

    python3 - "$results_dir" "$summary_json" << 'PYEOF'
import sys
import json
import os
import csv
import re
from pathlib import Path

results_dir = Path(sys.argv[1])
summary_json = Path(sys.argv[2])
csv_file = results_dir / "summary.csv"

if not csv_file.exists():
    print("No summary.csv found, skipping.")
    sys.exit(0)

rows = list(csv.DictReader(open(csv_file), delimiter="|"))

total_pass = sum(1 for r in rows if r["Result"] == "PASS")
total_fail = sum(1 for r in rows if r["Result"] == "FAIL")

# Tokens
token_values = []
for r in rows:
    tok = r.get("Tokens", "?")
    nums = re.findall(r"\d+", str(tok))
    if nums:
        token_values.append(int(nums[-1]))
avg_tokens = int(sum(token_values) / len(token_values)) if token_values else None

# Times
time_values = []
for r in rows:
    t = r.get("Time", "0s").rstrip("s")
    try:
        time_values.append(float(t))
    except ValueError:
        pass
avg_time = round(sum(time_values) / len(time_values), 1) if time_values else None
total_time_min = round(sum(time_values) / 60, 1) if time_values else None

# CapFlags
cap_flags = {}
for r in rows:
    flag = r.get("CapFlag", "UNKNOWN")
    cap_flags[flag] = cap_flags.get(flag, 0) + 1

# Per-difficulty
diff_stats = {}
for r in rows:
    diff = r.get("Difficulty", "UNKNOWN")
    if diff not in diff_stats:
        diff_stats[diff] = {"pass": 0, "fail": 0, "total": 0}
    diff_stats[diff]["total"] += 1
    if r["Result"] == "PASS":
        diff_stats[diff]["pass"] += 1
    else:
        diff_stats[diff]["fail"] += 1

# Per-mode
mode_stats = {}
for r in rows:
    mode = r.get("Mode", "full")
    if mode not in mode_stats:
        mode_stats[mode] = {"pass": 0, "fail": 0, "total": 0}
    mode_stats[mode]["total"] += 1
    if r["Result"] == "PASS":
        mode_stats[mode]["pass"] += 1
    else:
        mode_stats[mode]["fail"] += 1

# Per-task cross-mode comparison (the ablation matrix)
task_matrix = {}
for r in rows:
    tid = r["ID"]
    mode = r.get("Mode", "full")
    if tid not in task_matrix:
        task_matrix[tid] = {"name": r.get("Task", tid), "difficulty": r.get("Difficulty", "?")}
    task_matrix[tid][mode] = r["Result"]

# Telemetry aggregation
feature_counts = {}
for mode_dir in results_dir.iterdir():
    if not mode_dir.is_dir():
        continue
    for task_dir in mode_dir.iterdir():
        tel_file = task_dir / "telemetry.json"
        if tel_file.exists():
            try:
                tel = json.loads(tel_file.read_text())
                features = tel.get("features_activated", {})
                for feat, count in features.items():
                    feature_counts[feat] = feature_counts.get(feat, 0) + (count if isinstance(count, int) else 1)
            except (json.JSONDecodeError, KeyError):
                pass

summary = {
    "total_pass": total_pass,
    "total_fail": total_fail,
    "total_tasks": total_pass + total_fail,
    "pass_rate_pct": round(total_pass * 100 / (total_pass + total_fail), 1) if (total_pass + total_fail) else 0,
    "avg_tokens_per_task": avg_tokens,
    "avg_time_per_task_s": avg_time,
    "total_time_min": total_time_min,
    "capability_flags": cap_flags,
    "per_difficulty": diff_stats,
    "per_mode": mode_stats,
    "task_matrix": task_matrix,
    "feature_activation_counts": feature_counts,
}

summary_json.write_text(json.dumps(summary, indent=2))
print(f"Summary written to {summary_json}")
PYEOF
}

# ═══════════════════════════════════════════════════════════════
# REPORT GENERATOR
# ═══════════════════════════════════════════════════════════════

generate_report() {
    local results_dir="$1"
    local report_file="${results_dir}/../OVERNIGHT_REPORT_V2.md"

    python3 - "$results_dir" "$report_file" << 'PYEOF'
import sys
import json
import csv
from pathlib import Path
from datetime import datetime

results_dir = Path(sys.argv[1])
report_file = Path(sys.argv[2])
summary_json = results_dir / "summary.json"
csv_file = results_dir / "summary.csv"

if not summary_json.exists():
    print("No summary.json, cannot generate report.")
    sys.exit(0)

summary = json.loads(summary_json.read_text())
rows = list(csv.DictReader(open(csv_file), delimiter="|"))

# Build report
lines = []
lines.append("# ONI Overnight Benchmark Report v2")
lines.append("")
lines.append(f"**Date:** {datetime.now().strftime('%Y-%m-%d')}")
lines.append("**Hardware:** Apple M4 Max, 128GB unified memory")
lines.append("**Models:** Qwen3.5-27B-UD-Q8_K_XL (MIMIR) / Qwen3-Coder-Next-UD-Q6_K_XL (FENRIR) / GLM-4.7-Flash-UD-Q8_K_XL (SKULD)")
lines.append(f"**Total runs:** {summary['total_tasks']}")
lines.append(f"**Total time:** {summary.get('total_time_min', '?')} minutes")
lines.append("")
lines.append("---")
lines.append("")

# Executive Summary
lines.append("## Executive Summary")
lines.append("")
per_mode = summary.get("per_mode", {})
mode_table = []
for mode in ["full", "no-orchestrator", "no-kg", "lean", "ultra-lean"]:
    if mode in per_mode:
        m = per_mode[mode]
        rate = round(m["pass"] * 100 / m["total"], 1) if m["total"] else 0
        mode_table.append((mode, m["pass"], m["total"], rate))

if mode_table:
    baseline = mode_table[0][3] if mode_table else 0
    lines.append("| Mode | Pass/Total | Rate | Delta vs Full |")
    lines.append("|------|-----------|------|---------------|")
    for mode, p, t, rate in mode_table:
        delta = rate - baseline
        delta_str = f"+{delta:.1f}%" if delta > 0 else f"{delta:.1f}%" if delta < 0 else "baseline"
        lines.append(f"| {mode} | {p}/{t} | **{rate:.1f}%** | {delta_str} |")
    lines.append("")

# Best config
if mode_table:
    best = max(mode_table, key=lambda x: x[3])
    lines.append(f"**Best configuration:** `{best[0]}` at {best[3]:.1f}% ({best[1]}/{best[2]})")
    lines.append("")

lines.append("---")
lines.append("")

# Per-difficulty
lines.append("## Per-Difficulty Breakdown (all modes combined)")
lines.append("")
per_diff = summary.get("per_difficulty", {})
lines.append("| Difficulty | Pass | Fail | Total | Rate |")
lines.append("|-----------|------|------|-------|------|")
for diff in ["EASY", "MEDIUM", "HARD"]:
    if diff in per_diff:
        d = per_diff[diff]
        rate = round(d["pass"] * 100 / d["total"], 1) if d["total"] else 0
        lines.append(f"| {diff} | {d['pass']} | {d['fail']} | {d['total']} | {rate:.1f}% |")
lines.append("")

# Capability flags
lines.append("## Capability Flag Distribution")
lines.append("")
cap_flags = summary.get("capability_flags", {})
lines.append("| Flag | Count | % |")
lines.append("|------|-------|---|")
total = sum(cap_flags.values()) if cap_flags else 1
for flag, count in sorted(cap_flags.items(), key=lambda x: -x[1]):
    pct = round(count * 100 / total, 1)
    lines.append(f"| {flag} | {count} | {pct}% |")
lines.append("")

lines.append("---")
lines.append("")

# Ablation Matrix — tasks that flip between modes
lines.append("## Ablation Matrix — Key Task Flips")
lines.append("")
lines.append("Tasks where results changed across modes (most informative for feature impact):")
lines.append("")

task_matrix = summary.get("task_matrix", {})
modes_present = ["full", "no-orchestrator", "no-kg", "lean", "ultra-lean"]
modes_present = [m for m in modes_present if m in per_mode]

if task_matrix and len(modes_present) > 1:
    # Find tasks that flipped
    flipped_tasks = []
    for tid, data in task_matrix.items():
        results = [data.get(m, "?") for m in modes_present]
        if len(set(r for r in results if r != "?")) > 1:
            flipped_tasks.append((tid, data))

    if flipped_tasks:
        header = "| Task | " + " | ".join(modes_present) + " | Analysis |"
        sep = "|------|" + "|".join(["------"] * len(modes_present)) + "|----------|"
        lines.append(header)
        lines.append(sep)
        for tid, data in sorted(flipped_tasks, key=lambda x: x[0]):
            results = [data.get(m, "?") for m in modes_present]
            # Quick analysis
            if data.get("full") == "FAIL" and data.get("no-orchestrator") == "PASS":
                analysis = "Orchestrator hurts"
            elif data.get("full") == "FAIL" and data.get("no-kg") == "PASS":
                analysis = "KG hurts"
            elif data.get("full") == "FAIL" and data.get("lean") == "PASS":
                analysis = "Overhead kills it"
            elif data.get("full") == "PASS" and data.get("no-orchestrator") == "FAIL":
                analysis = "Orchestrator helps"
            else:
                analysis = "Mixed"
            result_strs = ["**PASS**" if r == "PASS" else "FAIL" if r == "FAIL" else "?" for r in results]
            lines.append(f"| {tid} {data.get('name','')} | " + " | ".join(result_strs) + f" | {analysis} |")
        lines.append("")
    else:
        lines.append("No tasks flipped between modes — all consistent.")
        lines.append("")

# Consistently failing tasks
lines.append("## Consistently Failing Tasks")
lines.append("")
lines.append("Tasks that failed across ALL modes (model capability ceiling):")
lines.append("")
always_fail = []
for tid, data in task_matrix.items():
    results = [data.get(m, "?") for m in modes_present]
    if all(r == "FAIL" for r in results if r != "?"):
        always_fail.append((tid, data))
if always_fail:
    for tid, data in sorted(always_fail, key=lambda x: x[0]):
        lines.append(f"- **{tid}** {data.get('name','')} ({data.get('difficulty','')})")
else:
    lines.append("None — all tasks passed in at least one mode.")
lines.append("")

# Stats
lines.append("---")
lines.append("")
lines.append("## Aggregate Statistics")
lines.append("")
lines.append(f"- **Average tokens per task:** {summary.get('avg_tokens_per_task', '?')}")
lines.append(f"- **Average time per task:** {summary.get('avg_time_per_task_s', '?')}s")
lines.append(f"- **Total inference time:** {summary.get('total_time_min', '?')} minutes")
lines.append("")

# Comparison with previous benchmark
lines.append("## Comparison with Previous Benchmark (2026-03-19)")
lines.append("")
lines.append("| Metric | Previous | Current |")
lines.append("|--------|----------|---------|")
lines.append("| Tasks | 27 | 42 |")
lines.append("| Models | qwen3.5:35b / qwen3-coder:30b / glm-4.7-flash | Qwen3.5-27B-UD-Q8_K_XL / Qwen3-Coder-Next-UD-Q6_K_XL / GLM-4.7-Flash-UD-Q8_K_XL |")
full_prev = "51%"
full_now = f"{per_mode.get('full', {}).get('pass', '?')}/{per_mode.get('full', {}).get('total', '?')}"
no_orch_prev = "81%"
no_orch_now = f"{per_mode.get('no-orchestrator', {}).get('pass', '?')}/{per_mode.get('no-orchestrator', {}).get('total', '?')}"
lines.append(f"| Full mode | {full_prev} | {full_now} |")
lines.append(f"| No-orchestrator | {no_orch_prev} | {no_orch_now} |")
lines.append("")

lines.append("---")
lines.append("")
lines.append("## Recommendations")
lines.append("")
lines.append("*(To be filled based on results)*")
lines.append("")

lines.append("---")
lines.append("")
lines.append(f"## Raw Data")
lines.append("")
lines.append(f"- Summary CSV: `{results_dir}/summary.csv`")
lines.append(f"- Summary JSON: `{results_dir}/summary.json`")
lines.append(f"- Per-task results: `{results_dir}/<mode>/<task_id>/`")
lines.append(f"  - `debug.log` — stderr from ONI")
lines.append(f"  - `output.txt` — stdout from ONI")
lines.append(f"  - `telemetry.json` — deep telemetry data")
lines.append("")

report_file.write_text("\n".join(lines))
print(f"Report written to {report_file}")
PYEOF
}

# ═══════════════════════════════════════════════════════════════
# ENTRY POINT
# ═══════════════════════════════════════════════════════════════

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  ONI OVERNIGHT BENCHMARK v2                                ║"
echo "║  Terminal-Bench 2.0 Expanded Suite                         ║"
echo "║  42 tasks × 5 configurations = 210 runs                   ║"
echo "╠══════════════════════════════════════════════════════════════╣"
echo "║  Models:                                                   ║"
echo "║    MIMIR  — Qwen3.5-27B-UD-Q8_K_XL (Heavy)               ║"
echo "║    FENRIR — Qwen3-Coder-Next-UD-Q6_K_XL (Medium)         ║"
echo "║    SKULD  — GLM-4.7-Flash-UD-Q8_K_XL (General)           ║"
echo "║  Configs: full, no-orchestrator, no-kg, lean, ultra-lean  ║"
echo "║  Started: $(date)                                          "
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# ═══════════════════════════════════════════════════════════════
# SERVER MANAGEMENT
# Heavy + Medium + General together OOM on 128GB with full configs.
# Strategy: run non-Heavy modes first, then reconfigure for full mode.
# ═══════════════════════════════════════════════════════════════

MODELS_DIR="$HOME/.cache/llama.cpp/models"
LLAMA_SERVER="$(which llama-server 2>/dev/null || echo /opt/homebrew/bin/llama-server)"

stop_all_servers() {
    pkill -f "llama-server.*--port 808[1-5]" 2>/dev/null || true
    sleep 3
}

start_lean_servers() {
    echo "Starting lean servers (Medium + General only)..."
    stop_all_servers

    # Medium (FENRIR) — full config, no competitors
    $LLAMA_SERVER \
        --model "$MODELS_DIR/UD-Q6_K_XL/Qwen3-Coder-Next-UD-Q6_K_XL-00001-of-00003.gguf" \
        --port 8082 --flash-attn on --ctx-size 65536 \
        --cache-type-k q4_0 --cache-type-v q4_0 \
        --n-gpu-layers 99 --threads 8 --threads-batch 16 \
        --batch-size 512 --ubatch-size 256 --parallel 1 --jinja \
        > /tmp/oni-medium.log 2>&1 &
    echo "  Medium (FENRIR) → :8082 [PID $!]"

    # General (SKULD) — for critic in no-kg mode
    $LLAMA_SERVER \
        --model "$MODELS_DIR/GLM-4.7-Flash-UD-Q8_K_XL.gguf" \
        --port 8083 --flash-attn on --ctx-size 16384 \
        --cache-type-k q4_0 --cache-type-v q4_0 \
        --n-gpu-layers 99 --threads 8 --threads-batch 16 \
        --batch-size 512 --ubatch-size 256 --parallel 1 --jinja \
        > /tmp/oni-general.log 2>&1 &
    echo "  General (SKULD) → :8083 [PID $!]"

    echo "  Waiting for servers to load..."
    sleep 45
    wait_for_server 8082 180
    wait_for_server 8083 120
}

start_full_servers() {
    echo "Starting full servers (all three, reduced configs)..."
    stop_all_servers

    # Heavy (MIMIR) — reduced ctx for memory fit
    $LLAMA_SERVER \
        --model "$MODELS_DIR/Qwen3.5-27B-UD-Q8_K_XL.gguf" \
        --port 8081 --flash-attn on --ctx-size 8192 \
        --cache-type-k q4_0 --cache-type-v q4_0 \
        --n-gpu-layers 99 --threads 8 --threads-batch 16 \
        --batch-size 256 --ubatch-size 256 --parallel 1 --jinja \
        --reasoning-format deepseek \
        > /tmp/oni-heavy.log 2>&1 &
    echo "  Heavy (MIMIR) → :8081 [PID $!]"

    # Medium (FENRIR) — reduced ctx to fit alongside Heavy
    $LLAMA_SERVER \
        --model "$MODELS_DIR/UD-Q6_K_XL/Qwen3-Coder-Next-UD-Q6_K_XL-00001-of-00003.gguf" \
        --port 8082 --flash-attn on --ctx-size 32768 \
        --cache-type-k q4_0 --cache-type-v q4_0 \
        --n-gpu-layers 99 --threads 8 --threads-batch 16 \
        --batch-size 512 --ubatch-size 256 --parallel 1 --jinja \
        > /tmp/oni-medium.log 2>&1 &
    echo "  Medium (FENRIR) → :8082 [PID $!]"

    # General (SKULD) — reduced ctx
    $LLAMA_SERVER \
        --model "$MODELS_DIR/GLM-4.7-Flash-UD-Q8_K_XL.gguf" \
        --port 8083 --flash-attn on --ctx-size 8192 \
        --cache-type-k q4_0 --cache-type-v q4_0 \
        --n-gpu-layers 99 --threads 8 --threads-batch 16 \
        --batch-size 256 --ubatch-size 256 --parallel 1 --jinja \
        > /tmp/oni-general.log 2>&1 &
    echo "  General (SKULD) → :8083 [PID $!]"

    echo "  Waiting for servers to load (Heavy is 33GB, may take ~60s)..."
    sleep 45
    wait_for_server 8081 120
    wait_for_server 8082 60
    wait_for_server 8083 60
}

wait_for_server() {
    local port="$1"
    local timeout_s="${2:-60}"
    local elapsed=0
    # Wait for health endpoint first
    while ! curl -s --max-time 2 "http://localhost:$port/health" >/dev/null 2>&1; do
        sleep 5
        elapsed=$((elapsed + 5))
        if [[ $elapsed -ge $timeout_s ]]; then
            echo "  WARNING: :$port health not ready after ${timeout_s}s"
            return 1
        fi
    done
    # Then verify model is actually loaded (not 503 "Loading model")
    while true; do
        local resp
        resp=$(curl -s --max-time 15 "http://localhost:$port/v1/chat/completions" \
            -H "Content-Type: application/json" \
            -d '{"model":"test","messages":[{"role":"user","content":"hi"}],"max_tokens":1}' 2>&1)
        if echo "$resp" | grep -q '"choices"'; then
            echo "  :$port — OK (model loaded, inference verified)"
            return 0
        fi
        sleep 10
        elapsed=$((elapsed + 10))
        if [[ $elapsed -ge $timeout_s ]]; then
            echo "  WARNING: :$port model not loaded after ${timeout_s}s"
            echo "  Last response: $resp"
            return 1
        fi
    done
}

# CSV header
echo "ID|Task|Difficulty|Result|Time|Tokens|CapFlag|Mode|Tier" > "$RESULTS_DIR/summary.csv"

# Phase 1: Run non-orchestrator modes (no Heavy server needed)
echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  PHASE 1: Non-orchestrator modes (4 configs × 42 tasks)   ║"
echo "╚══════════════════════════════════════════════════════════════╝"
start_lean_servers

for mode in "no-orchestrator" "no-kg" "lean" "ultra-lean"; do
    run_suite "$mode"
done

# Phase 2: Run full mode with all three servers
echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  PHASE 2: Full mode (orchestrator, all 3 servers)         ║"
echo "╚══════════════════════════════════════════════════════════════╝"
start_full_servers

# Verify Heavy is actually working before running full suite
if curl -s --max-time 10 "http://localhost:8081/v1/chat/completions" \
    -H "Content-Type: application/json" \
    -d '{"model":"test","messages":[{"role":"user","content":"hi"}],"max_tokens":5}' \
    2>&1 | grep -q '"choices"'; then
    echo "Heavy server verified — running full mode"
    run_suite "full"
else
    echo "WARNING: Heavy server not responding. Skipping full mode."
    echo "full|SKIP|SKIP|0" >> "$RESULTS_DIR/ablation_modes.txt"
fi

echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  ALL SUITES COMPLETE                                       ║"
echo "║  Finished: $(date)                                          "
echo "╠══════════════════════════════════════════════════════════════╣"

if [[ -f "$RESULTS_DIR/ablation_modes.txt" ]]; then
    while IFS="|" read -r m p f t; do
        pct=$(( p * 100 / t ))
        printf "║  %-20s %d/%d (%d%%)\n" "$m" "$p" "$t" "$pct"
    done < "$RESULTS_DIR/ablation_modes.txt"
fi

echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Generate telemetry summary
generate_summary "$RESULTS_DIR"

# Generate markdown report
generate_report "$RESULTS_DIR"

echo ""
echo "  Results:  $RESULTS_DIR/"
echo "  Report:   $RESULTS_DIR/../OVERNIGHT_REPORT_V2.md"
echo "  CSV:      $RESULTS_DIR/summary.csv"
echo "  JSON:     $RESULTS_DIR/summary.json"
echo ""
echo "Good night! ☽"
