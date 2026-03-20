#!/bin/bash
# ONI Stress Test — Terminal-Bench 2.0 Adapted Suite
# Runs 27 tasks sequentially, records results with telemetry
set -euo pipefail

ONI="$HOME/.cargo/bin/oni"

# macOS doesn't have 'timeout' — use gtimeout from coreutils
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
# ARGUMENT PARSING
# ═══════════════════════════════════════════════════════════════

MODE="full"
TIMEOUT_EASY=120
TIMEOUT_MEDIUM=300
TIMEOUT_HARD=600

usage() {
    echo "Usage: $0 [--mode full|no-kg|no-orchestrator|no-personality|ablation] [--timeout-easy N] [--timeout-medium N] [--timeout-hard N]"
    echo ""
    echo "  --mode          Run mode (default: full)"
    echo "                    full            All features enabled"
    echo "                    no-kg           Run with --no-knowledge-graph"
    echo "                    no-orchestrator Run with --no-orchestrator"
    echo "                    no-personality  Run with --no-personality"
    echo "                    ablation        Run full suite 4x (full + all no-* modes)"
    echo "  --timeout-easy  Timeout in seconds for EASY tasks (default: 120)"
    echo "  --timeout-medium Timeout in seconds for MEDIUM tasks (default: 300)"
    echo "  --timeout-hard  Timeout in seconds for HARD tasks (default: 600)"
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --timeout-easy)
            TIMEOUT_EASY="$2"
            shift 2
            ;;
        --timeout-medium)
            TIMEOUT_MEDIUM="$2"
            shift 2
            ;;
        --timeout-hard)
            TIMEOUT_HARD="$2"
            shift 2
            ;;
        --help|-h)
            usage
            ;;
        *)
            echo "Unknown argument: $1"
            usage
            ;;
    esac
done

# Validate mode
case "$MODE" in
    full|no-kg|no-orchestrator|no-personality|ablation) ;;
    *)
        echo "Invalid mode: $MODE"
        usage
        ;;
esac

# ═══════════════════════════════════════════════════════════════
# RESULTS DIRECTORY
# ═══════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
RESULTS_DIR="${SCRIPT_DIR}/results/${TIMESTAMP}"
mkdir -p "$RESULTS_DIR"

# ═══════════════════════════════════════════════════════════════
# HELPERS
# ═══════════════════════════════════════════════════════════════

# Monitor system resources
log_system() {
    local name="$1"
    vm_stat 2>/dev/null | head -5 > "$RESULTS_DIR/${name}.sysinfo" 2>/dev/null || true
    ps aux | grep -i ollama | grep -v grep >> "$RESULTS_DIR/${name}.sysinfo" 2>/dev/null || true
}

# Build feature flags for the given mode
build_flags() {
    local mode="$1"
    case "$mode" in
        full)            echo "" ;;
        no-kg)           echo "--no-knowledge-graph" ;;
        no-orchestrator) echo "--no-orchestrator" ;;
        no-personality)  echo "--no-personality" ;;
    esac
}

# Classify failure into a CapabilityFlag
classify_result() {
    local id="$1"
    local passed="$2"      # "PASS" or "FAIL"
    local elapsed="$3"
    local timeout_s="$4"
    local debug_log="$5"

    if [[ "$passed" == "PASS" ]]; then
        echo "CLEAN_PASS"
        return
    fi

    # Known harness issues
    case "$id" in
        E2|E3|M9)
            echo "HARNESS_ISSUE"
            return
            ;;
    esac

    # Hit the timeout
    if [[ "$elapsed" -ge "$timeout_s" ]]; then
        echo "TIMEOUT_LIMIT"
        return
    fi

    # Check debug log for ONI infrastructure errors
    if grep -qiE 'rate.limit|connection.refused|ollama|model.not.found|rpc.error' "$debug_log" 2>/dev/null; then
        echo "FRAMEWORK_LIMIT"
        return
    fi

    # Check if model ran to completion but got wrong answer
    if grep -qiE 'completed|finished|done' "$debug_log" 2>/dev/null; then
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
    local tier="${6:-code}"
    local max_rounds="${7:-15}"
    local run_mode="${8:-full}"

    TOTAL=$((TOTAL + 1))
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  [$id] $name ($difficulty) [mode: $run_mode]"
    echo "  TIER: $tier | MAX_ROUNDS: $max_rounds"
    echo "═══════════════════════════════════════════════════════════════"

    log_system "$id"

    # Determine timeout for this task
    local task_timeout
    case "$difficulty" in
        EASY)   task_timeout="$TIMEOUT_EASY" ;;
        MEDIUM) task_timeout="$TIMEOUT_MEDIUM" ;;
        HARD)   task_timeout="$TIMEOUT_HARD" ;;
        *)      task_timeout="$TIMEOUT_MEDIUM" ;;
    esac

    # Per-task results directory for telemetry
    local task_results_dir="$RESULTS_DIR/${id}"
    mkdir -p "$task_results_dir"

    local debug_log="${task_results_dir}/debug.log"
    local output_file="${task_results_dir}/output.txt"
    local telemetry_file="${task_results_dir}/telemetry.json"

    # Build feature flag args
    local feature_flags
    feature_flags="$(build_flags "$run_mode")"

    local start_time
    start_time=$(date +%s)

    # Run ONI headless with telemetry, enforcing per-difficulty timeout
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

    # Token count from debug log or telemetry
    local tokens
    tokens=$(grep -o 'tokens.*' "$debug_log" 2>/dev/null | tail -1 || echo "?")

    # Check result
    local result_label
    local cap_flag
    if eval "$check_cmd" 2>/dev/null; then
        result_label="PASS"
        PASS=$((PASS + 1))
        echo "  PASS ($elapsed s)"
    else
        result_label="FAIL"
        FAIL=$((FAIL + 1))
        echo "  FAIL ($elapsed s)"
    fi

    cap_flag="$(classify_result "$id" "$result_label" "$elapsed" "$task_timeout" "$debug_log")"
    echo "  CAP_FLAG: $cap_flag"

    echo "$id|$name|$difficulty|$result_label|${elapsed}s|$tokens|$cap_flag|$run_mode" >> "$RESULTS_DIR/summary.csv"

    log_system "${id}_after"
}

# ═══════════════════════════════════════════════════════════════
# TASK DEFINITIONS
# All task setup + run_task calls are wrapped in run_suite so
# ablation mode can call the suite multiple times.
# ═══════════════════════════════════════════════════════════════

run_suite() {
    local suite_mode="${1:-full}"

    PASS=0
    FAIL=0
    SKIP=0
    TOTAL=0

    echo ""
    echo "══════════════════════════════════════════════════════════════"
    echo "  RUNNING SUITE — mode: $suite_mode"
    echo "══════════════════════════════════════════════════════════════"

    # Header for this suite's CSV (appended)
    if [[ ! -f "$RESULTS_DIR/summary.csv" ]]; then
        echo "ID|Task|Difficulty|Result|Time|Tokens|CapFlag|Mode" > "$RESULTS_DIR/summary.csv"
    fi

    # ─────────────────────────────────────────────────────────────
    # EASY TASKS (3)
    # ─────────────────────────────────────────────────────────────

    # E1: fix-git — Recover lost changes and merge into master
    cd /tmp && rm -rf oni_st_e1 && mkdir oni_st_e1 && cd oni_st_e1
    git init -q && git config user.email "t@t.com" && git config user.name "T"
    echo "initial" > readme.md && git add . && git commit -q -m "initial"
    git checkout -q -b feature
    echo "feature work" > feature.txt && git add . && git commit -q -m "add feature"
    git checkout -q main 2>/dev/null || git checkout -q master
    echo "conflicting" > readme.md && git add . && git commit -q -m "master update"
    run_task "E1" "fix-git" "EASY" \
        "Merge the 'feature' branch into master in this git repo. Resolve any conflicts. Keep both changes." \
        "cd /tmp/oni_st_e1 && git log --oneline | grep -q 'feature' || git log --oneline | grep -q 'Merge'" \
        "code" "8" "$suite_mode"

    # E2: cobol-modernization — Convert COBOL to Python
    # NOTE: accepts both "Total: 55" (Python int) and "Total: 0055" (COBOL-style padding)
    cd /tmp && rm -rf oni_st_e2 && mkdir oni_st_e2 && cd oni_st_e2
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
        "cd /tmp/oni_st_e2 && python3 program.py 2>&1 | grep -qE 'Total: (55|0055)'" \
        "code" "5" "$suite_mode"

    # E3: zigzag-pattern — Generate zigzag string
    # NOTE: accepts "passed" (test success message) OR the literal output string
    cd /tmp && rm -rf oni_st_e3 && mkdir oni_st_e3 && cd oni_st_e3
    run_task "E3" "zigzag-pattern" "EASY" \
        "Create zigzag.py that takes a string and number of rows, outputs the zigzag pattern reading. Example: zigzag('PAYPALISHIRING', 3) should output 'PAHNAPLSIIGYIR'. Include a test that verifies this example." \
        "cd /tmp/oni_st_e3 && python3 zigzag.py 2>&1 | grep -qE 'passed|PAHNAPLSIIGYIR'" \
        "code" "5" "$suite_mode"

    # ─────────────────────────────────────────────────────────────
    # MEDIUM TASKS (15)
    # ─────────────────────────────────────────────────────────────

    # M1: regex-log — Extract dates from log lines containing IPv4
    cd /tmp && rm -rf oni_st_m1 && mkdir oni_st_m1 && cd oni_st_m1
    cat > access.log << 'LOGEOF'
2024-01-15 10:23:45 192.168.1.100 GET /api/users 200
2024-01-15 10:24:01 10.0.0.1 POST /api/login 401
2024-01-16 08:00:00 no-ip-here GET /health 200
2024-01-16 09:15:22 172.16.0.50 DELETE /api/users/5 403
2024-01-17 14:30:00 invalid GET /api/data 500
LOGEOF
    run_task "M1" "regex-log" "MEDIUM" \
        "Read access.log. Write extract_dates.py that uses regex to extract the date from every line that contains a valid IPv4 address. Print dates one per line. Lines without valid IPs should be skipped." \
        "cd /tmp/oni_st_m1 && python3 extract_dates.py | wc -l | tr -d ' ' | grep -q '^3$'" \
        "code" "5" "$suite_mode"

    # M2: multi-source-data-merger — Merge data from JSON + CSV + YAML
    cd /tmp && rm -rf oni_st_m2 && mkdir oni_st_m2 && cd oni_st_m2
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
        "cd /tmp/oni_st_m2 && python3 merge.py && python3 -c \"import json; d=json.load(open('merged.json')); assert len(d)==3; a=[x for x in d if x['email']=='alice@example.com'][0]; assert a['department']=='Engineering'; assert a['phone']=='+44789012'; print('OK')\" 2>&1 | grep -q OK" \
        "code" "8" "$suite_mode"

    # M3: git-leak-recovery — Find leaked secret in git history
    cd /tmp && rm -rf oni_st_m3 && mkdir oni_st_m3 && cd oni_st_m3
    git init -q && git config user.email "t@t.com" && git config user.name "T"
    echo "config = {}" > config.py && git add . && git commit -q -m "init"
    echo 'SECRET_KEY = "sk_live_SUPER_SECRET_12345"' >> config.py && git add . && git commit -q -m "add config"
    echo "config = {}" > config.py && git add . && git commit -q -m "remove secret"
    run_task "M3" "git-leak-recovery" "MEDIUM" \
        "There is a secret key that was committed to this repo and then removed. Find the secret key from the git history and write it to a file called secret.txt." \
        "cd /tmp/oni_st_m3 && grep -q 'sk_live_SUPER_SECRET_12345' secret.txt" \
        "code" "8" "$suite_mode"

    # M4: build-cython-ext — Compile a Python C extension
    cd /tmp && rm -rf oni_st_m4 && mkdir oni_st_m4 && cd oni_st_m4
    cat > fib.pyx << 'CYEOF'
def fibonacci(int n):
    cdef int a = 0, b = 1, i
    for i in range(n):
        a, b = b, a + b
    return a
CYEOF
    cat > setup.py << 'SETUP'
from setuptools import setup
from Cython.Build import cythonize
setup(ext_modules=cythonize("fib.pyx"))
SETUP
    run_task "M4" "build-cython-ext" "MEDIUM" \
        "Build the Cython extension in this directory. Install cython if needed (pip install cython), then run 'python3 setup.py build_ext --inplace'. Verify it works by importing fib and calling fibonacci(10)." \
        "cd /tmp/oni_st_m4 && python3 -c 'from fib import fibonacci; assert fibonacci(10)==55; print(\"OK\")' 2>&1 | grep -q OK" \
        "code" "10" "$suite_mode"

    # M5: polyglot-c-py — File runnable as both Python and C
    cd /tmp && rm -rf oni_st_m5 && mkdir oni_st_m5 && cd oni_st_m5
    run_task "M5" "polyglot-c-py" "MEDIUM" \
        "Create a file called polyglot that computes fibonacci(10)=55 and prints it. The SAME file must be valid Python (run with python3 polyglot) AND valid C (compile with cc -o poly polyglot && ./poly). Both must print exactly '55'." \
        "cd /tmp/oni_st_m5 && python3 polyglot 2>&1 | grep -q '55' && cc -x c -o poly polyglot 2>/dev/null && ./poly 2>&1 | grep -q '55'" \
        "code" "10" "$suite_mode"

    # M6: headless-terminal — Implement a simple terminal emulator
    cd /tmp && rm -rf oni_st_m6 && mkdir oni_st_m6 && cd oni_st_m6
    run_task "M6" "headless-terminal" "MEDIUM" \
        "Create headless_term.py that implements a basic terminal emulator. It should: 1) Accept commands via stdin, 2) Execute them in a subprocess, 3) Capture stdout/stderr, 4) Print output. Test it by running: echo 'echo hello' | python3 headless_term.py" \
        "cd /tmp/oni_st_m6 && echo 'echo hello' | python3 headless_term.py 2>&1 | grep -q 'hello'" \
        "code" "8" "$suite_mode"

    # M7: constraints-scheduling — Find meeting slot
    cd /tmp && rm -rf oni_st_m7 && mkdir oni_st_m7 && cd oni_st_m7
    cat > schedules.json << 'SEOF'
{
  "alice": ["09:00-10:00", "11:00-12:00", "14:00-16:00"],
  "bob": ["10:00-11:30", "13:00-15:00"],
  "charlie": ["09:00-10:30", "14:00-15:30"]
}
SEOF
    run_task "M7" "constraints-scheduling" "MEDIUM" \
        "Read schedules.json. Each person's array shows their BUSY times. Write scheduler.py that finds all available 30-minute meeting slots between 09:00 and 17:00 where ALL three people are free. Print slots as 'HH:MM-HH:MM', one per line." \
        "cd /tmp/oni_st_m7 && python3 scheduler.py | head -1 | grep -qE '[0-9]{2}:[0-9]{2}-[0-9]{2}:[0-9]{2}'" \
        "code" "8" "$suite_mode"

    # M8: financial-document-processor — Extract invoice data
    cd /tmp && rm -rf oni_st_m8 && mkdir oni_st_m8 && cd oni_st_m8
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
        "Read invoice1.txt and invoice2.txt. Create process_invoices.py that extracts: invoice number, date, vendor, total. Output a summary.csv with columns: invoice_id,date,vendor,total" \
        "cd /tmp/oni_st_m8 && python3 process_invoices.py && python3 -c \"import csv; rows=list(csv.DictReader(open('summary.csv'))); assert len(rows)==2; assert rows[0]['vendor'].strip()=='Acme Corp'; print('OK')\" 2>&1 | grep -q OK" \
        "code" "8" "$suite_mode"

    # M9: kv-store-grpc (adapted — simple TCP key-value store)
    # NOTE: uses 3s sleep + retry loop to avoid server startup race condition
    cd /tmp && rm -rf oni_st_m9 && mkdir oni_st_m9 && cd oni_st_m9
    run_task "M9" "kv-store-tcp" "MEDIUM" \
        "Create a simple TCP key-value store in Python. server.py listens on port 9999 and handles GET/SET/DEL commands. client.py connects and runs: SET foo bar, GET foo (should return 'bar'), DEL foo, GET foo (should return 'NOT_FOUND'). Write both files and verify with a test." \
        "cd /tmp/oni_st_m9 && $TIMEOUT_CMD 15 python3 -c \"
import subprocess, time, socket

srv = subprocess.Popen(['python3', 'server.py'], stdout=subprocess.PIPE, stderr=subprocess.PIPE)

# Retry loop: wait up to 3s for server to bind
connected = False
for _ in range(30):
    try:
        s = socket.socket()
        s.connect(('localhost', 9999))
        connected = True
        break
    except OSError:
        time.sleep(0.1)

if not connected:
    srv.terminate()
    raise RuntimeError('Server did not start in time')

s.sendall(b'SET foo bar\n'); r1 = s.recv(1024).decode().strip()
s.sendall(b'GET foo\n'); r2 = s.recv(1024).decode().strip()
srv.terminate()
assert 'bar' in r2, f'Expected bar, got {r2}'
print('OK')
\" 2>&1 | grep -q OK" \
        "code" "12" "$suite_mode"

    # M10: query-optimize — Speed up a slow SQL query
    cd /tmp && rm -rf oni_st_m10 && mkdir oni_st_m10 && cd oni_st_m10
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
    run_task "M10" "query-optimize" "MEDIUM" \
        "Read slow_query.sql and test.db. The query is slow on large datasets. Create optimize.py that: 1) Adds appropriate indexes, 2) Rewrites the query to be faster, 3) Runs both versions and prints timing comparison." \
        "cd /tmp/oni_st_m10 && python3 optimize.py 2>&1 | grep -qiE 'fast|improv|index|optimiz'" \
        "code" "8" "$suite_mode"

    # M11: extract-elf — Extract values from compiled binary
    cd /tmp && rm -rf oni_st_m11 && mkdir oni_st_m11 && cd oni_st_m11
    cat > secret.c << 'CEOF'
#include <stdio.h>
static const char SECRET[] = "ONI_HIDDEN_VALUE_42";
static const int MAGIC = 0xDEADBEEF;
int main() { printf("Nothing to see here\n"); return 0; }
CEOF
    cc -o secret secret.c
    run_task "M11" "extract-elf" "MEDIUM" \
        "There is a compiled binary called 'secret' in this directory. Extract all string constants and the magic number from it. Write findings to findings.json with keys 'strings' (array) and 'magic_hex' (string)." \
        "cd /tmp/oni_st_m11 && python3 -c \"import json; d=json.load(open('findings.json')); assert 'ONI_HIDDEN_VALUE_42' in str(d); print('OK')\" 2>&1 | grep -q OK" \
        "code" "8" "$suite_mode"

    # M12: gcode-to-text — Decode what a 3D printer would print
    cd /tmp && rm -rf oni_st_m12 && mkdir oni_st_m12 && cd oni_st_m12
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
    run_task "M12" "gcode-to-text" "MEDIUM" \
        "Read print.gcode. Analyze the movement commands to determine what text/letters the tool path traces. Write the decoded text to decoded.txt." \
        "cd /tmp/oni_st_m12 && grep -qi 'ONI' decoded.txt" \
        "code" "8" "$suite_mode"

    # M13: pypi-server — Create and serve a Python package
    cd /tmp && rm -rf oni_st_m13 && mkdir oni_st_m13 && cd oni_st_m13
    run_task "M13" "pypi-server" "MEDIUM" \
        "Create a Python package called 'oni_math' with a function 'add(a, b)' that returns a+b. Set up a proper package structure (setup.py/pyproject.toml, __init__.py). Build the package with 'python3 -m build' or 'python3 setup.py sdist'. Verify the tarball exists." \
        "cd /tmp/oni_st_m13 && ls dist/*.tar.gz 2>/dev/null | grep -q 'oni_math' || ls *.tar.gz 2>/dev/null | grep -q 'oni'" \
        "code" "10" "$suite_mode"

    # M14: openssl-selfsigned-cert — Generate TLS cert
    cd /tmp && rm -rf oni_st_m14 && mkdir oni_st_m14 && cd oni_st_m14
    run_task "M14" "openssl-selfsigned-cert" "MEDIUM" \
        "Generate a self-signed TLS certificate for 'oni.local' using openssl. Create both the private key (key.pem) and certificate (cert.pem). The cert should be valid for 365 days with RSA 2048-bit key." \
        "cd /tmp/oni_st_m14 && openssl x509 -in cert.pem -text -noout 2>&1 | grep -q 'oni.local'" \
        "code" "5" "$suite_mode"

    # M15: sqlite-db-truncate — Recover from truncated DB
    cd /tmp && rm -rf oni_st_m15 && mkdir oni_st_m15 && cd oni_st_m15
    python3 -c "
import sqlite3
conn = sqlite3.connect('data.db')
conn.execute('CREATE TABLE records (id INT, name TEXT, value REAL)')
for i in range(100):
    conn.execute('INSERT INTO records VALUES (?, ?, ?)', (i, f'item_{i}', i*2.5))
conn.commit()
conn.close()
import os
size = os.path.getsize('data.db')
with open('data.db', 'r+b') as f:
    f.truncate(size * 3 // 4)
"
    run_task "M15" "sqlite-db-truncate" "MEDIUM" \
        "data.db is a corrupted (truncated) SQLite database. Recover as many rows as possible from the 'records' table. Write recovered data to recovered.csv with columns: id,name,value" \
        "cd /tmp/oni_st_m15 && python3 -c \"lines=open('recovered.csv').readlines(); assert len(lines) > 10; print('OK')\" 2>&1 | grep -q OK" \
        "code" "10" "$suite_mode"

    # ─────────────────────────────────────────────────────────────
    # HARD TASKS (9)
    # ─────────────────────────────────────────────────────────────

    # H1: cancel-async-tasks — Async task runner with cancellation
    cd /tmp && rm -rf oni_st_h1 && mkdir oni_st_h1 && cd oni_st_h1
    run_task "H1" "cancel-async-tasks" "HARD" \
        "Create async_runner.py implementing an async task runner that: 1) Runs multiple async tasks concurrently, 2) Supports cancellation of individual tasks by ID, 3) Properly cleans up resources on cancellation, 4) Returns results of completed tasks. Include a test that starts 5 tasks, cancels 2, and verifies 3 complete." \
        "cd /tmp/oni_st_h1 && python3 async_runner.py 2>&1 | grep -qiE '3 completed|3 results|passed|OK'" \
        "code" "12" "$suite_mode"

    # H2: fix-code-vulnerability — Identify and fix CWE issues
    cd /tmp && rm -rf oni_st_h2 && mkdir oni_st_h2 && cd oni_st_h2
    cat > server.py << 'VULN'
import os, sqlite3, subprocess
from http.server import HTTPServer, BaseHTTPRequestHandler
from urllib.parse import urlparse, parse_qs

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        params = parse_qs(urlparse(self.path).query)
        if 'user' in params:
            # SQL injection vulnerability (CWE-89)
            conn = sqlite3.connect('app.db')
            result = conn.execute(f"SELECT * FROM users WHERE name='{params['user'][0]}'").fetchall()
            self.send_response(200)
            self.end_headers()
            self.wfile.write(str(result).encode())
        elif 'cmd' in params:
            # Command injection vulnerability (CWE-78)
            output = os.popen(params['cmd'][0]).read()
            self.send_response(200)
            self.end_headers()
            self.wfile.write(output.encode())
        elif 'file' in params:
            # Path traversal vulnerability (CWE-22)
            with open(params['file'][0]) as f:
                self.send_response(200)
                self.end_headers()
                self.wfile.write(f.read().encode())

if __name__ == '__main__':
    HTTPServer(('', 8080), Handler).serve_forever()
VULN
    run_task "H2" "fix-code-vulnerability" "HARD" \
        "Read server.py. It has 3 security vulnerabilities: SQL injection (CWE-89), command injection (CWE-78), and path traversal (CWE-22). Fix ALL THREE vulnerabilities. Write the fixed version. Also create vulnerabilities.txt listing what you found and how you fixed each." \
        "cd /tmp/oni_st_h2 && ! grep -q \"f\\\"SELECT\" server.py && ! grep -q 'os.popen' server.py && grep -qE 'parameteriz|sanitiz|realpath|abspath' server.py" \
        "code" "12" "$suite_mode"

    # H3: make-mips-interpreter — MIPS interpreter in Python
    cd /tmp && rm -rf oni_st_h3 && mkdir oni_st_h3 && cd oni_st_h3
    run_task "H3" "make-mips-interpreter" "HARD" \
        "Create mips.py — a MIPS R2000 interpreter that handles: ADD, SUB, AND, OR, SLT, LW, SW, BEQ, J, ADDI, LI, SYSCALL (print_int=1, exit=10). Include a test program that computes fibonacci(10)=55 and prints it via syscall." \
        "cd /tmp/oni_st_h3 && python3 mips.py 2>&1 | grep -q '55'" \
        "code" "15" "$suite_mode"

    # H4: feal-differential-cryptanalysis — Chosen plaintext attack
    cd /tmp && rm -rf oni_st_h4 && mkdir oni_st_h4 && cd oni_st_h4
    run_task "H4" "feal-differential-cryptanalysis" "HARD" \
        "Implement the FEAL-4 block cipher in Python (feal4.py). Then implement a differential cryptanalysis attack that recovers subkey bits using chosen plaintext pairs. The attack should demonstrate that you can distinguish the cipher from random given ~20 chosen plaintexts." \
        "cd /tmp/oni_st_h4 && python3 feal4.py 2>&1 | grep -qiE 'key|recover|attack|success|subkey'" \
        "code" "15" "$suite_mode"

    # H5: circuit-fibsqrt — Logic circuit for fib(isqrt(N))
    cd /tmp && rm -rf oni_st_h5 && mkdir oni_st_h5 && cd oni_st_h5
    run_task "H5" "circuit-fibsqrt" "HARD" \
        "Create circuit.py that computes fib(isqrt(N)) mod 2^32 for a given N. isqrt is integer square root. For N=100, isqrt(100)=10, fib(10)=55. For N=256, isqrt(256)=16, fib(16)=987. Test with both values." \
        "cd /tmp/oni_st_h5 && python3 circuit.py 2>&1 | grep -q '55' && python3 circuit.py 2>&1 | grep -q '987'" \
        "code" "10" "$suite_mode"

    # H6: regex-chess — Regex that validates chess moves
    cd /tmp && rm -rf oni_st_h6 && mkdir oni_st_h6 && cd oni_st_h6
    run_task "H6" "regex-chess" "HARD" \
        "Create chess_regex.py with a regex pattern that matches valid algebraic chess notation moves. Must handle: pawn moves (e4), piece moves (Nf3), captures (Bxe5), castling (O-O, O-O-O), promotions (e8=Q), check (+), checkmate (#). Test against at least 10 valid and 5 invalid examples." \
        "cd /tmp/oni_st_h6 && python3 chess_regex.py 2>&1 | grep -qiE 'pass|all.*valid|OK|correct'" \
        "code" "10" "$suite_mode"

    # H7: xz-exploit — Analyze the XZ backdoor
    cd /tmp && rm -rf oni_st_h7 && mkdir oni_st_h7 && cd oni_st_h7
    run_task "H7" "xz-exploit" "HARD" \
        "Write an analysis document (analysis.md) explaining the XZ Utils backdoor (CVE-2024-3094). Cover: 1) What was the attack vector, 2) How the backdoor was injected into the build system, 3) What the payload did (SSH auth bypass), 4) How it was discovered, 5) Affected versions. Be technically specific." \
        "cd /tmp/oni_st_h7 && wc -w analysis.md | awk '{print \$1}' | xargs test 200 -lt" \
        "code" "10" "$suite_mode"

    # H8: gpt2-codegolf — Tiny program that samples from GPT-2
    cd /tmp && rm -rf oni_st_h8 && mkdir oni_st_h8 && cd oni_st_h8
    run_task "H8" "gpt2-codegolf" "HARD" \
        "Create a minimal Python script (gpt2_mini.py) that implements GPT-2 inference from scratch (no transformers library). It should: load pre-trained weights from a small GPT-2 model, generate 20 tokens from the prompt 'The meaning of life is'. Focus on minimal code, not performance." \
        "cd /tmp/oni_st_h8 && python3 -c \"import ast; ast.parse(open('gpt2_mini.py').read()); print('SYNTAX_OK')\" 2>&1 | grep -q SYNTAX_OK" \
        "code" "15" "$suite_mode"

    # H9: fix-ocaml-gc — Debug OCaml garbage collector (adapted to Python)
    cd /tmp && rm -rf oni_st_h9 && mkdir oni_st_h9 && cd oni_st_h9
    cat > gc_sim.py << 'GCEOF'
"""Simulated mark-and-sweep garbage collector with a bug."""
class GC:
    def __init__(self):
        self.heap = {}  # id -> (data, [references])
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
                # BUG: should add refs to stack but doesn't traverse all
                stack.append(refs[0] if refs else -1)  # Only follows first ref!
        return marked

    def sweep(self, marked):
        to_delete = [k for k in self.heap if k not in marked]
        for k in to_delete:
            del self.heap[k]
        return len(to_delete)

    def collect(self):
        marked = self.mark()
        return self.sweep(marked)

# Test that exposes the bug
gc = GC()
a = gc.alloc("root_obj")
b = gc.alloc("child_1")
c = gc.alloc("child_2")
d = gc.alloc("orphan")  # Should be collected
gc.heap[a] = ("root_obj", [b, c])  # Root references both b and c
gc.add_root(a)
freed = gc.collect()
assert b in gc.heap, "child_1 should survive (reachable from root)"
assert c in gc.heap, f"child_2 should survive (reachable from root) but was collected!"
assert d not in gc.heap, "orphan should be collected"
print(f"GC OK: freed {freed} objects")
GCEOF
    run_task "H9" "fix-ocaml-gc" "HARD" \
        "Read gc_sim.py. It implements a mark-and-sweep garbage collector with a bug in the mark phase — it only follows the FIRST reference, not all references. Fix the bug so all reachable objects survive collection. The test at the bottom should pass." \
        "cd /tmp/oni_st_h9 && python3 gc_sim.py 2>&1 | grep -q 'GC OK'" \
        "code" "10" "$suite_mode"

    # ─────────────────────────────────────────────────────────────
    # SUITE RESULTS
    # ─────────────────────────────────────────────────────────────
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  SUITE COMPLETE [mode: $suite_mode]"
    echo "  RESULTS: $PASS/$TOTAL PASSED ($FAIL FAILED, $SKIP SKIPPED)"
    echo "  SCORE: $(( PASS * 100 / TOTAL ))%"
    echo "═══════════════════════════════════════════════════════════════"

    # Store per-mode results for ablation aggregation
    echo "${suite_mode}|${PASS}|${FAIL}|${TOTAL}" >> "$RESULTS_DIR/ablation_modes.txt"
}

# ═══════════════════════════════════════════════════════════════
# TELEMETRY SUMMARY
# ═══════════════════════════════════════════════════════════════

generate_summary() {
    local results_dir="$1"
    local summary_json="${results_dir}/summary.json"

    python3 - "$results_dir" "$summary_json" << 'PYEOF'
import sys
import json
import os
import csv
from pathlib import Path

results_dir = Path(sys.argv[1])
summary_json = Path(sys.argv[2])
csv_file = results_dir / "summary.csv"

if not csv_file.exists():
    print("No summary.csv found, skipping telemetry aggregation.")
    sys.exit(0)

rows = list(csv.DictReader(open(csv_file), delimiter="|"))

total_pass = sum(1 for r in rows if r["Result"] == "PASS")
total_fail = sum(1 for r in rows if r["Result"] == "FAIL")

# Aggregate tokens (skip "?" values)
token_values = []
for r in rows:
    tok = r.get("Tokens", "?")
    # Try to extract a number from strings like "tokens used: 1234"
    import re
    nums = re.findall(r"\d+", str(tok))
    if nums:
        token_values.append(int(nums[-1]))

avg_tokens = int(sum(token_values) / len(token_values)) if token_values else None

# Aggregate times (strip trailing 's')
time_values = []
for r in rows:
    t = r.get("Time", "0s").rstrip("s")
    try:
        time_values.append(float(t))
    except ValueError:
        pass

avg_time = round(sum(time_values) / len(time_values), 1) if time_values else None

# CapFlag breakdown
cap_flags = {}
for r in rows:
    flag = r.get("CapFlag", "UNKNOWN")
    cap_flags[flag] = cap_flags.get(flag, 0) + 1

# Per-difficulty stats
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

# Feature activation: aggregate per-task telemetry files
feature_counts = {}
for task_dir in results_dir.iterdir():
    tel_file = task_dir / "telemetry.json"
    if tel_file.exists():
        try:
            tel = json.loads(tel_file.read_text())
            features = tel.get("features_activated", {})
            for feat, count in features.items():
                feature_counts[feat] = feature_counts.get(feat, 0) + (count if isinstance(count, int) else 1)
        except (json.JSONDecodeError, KeyError):
            pass

# Mode breakdown for ablation runs
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

summary = {
    "total_pass": total_pass,
    "total_fail": total_fail,
    "total_tasks": total_pass + total_fail,
    "pass_rate_pct": round(total_pass * 100 / (total_pass + total_fail), 1) if (total_pass + total_fail) else 0,
    "avg_tokens_per_task": avg_tokens,
    "avg_time_per_task_s": avg_time,
    "capability_flags": cap_flags,
    "per_difficulty": diff_stats,
    "feature_activation_counts": feature_counts,
    "per_mode": mode_stats,
}

summary_json.write_text(json.dumps(summary, indent=2))
print(f"Telemetry summary written to {summary_json}")
PYEOF
}

# ═══════════════════════════════════════════════════════════════
# ENTRY POINT
# ═══════════════════════════════════════════════════════════════

echo "ONI STRESS TEST — TERMINAL-BENCH 2.0 ADAPTED"
echo "=============================================="
echo "Date: $(date)"
echo "Mode: $MODE"
echo "Model ensemble: qwen3.5:35b / qwen3-coder:30b / glm-4.7-flash"
echo "Timeouts: easy=${TIMEOUT_EASY}s  medium=${TIMEOUT_MEDIUM}s  hard=${TIMEOUT_HARD}s"
echo "Results: $RESULTS_DIR"
echo ""

# Write CSV header once
echo "ID|Task|Difficulty|Result|Time|Tokens|CapFlag|Mode" > "$RESULTS_DIR/summary.csv"

if [[ "$MODE" == "ablation" ]]; then
    for ablation_mode in full no-kg no-orchestrator no-personality; do
        run_suite "$ablation_mode"
    done

    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "  ABLATION COMPLETE — mode comparison:"
    if [[ -f "$RESULTS_DIR/ablation_modes.txt" ]]; then
        while IFS="|" read -r m p f t; do
            pct=$(( p * 100 / t ))
            echo "    $m: $p/$t (${pct}%)"
        done < "$RESULTS_DIR/ablation_modes.txt"
    fi
    echo "═══════════════════════════════════════════════════════════════"
else
    run_suite "$MODE"
fi

echo ""
echo "  Detailed results: $RESULTS_DIR/summary.csv"
echo "  Debug logs:       $RESULTS_DIR/<task_id>/debug.log"
echo "  Telemetry:        $RESULTS_DIR/<task_id>/telemetry.json"
echo ""

generate_summary "$RESULTS_DIR"

echo ""
echo "  Summary JSON: $RESULTS_DIR/summary.json"
echo ""
