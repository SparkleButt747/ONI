use ratatui::style::{Color, Modifier, Style};

// ── Base palette (DESIGN_SYSTEM.md — "Graphic Realism") ─────────────────────
// Near-black, not true black. Marathon / Ghost in the Shell / Wipeout aesthetic.
pub const BG: Color = Color::Rgb(10, 10, 9);            // --oni-black  #0a0a09
pub const PANEL: Color = Color::Rgb(26, 26, 24);        // --oni-panel  #1a1a18
pub const BORDER: Color = Color::Rgb(60, 60, 56);       // --oni-border #3c3c38  (was #2a2a27)
pub const DIM: Color = Color::Rgb(90, 90, 85);          // --oni-dim    #5a5a55  (was #3a3a37)
pub const MUTED: Color = Color::Rgb(130, 127, 118);     // --oni-muted  #827f76  (was #6b6860)
pub const TEXT: Color = Color::Rgb(200, 197, 187);       // --oni-text   #c8c5bb
pub const WHITE: Color = Color::Rgb(255, 255, 255);      // --oni-white  #ffffff

// ── Accent palette (semantic) ───────────────────────────────────────────────
pub const AMBER: Color = Color::Rgb(245, 166, 35);      // --acc-amber   #f5a623 — primary, active, cursor
pub const CYAN: Color = Color::Rgb(0, 212, 200);        // --acc-cyan    #00d4c8 — tool calls, Executor [⚡]
pub const CORAL: Color = Color::Rgb(255, 77, 46);       // --acc-coral   #ff4d2e — error, Critic [⊘]
pub const LIME: Color = Color::Rgb(180, 224, 51);       // --acc-lime    #b4e033 — success, accepted
pub const VIOLET: Color = Color::Rgb(123, 94, 167);     // --acc-violet  #7b5ea7 — Planner [Σ]
pub const WARNING: Color = Color::Rgb(232, 197, 71);    // --acc-warning #e8c547 — burn rate alert

// ── Legacy aliases (keep older references compiling) ────────────────────────
pub const DATA: Color = AMBER;                           // primary accent
pub const SYSTEM: Color = CYAN;                          // system identity
pub const ALERT: Color = CORAL;                          // errors/alerts
pub const STATE: Color = LIME;                           // state announcements
pub const GHOST: Color = DIM;                            // inactive/ghost elements

// ── Semantic styles ─────────────────────────────────────────────────────────
pub fn data_style() -> Style {
    Style::default().fg(AMBER).bg(BG)
}

pub fn system_style() -> Style {
    Style::default().fg(CYAN).bg(BG)
}

pub fn alert_style() -> Style {
    Style::default().fg(CORAL).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn state_style() -> Style {
    Style::default().fg(LIME).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn dim_style() -> Style {
    Style::default().fg(DIM).bg(BG)
}

pub fn input_style() -> Style {
    Style::default().fg(AMBER).bg(BG)
}

pub fn label_style() -> Style {
    Style::default().fg(AMBER).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn text_style() -> Style {
    Style::default().fg(TEXT).bg(BG)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED).bg(BG)
}
