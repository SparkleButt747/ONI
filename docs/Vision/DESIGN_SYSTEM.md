# ONI — Design System

**Codename: Graphic Realism**

Inspired by Marathon (Bungie 2025). Art Director Joseph Cross defined the style as "Graphic Realism" — stylised representation of reality, not photorealism. Wipeout meets Ghost in the Shell. Y2K cyberpunk crossed with acid graphic design posters.

ONI's visual language maps this directly to a terminal UI: machine consciousness, utility as aesthetics, saturated clarity on near-black surfaces.

---

## Design Philosophy

### 1. Graphic Realism
Objects are real but rendered with graphic clarity. No photorealism, no pure abstraction. Every surface communicates its function through visual language — labels, stripes, colour-coding. Reality, amplified.

### 2. Utility as aesthetics
Industrial signage, hazard markings, stencil type, warning colours — these are not decorations, they ARE the design. ONI's UI borrows this: every chrome element is functional, every stripe earns its place.

### 3. Saturated clarity
High-chroma accent colours on near-black backgrounds. No gradients diluting the signal. No blending. Flat planes of colour separated by hard edges.

### 4. Machine consciousness
ONI is an onboard intelligence. The UI should feel like looking at a system's self-representation — diagnostic, precise, slightly alien. Ghost in the Shell HUDs, not Apple glassmorphism.

### 5. Retrofuturism
Monospace terminals, CRT glitch artefacts, scan-line textures — 90s sci-fi futures as imagined from now. Technology that has been used, worn, maintained.

### 6. No comfort phrasing (visual equivalent)
No rounded-corner softness. No pastel reassurance. Sharp cuts, hard borders, direct type. The design communicates the same way ONI speaks.

---

## Colour System

All colours are defined in `crates/oni-core/src/palette.rs` as `ratatui::style::Color::Rgb` constants. The `oni-tui` crate imports them via `use oni_core::palette`.

### Base palette (near-black, not true black)

| Constant | RGB | Hex | Usage |
|---|---|---|---|
| `BG` | `(10, 10, 9)` | `#0a0a09` | Primary background — all views |
| `PANEL` | `(26, 26, 24)` | `#1a1a18` | Surfaces, sidebar, cards |
| `BORDER` | `(42, 42, 39)` | `#2a2a27` | All dividers and borders |
| `DIM` | `(58, 58, 55)` | `#3a3a37` | Inactive borders, tiled background textures |
| `MUTED` | `(107, 104, 96)` | `#6b6860` | Secondary text, timestamps, meta labels |
| `TEXT` | `(200, 197, 187)` | `#c8c5bb` | Primary body text, user messages |
| `WHITE` | `(255, 255, 255)` | `#ffffff` | Headings, emphasis |

### Accent palette (semantic)

| Constant | RGB | Hex | Semantic role |
|---|---|---|---|
| `AMBER` | `(245, 166, 35)` | `#f5a623` | ONI primary, active tasks, cursor, stat values |
| `CYAN` | `(0, 212, 200)` | `#00d4c8` | Tool calls, system identity, Executor `[Ψ]` |
| `CORAL` | `(255, 77, 46)` | `#ff4d2e` | Error, danger, Critic `[⊘]`, blocked, glitch pulse |
| `LIME` | `(180, 224, 51)` | `#b4e033` | Success, done, accepted, recovery hints |
| `VIOLET` | `(123, 94, 167)` | `#7b5ea7` | Planner `[Σ]`, planning state |
| `WARNING` | `(232, 197, 71)` | `#e8c547` | Caution, burn rate alert, hazard stripe gap |

### Legacy aliases (backward compat)

| Alias | Maps to |
|---|---|
| `DATA` | `AMBER` |
| `SYSTEM` | `CYAN` |
| `ALERT` | `CORAL` |
| `STATE` | `LIME` |
| `GHOST` | `BORDER` |

### Semantic style functions

`palette.rs` exposes style constructors used across the TUI:

```
data_style()    → AMBER fg, BG bg
system_style()  → CYAN fg, BG bg
alert_style()   → CORAL fg, BG bg, BOLD
state_style()   → LIME fg, BG bg, BOLD
dim_style()     → DIM fg, BG bg, DIM modifier
input_style()   → AMBER fg, BG bg
label_style()   → AMBER fg, BG bg, BOLD
text_style()    → TEXT fg, BG bg
muted_style()   → MUTED fg, BG bg
```

`theme.rs` in `oni-tui` re-exports these for convenience: `theme::data()`, `theme::alert()`, etc.

### Colour rules
1. **Accents on void/panel only.** Never accent on accent.
2. **No gradients between accent colours.**
3. **No opacity blends.** Flat fills or nothing.
4. **Colour encodes meaning, not sequence.** Don't rainbow-cycle for decoration.
5. **Near-black, not true black.** `BG = (10, 10, 9)` — true black is harsh on displays.

---

## Typography

ONI is a terminal UI rendered with Ratatui. All text is monospace — the terminal's default font. There is no font selection at the terminal level.

### Conventions

| Element | Style |
|---|---|
| Structural labels (status tags, section titles) | Uppercase (`theme::label(text)` helper calls `.to_uppercase()`) |
| User messages | `TEXT` colour, normal weight |
| ONI responses | `TEXT` colour, normal weight |
| Tool call log | `CYAN` for tool name, `MUTED` for args and timestamps |
| Error headlines | `CORAL`, BOLD, uppercase |
| Recovery hints | `LIME`, BOLD |
| Metadata, footnotes | `MUTED` or `DIM` |

### BigText widget

`BigText` (in `crates/oni-tui/src/widgets/big_text.rs`) renders digits and `.` as 3×5 block-character bitmaps using `█` (U+2588). Used for stat card values in MissionControl.

- Character width: 3 columns
- Character height: 5 rows
- Character gap: 1 column
- Supports: `0–9`, `.` (index 10 — single dot at baseline)
- Unknown characters render as blank space (3×5 space)

---

## Widgets

All widgets are in `crates/oni-tui/src/widgets/`. They implement the Ratatui `Widget` trait and are state-free (all state is passed in via fields).

### `Spectrum`

Dense bar chart for token rates or timing data. Bars rendered bottom-up using Unicode vertical block elements (`▁▂▃▄▅▆▇█`). Each column maps one value to a height within the given area. No labels — raw visual signal.

- Default colour: `DATA` (amber)
- `max_height`: normalisation ceiling (default 100)
- Values: `Vec<u16>` — one per column

### `BigText`

See Typography section above. Used in MissionControl stat cards for TURNS, TOKENS, TOK/S, TOOLS.

### `GlitchBlocks`

Decorative noise overlay. Fills an area with randomly placed block characters (`▓▒░▄▀▌▐█`). Reproducible via `seed: u64` using a fast LCG — the same seed always produces the same pattern. `density: f32` (0.0–1.0) controls fill ratio.

Used for Splash/Boot background texture.

### `GlitchPulse`

Error-state overlay effect. Applied as 3 frames (`frame: 0|1|2`) on error transitions. Each frame shifts alternate rows by ±1–3 columns and fills shifted cells with `CORAL █`. Frame 0: +2, frame 1: −3, frame 2: +1. Stops rendering at `frame > 2`.

### `HazardDivider`

Amber/dark repeating stripe pattern. Direct reference to Marathon's industrial safety signage. Pattern: 4 amber `█` then 1 dim `░`, repeating. Renders as a single-row stripe at full width.

Used for major section breaks, danger warnings.

### `ScanReveal`

Scan-entry animation overlay. Given `revealed_cols: u16`, masks all columns beyond that point with DIM-coloured spaces. The caller increments `revealed_cols` each frame to produce a left-to-right reveal. Fully revealed (`revealed_cols >= area.width`) is a no-op.

### `active_border_color` (function, not a widget)

In `widgets/border_pulse.rs`. Returns the border colour for a running task, alternating between `AMBER` and `BORDER` on a 2-second cycle based on `tick` (frame counter) and `fps`. Used in MissionControl sub-agent status panel.

```rust
pub fn active_border_color(tick: u64, fps: u32) -> Color
```

### `ChromaStripe` (ui module)

Not in `widgets/` — defined in `ui/chroma.rs`. A single-row accent stripe rendered at the very top of every frame. Maps all 6 accent colours across the terminal width in equal segments using `▀` (U+2580):

`CORAL → AMBER → WARNING → LIME → CYAN → VIOLET → CORAL`

---

## Views

All views are in `crates/oni-tui/src/ui/`. The main `draw()` dispatcher is in `ui/mod.rs`.

### Frame layout

Every frame (except full-screen error) follows this vertical structure:

```
┌─────────────────────────────────┐
│ ChromaStripe         (1 row)    │
│ Status bar           (1 row)    │
│ Main content area    (fill)     │
│ Input area           (3 rows)   │
│ Footer stats         (1 row)    │
└─────────────────────────────────┘
```

When sidebar is active, main content splits horizontally: `fill | 24 cols`.

### Chat view (`ui/chat.rs`)

Default view when messages exist and agent is not thinking. Messages render as a scrollable list:
- User messages: amber `you` badge + TEXT body
- ONI messages: plain TEXT, with diff blocks inline when applicable

Empty background fills with a dim tiled texture (repeating pattern text at `DIM` colour with `DIM` modifier) to fill unused space.

When width > 4, a 2-column vertical `RESPONSE` label renders on the right edge.

### MissionControl view (`ui/mission_control.rs`)

Activated by `ViewMode::MissionControl`. Vertical stack:

```
┌─────────────────────────────────┐
│ Stat cards (7 rows)             │  TURNS / TOKENS / TOK/S / TOOLS
│   BigText numbers + labels      │  BigText (3×5) in AMBER/CYAN
├─────────────────────────────────┤
│ Tool call log (fill)            │  timestamp | tool | args | latency
├─────────────────────────────────┤
│ Sub-agent status (5 rows)       │  [Σ] [⚡] [⊘] + border pulse
├─────────────────────────────────┤
│ Session info (3 rows)           │  model, cwd, session ID
└─────────────────────────────────┘
```

### Preferences view (`ui/preferences.rs`)

Activated by `ViewMode::Preferences`. Shows learned rules with confidence-based colour coding:
- `confidence >= 0.80` → `ACTIVE` tag in LIME
- `0.50 ≤ confidence < 0.80` → `LEARNING` tag in AMBER
- `< 0.50` → `WEAK` tag in DIM

### Splash / Boot view (`ui/splash.rs`)

Shown on first launch before any messages. Frame-driven boot sequence with stage thresholds:

| Frame | Stage |
|---|---|
| 0 | Logo appears (ASCII block ONI logo) |
| 6 | Tagline: `ONBOARD_NATIVE_INTELLIGENCE` |
| 9 | First HazardDivider |
| 11–14 | Init log lines appear one per frame |
| 16 | Second HazardDivider |
| 18 | Keybind reference |
| 21 | Input cursor |

Background: tiled texture at DIM. Spectrum widget displays dummy boot signal data.

### Thinking view (`ui/thinking.rs`)

Rendered when `app.is_thinking = true`. Shows existing chat messages in the upper area, then a 3-row thinking zone at the bottom:
- Row 1: `PROCESSING` label centred, AMBER BOLD
- Row 2–3: `throbber-widgets-tui` spinner in AMBER; background tiled `PROCESSING_` text at DIM
- Full area background: tiled `PROCESSING_` pattern

### Error view (`ui/error_state.rs`)

Full-screen takeover when `app.critical_error` is `Some`. Tiled background: `EXECUTION_FAILED_` pattern in dim CORAL. Centred overlay:
- `[ CRITICAL_FAILURE ]` banner — CORAL BOLD
- Error headline — uppercase, CORAL BOLD
- Detail lines — DIM
- Separator rule
- Recovery hint — LIME BOLD (pattern-matched from error text)
- `CTRL+C to exit` — DIM

---

## Motion

### Principles
- **No easing.** State transitions are instant. Motion communicates state, not brand.
- **Step functions.** CRT aesthetic — digital, not fluid.
- **No decorative animation.** Everything moves for a reason.

### Patterns

**Scan entry (`ScanReveal`):**
Left-to-right reveal via `revealed_cols` incremented per frame. Feels like data loading. Used in Splash boot sequence.

**Glitch pulse (`GlitchPulse`):**
On error: cells shift horizontally ±1–3 cols for 3 frames. CORAL fill. No spring, no bounce. Hard and digital. Applied as an overlay.

**Border pulse (`active_border_color`):**
Running tasks pulse border between `BORDER` and `AMBER` on a 2-second cycle (`tick % cycle_frames`). Colour only — no geometry changes.

**Throbber spinner:**
Used in Thinking view via `throbber-widgets-tui`. AMBER colour. Cycles through spinner frames driven by `app.throbber_state`.

**Tiled texture backgrounds:**
Used in Chat, Thinking, Splash, and Error views. Dim repeating text patterns (`PROCESSING_`, `EXECUTION_FAILED_`, generic pattern) fill empty space. Static — no animation. Give depth without distraction.

---

## Sub-agent Prefixes

| Agent | Prefix | Colour |
|---|---|---|
| Planner | `[Σ]` | `VIOLET` `#7b5ea7` |
| Executor | `[Ψ]` | `CYAN` `#00d4c8` |
| Critic | `[⊘]` | `CORAL` `#ff4d2e` |

---

## Do / Don't

### Do
- `BG = (10, 10, 9)` — near-black, not true black
- Hard borders, flat fills
- Uppercase labels everywhere structural
- AMBER for active / primary, CORAL for error, LIME for success, CYAN for tools
- Hazard stripes for danger / limit / warning sections
- `BigText` for key numeric stats
- Left-aligned everything
- Status always visible (token count, turn count, model)
- `ScanReveal` for content entry, `GlitchPulse` for error frames, border pulse for running tasks
- `tracing` for diagnostic output — not `println!`

### Don't
- Rounded corners on any structural element
- Glassmorphism, frosted panels, opacity blends
- Drop shadows for decoration
- Gradients between accent colours
- Emoji in UI output
- Animated spinners outside the Thinking view
- Easing curves or smooth transitions
- Tooltips — information lives inline
- Centred headers
- "Loading..." text — use `ScanReveal` or border pulse
- Raw `&s[..n]` string slicing — use char-boundary-safe helpers

---

## Voice / Tone

Design and copy must match. ONI looks direct — it sounds direct.

| Context | ONI says | ONI does not say |
|---|---|---|
| Task complete | `Done. 3 files written, tests pass.` | `Great news! I've successfully completed your task!` |
| Error found | `Race condition. Line 47. Fix incoming.` | `I noticed there might be a potential issue with...` |
| Critic reject | `Rejected. JWT written to plaintext. Replanning.` | `Hmm, I'm not entirely sure this is the best approach.` |
| Blocked | `Blocked: CI requires approval. Your call.` | `I seem to be unable to proceed at this time.` |
| Uncertainty | `Not sure. Two options. Which?` | `I apologise for the confusion. Let me try to help...` |
