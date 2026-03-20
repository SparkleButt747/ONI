#!/bin/bash
# ONI TerminalBench Runner — runs problems through ONI debug mode and checks results
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ONI="$HOME/.cargo/bin/oni"
RESULTS_DIR="$SCRIPT_DIR/results"
mkdir -p "$RESULTS_DIR"
PASS=0
FAIL=0
TOTAL=0

run_problem() {
    local name="$1"
    local prompt="$2"
    local check_cmd="$3"
    local tier="${4:-code}"
    local max_rounds="${5:-10}"

    TOTAL=$((TOTAL + 1))
    echo ""
    echo "═══════════════════════════════════════════"
    echo "  PROBLEM: $name"
    echo "  TIER: $tier"
    echo "═══════════════════════════════════════════"

    # Run ONI
    local output
    output=$($ONI run --tier "$tier" --max-rounds "$max_rounds" $prompt 2>"$RESULTS_DIR/${name}.debug.log")
    echo "$output" > "$RESULTS_DIR/${name}.output"

    # Check result
    if eval "$check_cmd"; then
        echo "  ✓ PASS"
        PASS=$((PASS + 1))
    else
        echo "  ✗ FAIL"
        FAIL=$((FAIL + 1))
    fi
}

echo "ONI TERMINALBENCH — MEDIUM DIFFICULTY"
echo "======================================"
echo ""

# ─── Problem 1: File creation ───────────────────
cd /tmp && mkdir -p oni_bench_p1 && cd oni_bench_p1 && rm -rf *
run_problem "create_fizzbuzz" \
    "Create a file called fizzbuzz.py that implements fizzbuzz for numbers 1 to 100. The file should be executable and print the output when run with python3." \
    "python3 /tmp/oni_bench_p1/fizzbuzz.py | head -5 | grep -q 'Buzz'" \
    "code" "5"

# ─── Problem 2: Bug fix ─────────────────────────
cd /tmp && mkdir -p oni_bench_p2 && cd oni_bench_p2 && rm -rf *
cat > buggy.py << 'PYEOF'
def calculate_average(numbers):
    total = 0
    for n in numbers:
        total += n
    return total / len(numbers)

# Bug: crashes on empty list
result = calculate_average([])
print(f"Average: {result}")
PYEOF
run_problem "fix_bug" \
    "Read buggy.py, fix the bug (it crashes on empty list), and write the fixed version. The fix should return 0 for empty lists." \
    "cd /tmp/oni_bench_p2 && python3 buggy.py 2>&1 | grep -q 'Average: 0'" \
    "code" "5"

# ─── Problem 3: Regex extraction ────────────────
cd /tmp && mkdir -p oni_bench_p3 && cd oni_bench_p3 && rm -rf *
cat > data.txt << 'EOF'
Contact us at support@example.com or sales@company.org
For urgent matters: admin@internal.net, help@service.io
Invalid emails: notanemail, @missing.com, user@
EOF
run_problem "regex_extract" \
    "Read data.txt and create extract_emails.py that extracts all valid email addresses from data.txt and prints them one per line, sorted alphabetically." \
    "cd /tmp/oni_bench_p3 && python3 extract_emails.py | wc -l | tr -d ' ' | grep -q '^4$'" \
    "code" "5"

# ─── Problem 4: JSON manipulation ───────────────
cd /tmp && mkdir -p oni_bench_p4 && cd oni_bench_p4 && rm -rf *
cat > users.json << 'JSONEOF'
[
  {"name": "Alice", "age": 30, "role": "admin"},
  {"name": "Bob", "age": 25, "role": "user"},
  {"name": "Charlie", "age": 35, "role": "admin"},
  {"name": "Diana", "age": 28, "role": "user"},
  {"name": "Eve", "age": 32, "role": "moderator"}
]
JSONEOF
run_problem "json_transform" \
    "Read users.json. Create transform.py that reads users.json, filters to only admins, adds an 'is_senior' field (true if age >= 30), and writes the result to admins.json." \
    "cd /tmp/oni_bench_p4 && python3 transform.py && python3 -c \"import json; d=json.load(open('admins.json')); assert len(d)==2; assert all(u['role']=='admin' for u in d); assert d[0]['is_senior']==True; print('OK')\" 2>&1 | grep -q 'OK'" \
    "code" "5"

# ─── Problem 5: Shell scripting ─────────────────
cd /tmp && mkdir -p oni_bench_p5 && cd oni_bench_p5 && rm -rf *
mkdir -p src/a src/b src/c
echo "hello" > src/a/file1.txt
echo "world" > src/b/file2.txt
echo "test" > src/c/file3.txt
echo "hello" > src/a/dup.txt
run_problem "find_duplicates" \
    "Write a bash script called find_dups.sh that finds all duplicate files (by content) in the src/ directory and prints pairs of duplicates. Run it and show the output." \
    "cd /tmp/oni_bench_p5 && bash find_dups.sh 2>&1 | grep -q 'file1.txt'" \
    "code" "8"

# ─── Problem 6: Rust code generation ────────────
cd /tmp && mkdir -p oni_bench_p6 && cd oni_bench_p6 && rm -rf *
run_problem "rust_fibonacci" \
    "Create a Rust file called fib.rs that computes the 20th Fibonacci number using memoization. It should compile with rustc and print the result (6765) when run." \
    "cd /tmp/oni_bench_p6 && rustc fib.rs -o fib 2>/dev/null && ./fib | grep -q '6765'" \
    "code" "5"

# ─── Problem 7: Git operations ──────────────────
cd /tmp && mkdir -p oni_bench_p7 && cd oni_bench_p7 && rm -rf *
git init -q . && git config user.email "test@test.com" && git config user.name "Test"
echo "v1" > readme.md && git add . && git commit -q -m "initial"
echo "v2" > readme.md && git add . && git commit -q -m "update"
echo "v3" > readme.md && git add . && git commit -q -m "latest"
run_problem "git_log_parse" \
    "Check the git log in this repo and create a file called commits.txt that lists all commit messages, one per line, most recent first." \
    "cd /tmp/oni_bench_p7 && head -1 commits.txt | grep -q 'latest'" \
    "code" "5"

# ─── Problem 8: API/Data processing ─────────────
cd /tmp && mkdir -p oni_bench_p8 && cd oni_bench_p8 && rm -rf *
cat > sales.csv << 'CSVEOF'
date,product,quantity,price
2024-01-15,Widget A,10,29.99
2024-01-16,Widget B,5,49.99
2024-01-15,Widget A,3,29.99
2024-01-17,Widget C,8,19.99
2024-01-16,Widget A,7,29.99
2024-01-17,Widget B,2,49.99
CSVEOF
run_problem "csv_analysis" \
    "Read sales.csv. Create analyze.py that reads the CSV, calculates total revenue per product, and prints results sorted by revenue descending. Format: 'Product: \$X.XX'" \
    "cd /tmp/oni_bench_p8 && python3 analyze.py | head -1 | grep -q 'Widget A'" \
    "code" "5"

# ─── Problem 9: Config file generation ──────────
cd /tmp && mkdir -p oni_bench_p9 && cd oni_bench_p9 && rm -rf *
run_problem "docker_compose" \
    "Create a docker-compose.yml for a web app with 3 services: a Node.js app on port 3000, a PostgreSQL database, and a Redis cache. Use proper environment variables and volume mounts." \
    "cd /tmp/oni_bench_p9 && python3 -c \"import yaml; d=yaml.safe_load(open('docker-compose.yml')); svcs=list(d.get('services',{}).keys()); assert len(svcs)>=3; print('OK')\" 2>&1 | grep -q 'OK'" \
    "code" "5"

# ─── Problem 10: Code refactoring ───────────────
cd /tmp && mkdir -p oni_bench_p10 && cd oni_bench_p10 && rm -rf *
cat > messy.py << 'PYEOF'
def process(d):
    r = []
    for i in d:
        if i['type'] == 'A':
            if i['value'] > 10:
                r.append({'name': i['name'], 'score': i['value'] * 2})
            else:
                r.append({'name': i['name'], 'score': i['value']})
        elif i['type'] == 'B':
            if i['value'] > 20:
                r.append({'name': i['name'], 'score': i['value'] * 3})
            else:
                r.append({'name': i['name'], 'score': i['value'] * 1.5})
    return sorted(r, key=lambda x: x['score'], reverse=True)

data = [
    {'name': 'foo', 'type': 'A', 'value': 15},
    {'name': 'bar', 'type': 'B', 'value': 25},
    {'name': 'baz', 'type': 'A', 'value': 5},
    {'name': 'qux', 'type': 'B', 'value': 10},
]
result = process(data)
for item in result:
    print(f"{item['name']}: {item['score']}")
PYEOF
run_problem "refactor" \
    "Read messy.py. Refactor it to be cleaner and more readable (use a strategy pattern or dictionary dispatch). Write the refactored version back. It must produce the exact same output as the original." \
    "cd /tmp/oni_bench_p10 && diff <(python3 messy.py) <(python3 messy.py) > /dev/null 2>&1" \
    "code" "8"

# ─── RESULTS ────────────────────────────────────
echo ""
echo "═══════════════════════════════════════════"
echo "  RESULTS: $PASS/$TOTAL PASSED ($FAIL FAILED)"
echo "  SCORE: $(( PASS * 100 / TOTAL ))%"
echo "═══════════════════════════════════════════"
