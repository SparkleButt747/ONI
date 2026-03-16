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

### Base palette (near-black, not true black)

| Token | Hex | Usage |
|---|---|---|
| `--oni-black` | `#0a0a09` | Primary background |
| `--oni-panel` | `#1a1a18` | Surfaces, panels, cards |
| `--oni-border` | `#2a2a27` | All dividers and borders |
| `--oni-dim` | `#3a3a37` | Inactive borders, subdued structure |
| `--oni-muted` | `#6b6860` | Secondary text, labels |
| `--oni-text` | `#c8c5bb` | Primary body text |
| `--oni-white` | `#ffffff` | Headings, emphasis |

### Accent palette (semantic)

| Token | Hex | Semantic role |
|---|---|---|
| `--acc-amber` | `#f5a623` | ONI primary, active tasks, cursor |
| `--acc-cyan` | `#00d4c8` | Tool calls, info, Executor [⚡] |
| `--acc-coral` | `#ff4d2e` | Error, danger, Critic [⊘], blocked |
| `--acc-lime` | `#b4e033` | Success, done, accepted |
| `--acc-violet` | `#7b5ea7` | Planner [Σ], planning state |
| `--acc-warning` | `#e8c547` | Caution, burn rate alert |

### Rules
1. **Accents on void/panel only.** Never accent on accent.
2. **No gradients between accent colours.**
3. **No opacity blends.** Flat fills or nothing.
4. **Colour encodes meaning, not sequence.** Don't rainbow-cycle through accents for decoration.
5. **Near-black, not true black.** `#0a0a09` — true black is harsh and unnatural on displays.

---

## Typography

### Font stack

| Role | Font | Weight | Notes |
|---|---|---|---|
| Display / headings | Barlow Condensed | 900 (Black) | Uppercase, tracked tight, all structural labels |
| Subheadings | Barlow Condensed | 600 (SemiBold) | Amber colour, section titles |
| Monospace (terminal, code, tool output) | Share Tech Mono | 400 | All REPL output, tool call log, status tags |
| Body copy | Barlow | 300 (Light) | ONI prose responses, explanations |

**Install:**
```bash
# Google Fonts
@import url('https://fonts.googleapis.com/css2?family=Share+Tech+Mono&family=Barlow+Condensed:wght@600;700;900&family=Barlow:wght@300;400&display=swap');
```

For terminal (ink components), fall back to terminal's monospace font for body text. Barlow requires a web renderer (future web UI, docs site).

### Scale

| Element | Size | Weight | Transform | Colour |
|---|---|---|---|---|
| H1 / mission title | 28–52px | 900 | UPPERCASE | white |
| H2 / section | 18–22px | 700 | UPPERCASE | white, amber |
| Label / tag | 9–11px | 600 | UPPERCASE | muted, accent |
| Mono body | 11–12px | 400 | as-is | cyan (tools), muted (meta) |
| Body prose | 13px | 300 | as-is | oni-text |

### Rules
1. **Zero other typefaces.** Barlow Condensed + Share Tech Mono + Barlow only.
2. **All structural labels uppercase.** Status tags, section titles, stat bar labels.
3. **Never centre text in UI.** Hard left alignment always. Exception: stat values in Mission Control stat bar.
4. **No mid-sentence bolding.** Code in monospace. No `**bold**` emphasis mid-prose.

---

## Spacing

| Token | Value | Usage |
|---|---|---|
| `space-1` | 4px | Tight internal padding |
| `space-2` | 8px | Gap between related elements |
| `space-3` | 12px | Panel internal padding |
| `space-4` | 16px | Section gap |
| `space-6` | 24px | Major section spacing |

---

## Components

### Status tags
```
RUNNING   BLOCKED   ERROR   DONE   IDLE
```
- Font: Share Tech Mono, 9px, uppercase
- Border: 1px solid, matching text colour
- Background: transparent (except `ERROR` which uses solid coral fill)
- No border-radius. Ever.
- Colours: `RUNNING` amber, `BLOCKED` coral, `ERROR` coral fill, `DONE` lime, `IDLE` muted

### Progress bars
- Height: 5px
- Border-radius: 0
- No animation
- Fill colour semantic: amber = active, lime = complete, coral = overbudget
- Track colour: `--oni-border`
- Label above bar in mono, not inside

### Tool call log line
```
14:22:04  bash         npx jest UserService --watch=false         1.2s
```
- Font: Share Tech Mono, 11px
- Columns: timestamp (muted), tool_name (cyan), arg (muted), latency (muted)
- Background: `--oni-panel`
- Padding: 5px 8px
- No border — background differentiates from surrounding content

### Hazard dividers
```
████░████░████░████░████░████░
```
- 6px height, full width
- Amber/black repeating stripe: `repeating-linear-gradient(90deg, #f5a623 0, #f5a623 10px, transparent 10px, transparent 20px)`
- Opacity: 0.7
- Used for: major section breaks, destructive action warnings, budget alerts
- Direct reference to Marathon's industrial safety signage

### Section headers
- Barlow Condensed 700, uppercase, white, 22px
- Left border accent: 2–3px solid in section semantic colour
- No background
- Hard left edge, never centred

### Sub-agent prefixes
| Agent | Prefix | Colour |
|---|---|---|
| Planner | `[Σ]` | `#7b5ea7` violet |
| Executor | `[⚡]` | `#00d4c8` cyan |
| Critic | `[⊘]` | `#ff4d2e` coral |

---

## Motion

### Principles
- **Near-instant transitions.** Machines don't ease in. 0ms for state changes, 150ms max for content assembly.
- **No decorative animation.** Motion communicates state, not brand.
- **Step functions over easing.** CRT aesthetic — digital, not fluid.

### Patterns

**Scan entry (element load):**
- Technique: left-to-right or top-to-bottom reveal using clip-path
- Duration: 180ms
- Easing: `steps(8)` — pixel-by-pixel, not smooth
- Feels like data being loaded, not a CSS transition

**Glitch pulse (error state):**
- On error: element X-shifts ±3px × 3 rapid iterations
- Border flashes coral
- Duration: 120ms total
- No spring, no bounce. Hard, digital, wrong.

**Border pulse (active task):**
- Running tasks pulse border between `--oni-border` and `--acc-amber`
- Loop: 2s, `border-color` only
- No opacity, no scale

**Block cursor:**
- 7×13px solid block (not underline, not beam)
- Amber fill (`--acc-amber`)
- Blink: `steps(1)` — binary on/off, no fade
- Period: 1s
- Identical in all input contexts

**Type stream:**
- ONI responses appear character-by-character
- Matches actual SSE streaming (not faked)
- Reinforces machine-printing aesthetic

---

## Terminal Colour Mapping

In the ink TUI and raw terminal output, map design tokens to ANSI colours:

| Token | ANSI |
|---|---|
| `--acc-amber` | `chalk.hex('#f5a623')` or bold yellow |
| `--acc-cyan` | `chalk.hex('#00d4c8')` or cyan |
| `--acc-coral` | `chalk.hex('#ff4d2e')` or red |
| `--acc-lime` | `chalk.hex('#b4e033')` or green |
| `--acc-violet` | `chalk.hex('#7b5ea7')` or magenta |
| `--oni-muted` | `chalk.gray` |
| `--oni-text` | default terminal colour |

**Degradation:** when terminal doesn't support 24-bit colour, fall back to ANSI 256 → ANSI 8. Never lose the semantic meaning (error = red, success = green, active = yellow).

---

## Do / Don't Reference

### Do
- Hard borders: 1px solid, no box-shadow
- Flat colour fills
- Block cursor, amber, binary blink
- Barlow Condensed for all labels, uppercase
- Share Tech Mono for all terminal/tool output
- Hazard stripes for danger/limit/warning sections
- Left-aligned everything
- Status always visible (token count, burn rate, task count)
- Colour encodes meaning
- Near-black background `#0a0a09`

### Don't
- `border-radius > 0` on any structural element
- Glassmorphism, frosted panels, `backdrop-filter`
- Drop shadows or `box-shadow` for decoration
- Gradients between accent colours
- Emoji in UI
- Inter, Roboto, system-ui fonts
- Animated spinners — use border pulse
- Easing curves >150ms
- Tooltips — information lives inline
- Centred headers or hero text
- "Loading..." text — show progress bars or border pulses
- Rounded status badges

---

## Voice / Tone (Companion to Visual)

Design and copy must match. ONI looks direct — it sounds direct.

| Context | ONI says | ONI does not say |
|---|---|---|
| Task complete | `Done. 3 files written, tests pass.` | `Great news! I've successfully completed your task!` |
| Error found | `Race condition. Line 47. Fix incoming.` | `I noticed there might be a potential issue with...` |
| Critic reject | `Rejected. JWT written to plaintext. Replanning.` | `Hmm, I'm not entirely sure this is the best approach.` |
| Blocked | `Blocked: CI requires approval. Your call.` | `I seem to be unable to proceed at this time.` |
| Uncertainty | `Not sure. Two options. Which?` | `I apologise for the confusion. Let me try to help...` |

The visual language and the voice reinforce each other. Terse copy on a direct, hard-edged interface. Verbose copy on a soft, rounded interface. ONI is neither soft nor verbose.
