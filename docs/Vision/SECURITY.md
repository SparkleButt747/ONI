# ONI — Security Model

ONI is a local-first tool. No cloud APIs, no OAuth tokens, no keychain dependencies.
All security controls are implemented in Rust and enforced at the tool layer.

---

## Threat Model

ONI runs on a developer's local machine with access to:
- Their file system (reads always; writes only with `--write`)
- Their shell (only with `--exec`)
- External HTTP (get_url tool; filtered to public addresses only)

Primary threats:
1. **Unintended file modification** — agent writes files without authorisation
2. **Unintended code execution** — agent runs dangerous commands
3. **Path traversal** — agent escapes the project directory
4. **Grep injection** — pattern argument treated as a flag by grep
5. **Prompt injection via codebase** — malicious content in indexed files manipulates agent behaviour
6. **SSRF via get_url** — agent fetches internal/private network addresses

---

## Permission Model

`ToolRegistry::new(allow_write, allow_exec)` gates tool registration at startup.
No permission = no tool object in the registry = no possible call path.

```
oni chat                      # read_file, list_dir, search_files, get_url, undo
oni chat --write              # + write_file, edit_file
oni chat --exec               # + bash, forge_tool
oni chat --write --exec       # full access
```

Tools always available (no flag required):
- `read_file`, `list_dir`, `search_files` — read-only, no side effects
- `get_url` — outbound HTTP to public addresses only (see URL Validation below)
- `undo` — reverts the last write_file or edit_file from the undo history
- `ask_user` — pauses the agent to ask the user a direct question

Tools gated on `--write`:
- `write_file`, `edit_file`

Tools gated on `--exec`:
- `bash`, `forge_tool`

Flags are session-scoped (CLI args) or set in `oni.toml` via `[agent] allow_write` / `allow_exec`.

---

## Bash Blocklist

`BLOCKED_PATTERNS` in `crates/oni-agent/src/tools/bash.rs`. Before any execution, the command
string is normalised (lowercased, whitespace collapsed) and checked against:

```
rm -rf /
rm -rf /*
rm -rf ~
mkfs
dd if=
:(){ :|:& };:    (fork bomb)
chmod -r 777 /
sudo rm
sudo dd
sudo mkfs
> /dev/sda
curl | sh
curl | bash
wget | sh
wget | bash
```

If any pattern matches, the tool returns `"BLOCKED: Command matches security blocklist pattern."`
without spawning a process.

---

## Path Traversal Prevention

Both `write_file` and `edit_file` call `is_safe_path()` before touching disk.

`write_file` (`crates/oni-agent/src/tools/write_file.rs`):
- Iterates `Path::components()` — rejects any `Component::ParentDir` (`..`)
- For absolute paths: checks `path.starts_with(cwd)` — rejects anything outside cwd

`edit_file` (`crates/oni-agent/src/tools/edit_file.rs`):
- Same component-based `..` detection
- Does not need the absolute-path check because edit_file operates on existing files

Result: `../../.zshrc`, `/etc/passwd`, and similar attempts return an error string
without any disk access.

---

## Grep Injection Prevention

`search_files` passes the user pattern to `grep` with `--` before the pattern argument:

```rust
cmd.arg("--").arg(pattern).arg(search_path);
```

This prevents a pattern like `--include=*.rs -e INJECTED` from being interpreted
as grep flags.

---

## Forge Tool Safety

`forge_tool` (`crates/oni-agent/src/tools/forge_tool.rs`) generates and runs one-off bash
scripts. It applies two layers of safety before execution:

1. **Same blocklist as bash** — lowercased script body checked against `BLOCKED_PATTERNS`
2. **Syntax check** — `bash -n -c <script>` runs before execution; rejects on syntax error
3. **30-second timeout** — `child.try_wait()` loop kills the process after 30 seconds

---

## TUI Inline Shell

`:cmd` prefix in the TUI (`crates/oni-tui/src/app.rs` → `handle_slash_command`) executes
shell commands directly, bypassing the agent. It applies its own blocklist — identical
patterns to the bash tool — with the same normalisation (lowercase + whitespace collapse)
before matching.

---

## URL Validation

`get_url` (`crates/oni-agent/src/tools/get_url.rs`) enforces two checks before any
network call:

**Scheme allowlist** — rejects anything that is not `http://` or `https://`:
```
Error: only http:// and https:// URLs are allowed
```

**Private address blocklist** — rejects:
- `localhost`
- `127.0.0.1`
- `0.0.0.0`
- `169.254.*` (link-local / APIPA)
- `10.*` (RFC 1918 private)
- `192.168.*` (RFC 1918 private)

This prevents SSRF to local services (Ollama itself, dev servers, cloud metadata endpoints).

---

## No Auth, No Credentials

ONI talks to Ollama, which is local. There are no API keys, OAuth tokens, or credentials
of any kind stored anywhere by ONI:
- No keychain access
- No `.env` files
- No token fields in the SQLite database
- No credentials in the TOML config

The only network connection from ONI core is to `http://localhost:11434` (Ollama) and
outbound `get_url` fetches to public URLs.

---

## Context Isolation

The system prompt is assembled in `build_system_prompt()` (and its variants) in
`crates/oni-agent/src/system_prompt.rs`. Retrieved codebase chunks are appended
after the operational instructions under a `## CONTEXT` section header.

The model receives:
```
[system instructions] ... ## CONTEXT [retrieved file chunks] ...
```

User messages are in the `user` role, separate from the system turn. The model
sees instructions and data as distinct conversation roles, not interleaved text.

There is no additional sandboxing wrapper around retrieved chunks; the separation
is structural (system role content vs. user/tool role content).

---

## Autonomy Level and Confirmations

`AutonomyLevel` (Low / Medium / High) controls when the agent stops to ask the
user before executing a tool. Confirmations are separate from the blocklist —
the blocklist blocks outright, confirmations pause for user approval.

| Level | Confirmation required |
|-------|----------------------|
| Low | Everything except `read_file`, `list_dir`, `search_files` |
| Medium | `bash` always; `write_file` and `edit_file` always |
| High | Nothing (blocklist still enforced) |

In headless mode (no TUI proposal channel), confirmations are auto-approved.

---

## Undo

`write_file` and `edit_file` snapshot the file's current content into `UndoHistory`
before writing. The `undo` tool reverts the last snapshot. History depth is 50 entries.
This is a defence-in-depth measure, not a security control — it lets users recover from
unintended writes.

---

## Audit Checklist

- `grep -r "sudo" crates/` — should only appear inside `BLOCKED_PATTERNS` strings
- `grep -r "keychain\|keytar\|oauth\|api_key" crates/` — should return nothing
- `grep -r "rejectUnauthorized" crates/` — should return nothing (reqwest defaults)
- `cargo test` — includes `t_tool_14_bash_blocks_rm_rf_root` and related blocklist tests
- Path traversal: `cargo test t_tool` — covers `../` rejection in write_file and edit_file
- `write_file` blocked without `--write`: confirmed by `ToolRegistry::new(false, _)` test
- `bash` blocked without `--exec`: confirmed by `ToolRegistry::new(_, false)` test
