# ONI Codebase Audit Fixes Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 50 issues found in the ONI codebase audit — crashers, security holes, correctness bugs, performance, and dead code.

**Architecture:** Fixes are grouped into 10 tasks ordered by severity. Each task is independent and can be parallelised. All fixes are in existing files — no new files created except a shared `parsing.rs` module to deduplicate code between agent.rs and orchestrator.rs.

**Tech Stack:** Rust, tokio, rusqlite, ratatui, serde

**Build command:** `$HOME/.cargo/bin/cargo build`
**Test command:** `$HOME/.cargo/bin/cargo test`

---

## File Map

| File | Tasks | Primary Issues |
|------|-------|----------------|
| `crates/oni-agent/src/tools/get_url.rs` | 1 | block_on deadlock, URL validation |
| `crates/oni-agent/src/tools/ask_user.rs` | 1 | blocking recv on Tokio worker |
| `crates/oni-core/src/personality.rs` | 2 | decay math, UTF-8 panic, silent save failures |
| `crates/oni-agent/src/tools/read_file.rs` | 2 | UTF-8 truncation panic |
| `crates/oni-ollama/src/client.rs` | 2 | expect() panic on TLS |
| `crates/oni-agent/src/tools/bash.rs` | 3 | blocklist bypass, blocking Command |
| `crates/oni-agent/src/tools/edit_file.rs` | 3 | missing path safety |
| `crates/oni-agent/src/tools/write_file.rs` | 3 | weak path check |
| `crates/oni-agent/src/tools/search_files.rs` | 3 | grep flag injection |
| `crates/oni-agent/src/tools/forge_tool.rs` | 3 | no execution timeout |
| `crates/oni-tui/src/app.rs` | 3 | inline shell no blocklist |
| `crates/oni-agent/src/tools/mod.rs` | 4 | unknown tool returns Ok |
| `crates/oni-agent/src/agent.rs` | 4, 5 | operator precedence, strip_thinking |
| `crates/oni-agent/src/orchestrator.rs` | 5, 10 | replan from 0, dead fields, duplication |
| `crates/oni-agent/src/parsing.rs` | 5 | NEW — shared strip_thinking + extract_text_tool_call |
| `crates/oni-agent/src/conversation.rs` | 6 | token undercount, compaction placement |
| `crates/oni-agent/src/callbacks.rs` | 6 | deterministic "random" |
| `crates/oni-core/src/types.rs` | 7 | Heavy tier no tools |
| `crates/oni-core/src/config.rs` | 7 | config merge, dead default_tier |
| `crates/oni-agent/src/knowledge_graph.rs` | 8 | O(n²) search/gc |
| `crates/oni-agent/src/preferences.rs` | 8 | connection per op, SQL REPLACE bug |
| `crates/oni-agent/src/plan_store.rs` | 9 | global path collision |
| `crates/oni-agent/src/linter.rs` | 9 | workspace-wide lint |
| `crates/oni-agent/src/agent_defs.rs` | 9 | YAML delimiter parsing |
| `crates/oni-agent/src/telemetry.rs` | 10 | unused params |
| `crates/oni-agent/src/prompts.rs` | 10 | dead NORN |
| `crates/oni-agent/src/reflection.rs` | 10 | unused variable, silent write_soul |
| `crates/oni-agent/src/tools/undo.rs` | 10 | silent failures |

---

### Task 1: Fix async deadlocks (Critical — will crash in production)

**Findings:** #8 (get_url block_on), #9 (ask_user blocking recv), #37 (client.rs expect panic)

**Files:**
- Modify: `crates/oni-agent/src/tools/get_url.rs`
- Modify: `crates/oni-agent/src/tools/ask_user.rs`
- Modify: `crates/oni-agent/src/tools/mod.rs:18-23` (Tool trait)
- Modify: `crates/oni-ollama/src/client.rs`

**Context:** The `Tool` trait has a synchronous `execute()` method. Two tools (`get_url`, `ask_user`) need async I/O but are called from async contexts. `block_on` deadlocks on single-threaded Tokio; `std::sync::mpsc::recv` blocks the worker. The fix is `tokio::task::block_in_place` which moves the blocking call off the reactor thread.

- [ ] **Step 1: Fix get_url.rs — replace block_on with block_in_place**

```rust
// crates/oni-agent/src/tools/get_url.rs:72-73
// Replace:
//   let handle = tokio::runtime::Handle::current();
//   let result = handle.block_on(async move {
// With:
let result = tokio::task::block_in_place(|| {
    tokio::runtime::Handle::current().block_on(async move {
```

Also add URL scheme validation at the top of execute():
```rust
// After extracting url string, before the async block:
if !url.starts_with("http://") && !url.starts_with("https://") {
    return Ok(format!("Error: only http:// and https:// URLs are allowed, got: {}", url));
}
// Reject private IPs
let is_private = url.contains("://localhost") || url.contains("://127.0.0.1")
    || url.contains("://0.0.0.0") || url.contains("://169.254.")
    || url.contains("://10.") || url.contains("://192.168.");
if is_private {
    return Ok("Error: fetching private/internal network addresses is not allowed.".into());
}
```

- [ ] **Step 2: Fix ask_user.rs — wrap blocking recv in block_in_place**

```rust
// In ask_user.rs execute(), wrap the resp_rx.recv() call:
let response = tokio::task::block_in_place(|| resp_rx.recv());
```

- [ ] **Step 3: Fix client.rs — replace expect with error propagation**

```rust
// crates/oni-ollama/src/client.rs — OllamaClient::new
// Change .expect("Failed to build HTTP client") to:
// Make new() return Result<Self> or use .unwrap_or_else with a fallback client
let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(timeout_secs))
    .build()
    .unwrap_or_else(|_| reqwest::Client::new()); // Fallback to default client
```

- [ ] **Step 4: Build and test**

Run: `$HOME/.cargo/bin/cargo build && $HOME/.cargo/bin/cargo test`
Expected: Clean build, 181 tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/oni-agent/src/tools/get_url.rs crates/oni-agent/src/tools/ask_user.rs crates/oni-ollama/src/client.rs
git commit -m "fix: resolve async deadlocks in get_url and ask_user tools

- Replace block_on with block_in_place to prevent Tokio worker starvation
- Add URL scheme + private IP validation to get_url
- Replace expect() with fallback in OllamaClient::new"
```

---

### Task 2: Fix UTF-8 panics and personality decay math

**Findings:** #1 (decay formula 18x too aggressive), #2 (journal UTF-8 panic), #36 (read_file truncation panic), #38 (silent save failures)

**Files:**
- Modify: `crates/oni-core/src/personality.rs:176-202` (decay), `:496-514` (journal truncation), `:172,382` (save methods)
- Modify: `crates/oni-agent/src/tools/read_file.rs:44-47`
- Modify: `crates/oni-agent/src/agent.rs` (build_tool_summary byte slice)
- Modify: `crates/oni-agent/src/plan_store.rs` (summary byte slice)

- [ ] **Step 1: Fix the decay formula in personality.rs:176-202**

Replace the incorrect `ln().recip()` formulations with proper half-life decay:
```rust
pub fn apply_decay(&mut self) {
    let elapsed_hours = (now_secs() - self.last_updated) as f64 / 3600.0;
    if elapsed_hours < 0.1 {
        return;
    }

    let ln2 = std::f64::consts::LN_2;

    // Connection decays without interaction (half-life ~48h)
    self.connection *= (-elapsed_hours * ln2 / 48.0).exp().max(0.3);

    // Curiosity decays slowly (half-life ~72h)
    self.curiosity *= (-elapsed_hours * ln2 / 72.0).exp().max(0.2);

    // Frustration decays fast (half-life ~4h)
    self.frustration *= (-elapsed_hours * ln2 / 4.0).exp();

    // Confidence recovers slowly toward 0.7
    self.confidence += (0.7 - self.confidence) * (1.0 - (-elapsed_hours / 24.0).exp());

    // Boredom grows with time away (capped at 1.0)
    self.boredom = (self.boredom + elapsed_hours / 168.0).min(1.0);

    // Impatience decays (half-life ~8h)
    self.impatience *= (-elapsed_hours * ln2 / 8.0).exp();

    self.last_updated = now_secs();
    self.clamp();
}
```

- [ ] **Step 2: Fix all byte-index truncation panics**

Create a helper function and use it everywhere:
```rust
// In personality.rs, add near the top:
fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
```

Apply in personality.rs journal truncation (~line 502-514):
```rust
// Replace &yesterday[..500] with:
safe_truncate(&yesterday, 500)
// Replace &today[..500] with:
safe_truncate(&today, 500)
```

In `read_file.rs:44-47`:
```rust
// Replace &content[..100_000] with:
let end = {
    let mut e = 100_000.min(content.len());
    while e > 0 && !content.is_char_boundary(e) { e -= 1; }
    e
};
&content[..end]
```

In `agent.rs` `build_tool_summary` (~line 799):
```rust
// Replace &cmd[..60] with:
let end = {
    let mut e = 60.min(cmd.len());
    while e > 0 && !cmd.is_char_boundary(e) { e -= 1; }
    e
};
&cmd[..end]
```

In `plan_store.rs` `summary` (~line 143):
```rust
// Replace &self.task[..50] with same pattern
```

- [ ] **Step 3: Fix silent save failures — add tracing::warn**

In `EmotionalState::save()` and `RelationshipState::save()`:
```rust
// Replace: let _ = std::fs::write(...);
// With:
if let Err(e) = std::fs::write(&path, &json) {
    tracing::warn!("Failed to save {}: {}", path.display(), e);
}
```

- [ ] **Step 4: Build and test**

Run: `$HOME/.cargo/bin/cargo build && $HOME/.cargo/bin/cargo test`

- [ ] **Step 5: Commit**

```bash
git add crates/oni-core/src/personality.rs crates/oni-agent/src/tools/read_file.rs crates/oni-agent/src/agent.rs crates/oni-agent/src/plan_store.rs
git commit -m "fix: correct decay formula, fix UTF-8 truncation panics, log save failures

- Decay was 18x too aggressive (ln().recip() vs ln(2)/half_life)
- Byte-index string slicing panicked on multi-byte chars
- EmotionalState/RelationshipState save errors now logged"
```

---

### Task 3: Fix security holes in tools

**Findings:** #27 (bash blocklist bypass), #28 (write_file path check), #29 (edit_file no path safety), #30 (grep flag injection), #31 (get_url — done in Task 1), #32 (forge_tool no timeout), #14 (inline shell no blocklist)

**Files:**
- Modify: `crates/oni-agent/src/tools/bash.rs:5-21,64-72`
- Modify: `crates/oni-agent/src/tools/write_file.rs:7-21`
- Modify: `crates/oni-agent/src/tools/edit_file.rs:43-47`
- Modify: `crates/oni-agent/src/tools/search_files.rs:55-62`
- Modify: `crates/oni-agent/src/tools/forge_tool.rs`
- Modify: `crates/oni-tui/src/app.rs:332-356`

- [ ] **Step 1: Fix bash blocklist — normalise whitespace, case-insensitive**

```rust
// crates/oni-agent/src/tools/bash.rs — replace the blocklist check:
fn is_blocked(command: &str) -> bool {
    // Normalise: lowercase, collapse whitespace, strip leading/trailing
    let normalised: String = command
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    for pattern in BLOCKED_PATTERNS {
        if normalised.contains(pattern) {
            return true;
        }
    }
    false
}

// In execute(), replace:
//   for pattern in BLOCKED_PATTERNS { if command.contains(pattern) ...
// With:
if is_blocked(command) {
    return Ok("BLOCKED: Command matches security blocklist.".into());
}
```

- [ ] **Step 2: Fix write_file path safety — use canonicalisation**

```rust
// crates/oni-agent/src/tools/write_file.rs — replace is_safe_path:
fn is_safe_path(path: &str) -> bool {
    let p = std::path::Path::new(path);
    // Reject path traversal components
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return false;
        }
    }
    // Reject absolute paths outside cwd
    if p.is_absolute() {
        let cwd = match std::env::current_dir() {
            Ok(c) => c,
            Err(_) => return false, // Can't verify — reject
        };
        if !p.starts_with(&cwd) {
            return false;
        }
    }
    true
}
```

- [ ] **Step 3: Add path safety to edit_file**

```rust
// crates/oni-agent/src/tools/edit_file.rs — add at top of execute(), after extracting path:
// Reuse the same is_safe_path function (move to mod.rs or duplicate)
fn is_safe_path(path: &str) -> bool {
    let p = std::path::Path::new(path);
    for component in p.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return false;
        }
    }
    if p.is_absolute() {
        let cwd = match std::env::current_dir() {
            Ok(c) => c,
            Err(_) => return false,
        };
        if !p.starts_with(&cwd) {
            return false;
        }
    }
    true
}

// Add at top of execute() body, after extracting `path`:
if !is_safe_path(path) {
    return Ok(format!("Error: path '{}' is outside the project directory.", path));
}
```

- [ ] **Step 4: Fix grep flag injection — add `--` separator**

```rust
// crates/oni-agent/src/tools/search_files.rs:62
// Replace: cmd.arg(pattern).arg(search_path);
// With:
cmd.arg("--").arg(pattern).arg(search_path);
```

- [ ] **Step 5: Add execution timeout to forge_tool**

```rust
// In forge_tool.rs, where Command::new("bash") is called:
// Add timeout using std::process::Command + a thread:
use std::time::Duration;

let mut child = Command::new("bash")
    .arg(&script_path)
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()
    .map_err(|e| oni_core::error::err!("Failed to spawn: {}", e))?;

// 30-second timeout
let status = match child.wait_timeout(Duration::from_secs(30)) {
    // Note: wait_timeout is not in std. Use a thread instead:
    // Spawn a thread that waits, main thread sleeps then kills
};
// Alternative: use std::thread + child.kill():
let timeout = Duration::from_secs(30);
let start = std::time::Instant::now();
loop {
    match child.try_wait() {
        Ok(Some(status)) => break,
        Ok(None) => {
            if start.elapsed() > timeout {
                let _ = child.kill();
                return Ok("Error: script execution timed out after 30 seconds.".into());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err(e) => return Ok(format!("Error waiting for script: {}", e)),
    }
}
```

- [ ] **Step 6: Add blocklist to TUI inline shell**

```rust
// crates/oni-tui/src/app.rs:346 — before Command::new("bash"):
// Import or inline the bash tool's is_blocked function
let normalised: String = cmd.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ");
let blocked_patterns = ["rm -rf /", "rm -rf /*", "rm -rf ~", "mkfs", "dd if=",
    ":(){:|:&};:", "chmod -r 777 /", "sudo rm", "sudo dd", "sudo mkfs",
    "> /dev/sda", "curl | sh", "curl | bash", "wget | sh", "wget | bash"];
if blocked_patterns.iter().any(|p| normalised.contains(p)) {
    self.messages.push(DisplayMessage::System("BLOCKED: Command matches security blocklist.".into()));
    return true;
}
```

- [ ] **Step 7: Build and test**

Run: `$HOME/.cargo/bin/cargo build && $HOME/.cargo/bin/cargo test`

- [ ] **Step 8: Commit**

```bash
git add crates/oni-agent/src/tools/bash.rs crates/oni-agent/src/tools/write_file.rs \
  crates/oni-agent/src/tools/edit_file.rs crates/oni-agent/src/tools/search_files.rs \
  crates/oni-agent/src/tools/forge_tool.rs crates/oni-tui/src/app.rs
git commit -m "fix(security): harden tool blocklist, add path safety to edit_file, fix grep injection

- Bash blocklist now normalises whitespace and case before matching
- write_file uses component-based path traversal check instead of string contains
- edit_file now has same path safety as write_file
- grep pattern preceded by -- to prevent flag injection
- forge_tool scripts have 30s execution timeout
- TUI inline shell (:cmd) now checks same blocklist as bash tool"
```

---

### Task 4: Fix agent logic bugs

**Findings:** #3 (operator precedence), #4 (strip_thinking), #6 (unknown tool Ok), #12 (deterministic callbacks)

**Files:**
- Modify: `crates/oni-agent/src/agent.rs:330-340` (should_orchestrate)
- Modify: `crates/oni-agent/src/agent.rs:810-825` (strip_thinking)
- Modify: `crates/oni-agent/src/tools/mod.rs` (execute unknown tool)
- Modify: `crates/oni-agent/src/callbacks.rs`

- [ ] **Step 1: Fix operator precedence in should_orchestrate**

```rust
// crates/oni-agent/src/agent.rs ~line 334
// Replace:
let has_numbered_steps = lower.contains("1)") || lower.contains("1.")
    && (lower.contains("2)") || lower.contains("2."));
// With:
let has_numbered_steps = (lower.contains("1)") || lower.contains("1."))
    && (lower.contains("2)") || lower.contains("2."));
```

- [ ] **Step 2: Fix strip_thinking to search from correct offset**

```rust
// Both in agent.rs and orchestrator.rs — replace strip_thinking:
fn strip_thinking(content: &str) -> String {
    let mut result = content.to_string();
    loop {
        let Some(start) = result.find("<think>") else { break };
        if let Some(relative_end) = result[start..].find("</think>") {
            let end = start + relative_end;
            result = format!("{}{}", &result[..start], &result[end + 8..]);
        } else {
            // Incomplete thinking block — strip from <think> to end
            result = result[..start].to_string();
            break;
        }
    }
    result.trim().to_string()
}
```

- [ ] **Step 3: Fix unknown tool — return Err instead of Ok**

```rust
// crates/oni-agent/src/tools/mod.rs — in ToolRegistry::execute():
// Find where it returns Ok("Unknown tool: ...") and change to:
Err(oni_core::error::err!("Unknown tool: {}", name))
```

- [ ] **Step 4: Fix callbacks — use actual randomness**

```rust
// crates/oni-agent/src/callbacks.rs — replace the deterministic hash check with:
// Add `rand` to oni-agent/Cargo.toml dependencies, or use a simpler approach:
use std::time::{SystemTime, UNIX_EPOCH};
let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .subsec_nanos();
if nanos % 5 != 0 {
    return None; // ~80% of the time, skip
}
// This uses nanosecond jitter as a lightweight PRNG without adding a dependency
```

- [ ] **Step 5: Build and test**

Run: `$HOME/.cargo/bin/cargo build && $HOME/.cargo/bin/cargo test`

- [ ] **Step 6: Commit**

```bash
git add crates/oni-agent/src/agent.rs crates/oni-agent/src/tools/mod.rs crates/oni-agent/src/callbacks.rs
git commit -m "fix: operator precedence in orchestrator heuristic, strip_thinking offset, unknown tool error

- should_orchestrate: || vs && precedence now correct with parens
- strip_thinking: searches for </think> from <think> position, not string start
- Unknown tool returns Err not Ok (was logging false success telemetry)
- Callbacks use nanosecond jitter instead of deterministic hash"
```

---

### Task 5: Fix orchestrator replan and deduplicate parsing code

**Findings:** #7 (replan from step 0), #17 (duplicated strip_thinking/extract_text_tool_call)

**Files:**
- Create: `crates/oni-agent/src/parsing.rs`
- Modify: `crates/oni-agent/src/lib.rs`
- Modify: `crates/oni-agent/src/agent.rs`
- Modify: `crates/oni-agent/src/orchestrator.rs`

- [ ] **Step 1: Create parsing.rs with shared functions**

Move `strip_thinking`, `TextToolCall`, and `extract_text_tool_call` from `agent.rs` into a new `crates/oni-agent/src/parsing.rs`. Make them `pub(crate)`.

- [ ] **Step 2: Update lib.rs**

Add `pub(crate) mod parsing;`

- [ ] **Step 3: Update agent.rs and orchestrator.rs**

Replace the duplicated private functions with `use crate::parsing::{strip_thinking, extract_text_tool_call, TextToolCall};`

- [ ] **Step 4: Fix orchestrator replan — resume from failed step**

```rust
// In orchestrator.rs run_task(), the 'replan loop:
// After `continue 'replan;`, the loop restarts at step 0.
// Add a `start_from` variable:

let mut start_from = 0;

'replan: loop {
    let total = steps.len();

    for (idx, step) in steps.iter().enumerate().skip(start_from) {
        // ... existing step execution ...

        // On rejection + replan:
        if !recovered {
            // ... existing replan logic ...
            start_from = idx; // Resume from the failed step
            continue 'replan;
        }
    }
    break;
}
```

- [ ] **Step 5: Build and test**

- [ ] **Step 6: Commit**

```bash
git add crates/oni-agent/src/parsing.rs crates/oni-agent/src/lib.rs \
  crates/oni-agent/src/agent.rs crates/oni-agent/src/orchestrator.rs
git commit -m "fix: deduplicate parsing code, fix orchestrator replan to resume from failed step

- Shared strip_thinking/extract_text_tool_call in parsing.rs
- Replan no longer re-executes completed steps"
```

---

### Task 6: Fix conversation and callbacks correctness

**Findings:** #42 (token undercount), #43 (compaction mid-conversation system message)

**Files:**
- Modify: `crates/oni-agent/src/conversation.rs:54-62`

- [ ] **Step 1: Fix token estimation to include tool_calls**

```rust
// conversation.rs estimated_tokens():
pub fn estimated_tokens(&self) -> u64 {
    let sys_tokens = self.system_prompt.len() as u64 / 4;
    let msg_tokens: u64 = self
        .messages
        .iter()
        .map(|m| {
            let content_tokens = m.content.len() as u64 / 4;
            let tool_tokens = m.tool_calls.as_ref()
                .map(|tc| serde_json::to_string(tc).unwrap_or_default().len() as u64 / 4)
                .unwrap_or(0);
            content_tokens + tool_tokens
        })
        .sum();
    sys_tokens + msg_tokens
}
```

- [ ] **Step 2: Fix compaction — use user role for summary, place at index 0**

```rust
// In conversation.rs compact():
// Instead of inserting a system message mid-conversation, insert at index 0:
pub fn compact(&mut self, summary: &str, retain_last: usize) {
    let retain = retain_last.min(self.messages.len());
    let keep = self.messages.split_off(self.messages.len() - retain);
    self.messages.clear();
    // Insert summary as a user message at the start (not system — LLMs reject mid-stream system)
    self.messages.push(ChatMessage::user(&format!("[Context summary: {}]", summary)));
    self.messages.extend(keep);
}
```

- [ ] **Step 3: Build and test**

- [ ] **Step 4: Commit**

```bash
git add crates/oni-agent/src/conversation.rs
git commit -m "fix: include tool_calls in token estimation, fix compaction message placement"
```

---

### Task 7: Fix config and type correctness

**Findings:** #10 (dead default_tier), #40 (config no merge), #41 (Heavy no tools)

**Files:**
- Modify: `crates/oni-core/src/types.rs:31-36`
- Modify: `crates/oni-core/src/config.rs`

- [ ] **Step 1: Add Heavy to supports_tools()**

```rust
// crates/oni-core/src/types.rs:31-36
pub fn supports_tools(&self) -> bool {
    matches!(
        self,
        ModelTier::Heavy | ModelTier::Medium | ModelTier::General | ModelTier::Fast
    )
}
```

- [ ] **Step 2: Delete dead AgentConfig::default_tier method**

Remove the `default_tier()` method from `AgentConfig` entirely (grep confirms no callers).

- [ ] **Step 3: Fix config loading — merge project over global**

```rust
// config.rs load_config() — when project config exists:
// Instead of full replacement, deserialize into global config using serde defaults:
// Use toml::Value merge approach:
if let Ok(project_str) = std::fs::read_to_string(&project_config_path) {
    // Parse project config as Value, merge into base config Value
    if let Ok(project_val) = project_str.parse::<toml::Value>() {
        let mut base_val = toml::Value::try_from(&config)
            .unwrap_or(toml::Value::Table(toml::map::Map::new()));
        merge_toml(&mut base_val, &project_val);
        if let Ok(merged) = base_val.try_into::<OniConfig>() {
            config = merged;
        }
    }
}

// Add merge helper:
fn merge_toml(base: &mut toml::Value, overlay: &toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_table), toml::Value::Table(overlay_table)) => {
            for (key, value) in overlay_table {
                if let Some(base_value) = base_table.get_mut(key) {
                    merge_toml(base_value, value);
                } else {
                    base_table.insert(key.clone(), value.clone());
                }
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}
```

- [ ] **Step 4: Build and test**

- [ ] **Step 5: Commit**

```bash
git add crates/oni-core/src/types.rs crates/oni-core/src/config.rs
git commit -m "fix: add Heavy tier to supports_tools, delete dead default_tier, merge project config"
```

---

### Task 8: Fix performance issues

**Findings:** #21/#22 (KG O(n²)), #23 (preferences connection per op), #24 (bash blocking Command), #11 (write_file fake diff)

**Files:**
- Modify: `crates/oni-agent/src/knowledge_graph.rs`
- Modify: `crates/oni-agent/src/preferences.rs`
- Modify: `crates/oni-agent/src/tools/bash.rs`

- [ ] **Step 1: Fix knowledge_graph — use HashSet for search and gc**

```rust
// knowledge_graph.rs search():
use std::collections::HashSet;

// Replace Vec<String> matching_ids with HashSet<String>
let matching_ids: HashSet<String> = self.nodes.iter()
    .filter(|(_, n)| n.content.to_lowercase().contains(&query_lower))
    .map(|(id, _)| id.clone())
    .collect();

// Then filter with .contains() which is O(1) on HashSet
```

Apply the same fix to `gc()` where `stale_ids` is a Vec.

- [ ] **Step 2: Fix preferences — cache connection**

```rust
// preferences.rs — change PreferenceEngine to hold a connection:
pub struct PreferenceEngine {
    conn: rusqlite::Connection,  // Was: db_path: PathBuf
}

impl PreferenceEngine {
    pub fn new(db_path: PathBuf) -> Self {
        let conn = rusqlite::Connection::open(&db_path)
            .unwrap_or_else(|_| rusqlite::Connection::open_in_memory().unwrap());
        Self { conn }
    }
    // Replace all open_conn() calls with &self.conn
}
```

- [ ] **Step 3: Fix bash.rs truncation — use char boundary**

```rust
// bash.rs:106-108 — replace result.truncate(50_000) with safe truncation:
if result.len() > 50_000 {
    let mut end = 50_000;
    while end > 0 && !result.is_char_boundary(end) {
        end -= 1;
    }
    result.truncate(end);
    result.push_str("\n...[truncated]");
}
```

- [ ] **Step 4: Build and test**

- [ ] **Step 5: Commit**

```bash
git add crates/oni-agent/src/knowledge_graph.rs crates/oni-agent/src/preferences.rs \
  crates/oni-agent/src/tools/bash.rs
git commit -m "perf: HashSet for KG search/gc, cache SQLite connection in preferences, safe truncation"
```

---

### Task 9: Fix remaining correctness issues

**Findings:** #44 (linter workspace-wide), #45 (preferences SQL REPLACE), #46 (plan_store global path), #47 (agent_defs YAML parsing), #49 (health.rs prefix matching)

**Files:**
- Modify: `crates/oni-agent/src/linter.rs`
- Modify: `crates/oni-agent/src/preferences.rs`
- Modify: `crates/oni-agent/src/plan_store.rs`
- Modify: `crates/oni-agent/src/agent_defs.rs`
- Modify: `crates/oni-ollama/src/health.rs` (if it exists, otherwise `client.rs`)

- [ ] **Step 1: Fix linter — find nearest Cargo.toml**

```rust
// linter.rs — for Rust files, traverse up to find Cargo.toml:
fn find_manifest(file_path: &str) -> Option<String> {
    let mut dir = std::path::Path::new(file_path).parent()?;
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.exists() {
            return Some(manifest.to_string_lossy().to_string());
        }
        dir = dir.parent()?;
    }
}

// Then pass --manifest-path to clippy:
if let Some(manifest) = find_manifest(path) {
    cmd.arg("--manifest-path").arg(manifest);
}
```

- [ ] **Step 2: Fix plan_store — include project dir hash in filename**

```rust
// plan_store.rs — PersistedPlan path:
fn plan_path(project_dir: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    project_dir.hash(&mut hasher);
    let hash = hasher.finish();
    let data = oni_core::config::data_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
    data.join(format!("active-plan-{:x}.json", hash))
}
```

- [ ] **Step 3: Fix agent_defs YAML delimiter**

```rust
// agent_defs.rs — replace after_first.find("---") with:
after_first.find("\n---").map(|pos| pos + 1) // +1 to skip the \n
// Or if the delimiter can be at the very start of a line:
```

- [ ] **Step 4: Fix health.rs model name matching**

```rust
// Replace prefix matching with exact or tag-aware matching:
names.iter().any(|m| m == name || m == &format!("{}:latest", name))
```

- [ ] **Step 5: Build and test**

- [ ] **Step 6: Commit**

```bash
git add crates/oni-agent/src/linter.rs crates/oni-agent/src/plan_store.rs \
  crates/oni-agent/src/agent_defs.rs crates/oni-ollama/
git commit -m "fix: linter targets correct crate, plan_store per-project, YAML delimiter, model matching"
```

---

### Task 10: Clean up dead code and warnings

**Findings:** #15/#16 (telemetry unused params), #17/#18 (orchestrator dead fields), #19 (NORN), #20/#50 (eval runner stub — defer to separate task), #5 (undo silent failures), #34 (reflection silent write_soul)

**Files:**
- Modify: `crates/oni-agent/src/telemetry.rs`
- Modify: `crates/oni-agent/src/orchestrator.rs`
- Modify: `crates/oni-agent/src/prompts.rs`
- Modify: `crates/oni-agent/src/reflection.rs`
- Modify: `crates/oni-agent/src/tools/undo.rs`
- Modify: `crates/oni-core/src/personality.rs` (unused imports/vars)

- [ ] **Step 1: Fix telemetry unused params — actually record them**

```rust
// telemetry.rs orchestrator_plan():
pub fn orchestrator_plan(&self, steps: usize) {
    let mut inner = self.inner.lock().unwrap();
    inner.orchestrator_plans += 1;
    inner.features_used.entry("orchestrator".to_string()).and_modify(|c| *c += 1).or_insert(1);
    if inner.enabled {
        let ts = inner.start.elapsed().as_millis() as u64;
        inner.events.push(TelemetryEvent {
            timestamp_ms: ts,
            layer: TelemetryLayer::Orchestrator,
            event: "plan".into(),
            data: [("steps".to_string(), serde_json::json!(steps))].into(),
        });
    }
}

// telemetry.rs compaction_triggered():
pub fn compaction_triggered(&self, tokens_before: u64, tokens_after: u64) {
    let mut inner = self.inner.lock().unwrap();
    inner.compaction_triggers += 1;
    inner.features_used.entry("compaction".to_string()).and_modify(|c| *c += 1).or_insert(1);
    if inner.enabled {
        let ts = inner.start.elapsed().as_millis() as u64;
        inner.events.push(TelemetryEvent {
            timestamp_ms: ts,
            layer: TelemetryLayer::Compaction,
            event: "compaction".into(),
            data: [
                ("tokens_before".to_string(), serde_json::json!(tokens_before)),
                ("tokens_after".to_string(), serde_json::json!(tokens_after)),
            ].into(),
        });
    }
}
```

- [ ] **Step 2: Remove dead orchestrator fields**

Remove `max_spawn_depth` and `spawn_depth` from the `Orchestrator` struct and constructor.

- [ ] **Step 3: Remove dead NORN constant from prompts.rs**

Delete the `NORN` constant. If a NORN agent is needed later, it can be added back.

- [ ] **Step 4: Fix reflection — propagate write_soul error and remove unused variable**

```rust
// reflection.rs:91 — remove: let yesterday = personality::read_yesterday_journal();
// reflection.rs:154 — change: let _ = personality::write_soul(&soul);
// To:
if let Err(e) = personality::write_soul(&soul) {
    tracing::warn!("Failed to apply personality mutation: {}", e);
}
```

- [ ] **Step 5: Fix undo.rs — propagate errors**

```rust
// undo.rs — replace let _ = std::fs::write/remove_file with:
std::fs::write(&path, &content).map_err(|e| oni_core::error::err!("Failed to undo write to {}: {}", path, e))?;
// Or at minimum return the error string:
if let Err(e) = std::fs::write(&path, &content) {
    return Ok(format!("Error reverting {}: {}", path, e));
}
```

- [ ] **Step 6: Fix compiler warnings in personality.rs**

```rust
// Remove unused import: Path (line 11)
// Prefix unused param: _session_id (line 431)
```

- [ ] **Step 7: Build and test — should have 0 warnings in oni-agent and oni-core**

Run: `$HOME/.cargo/bin/cargo build 2>&1 | grep warning`
Expected: Only warnings from external crates, none from oni-* crates

- [ ] **Step 8: Commit**

```bash
git add crates/oni-agent/ crates/oni-core/
git commit -m "chore: clean up dead code, fix compiler warnings, propagate errors

- Telemetry now records orchestrator step count and compaction token delta
- Removed dead max_spawn_depth, spawn_depth, NORN constant
- Undo/reflection errors no longer silently discarded
- Fixed all compiler warnings in oni-agent and oni-core"
```

---

## Post-Implementation Notes

**Not covered in this plan (separate work):**
- **#20/#50 — Eval runner implementation**: The eval framework needs real LLM calls + assertion checking. This is a feature, not a bugfix — should be its own plan.
- **#18 — palette.rs in oni-core**: Moving it to oni-tui is an architectural refactor that touches imports across all crates. Worth doing but not urgent.
- **#33 — SQL format! in oni-db**: The value is a `u32` so there's no injection risk, but the pattern should be cleaned up with parameterised queries.

**After all 10 tasks, run the full ablation benchmark again:**
```bash
$HOME/.cargo/bin/cargo install --path . --force
bash bench/stress_test.sh --mode full
```

Compare results against the pre-fix baseline (14/27 = 51% in full mode). The orchestrator heuristic fix (Task 4 Step 1 + existing `should_orchestrate` rewrite) and Heavy tier tools fix (Task 7 Step 1) should directly improve benchmark scores.
