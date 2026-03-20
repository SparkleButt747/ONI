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

// ── Accent palette (Marathon 2026 "Graphic Realism" neons) ──────────────────
pub const MAGENTA: Color = Color::Rgb(234, 2, 126);     // --acc-magenta      #ea027e — primary, active, cursor
pub const ELECTRIC_BLUE: Color = Color::Rgb(54, 1, 251); // --acc-electric-blue #3601fb — Planner [Σ]
pub const CYAN: Color = Color::Rgb(0, 212, 200);        // --acc-cyan         #00d4c8 — tool calls, Executor [Ψ]
pub const CORAL: Color = Color::Rgb(255, 77, 46);       // --acc-coral        #ff4d2e — error, Critic [⊘]
pub const LIME: Color = Color::Rgb(192, 252, 4);        // --acc-lime         #c0fc04 — success, accepted
pub const WARNING: Color = Color::Rgb(232, 197, 71);    // --acc-warning      #e8c547 — burn rate alert

// ── Legacy aliases (keep older references compiling) ────────────────────────
pub const AMBER: Color = MAGENTA;                        // was #f5a623, now maps to MAGENTA
pub const VIOLET: Color = ELECTRIC_BLUE;                 // was #7b5ea7, now maps to ELECTRIC_BLUE
pub const DATA: Color = MAGENTA;                         // primary accent
pub const SYSTEM: Color = CYAN;                          // system identity
pub const ALERT: Color = CORAL;                          // errors/alerts
pub const STATE: Color = LIME;                           // state announcements
pub const GHOST: Color = DIM;                            // inactive/ghost elements

// ── Semantic styles ─────────────────────────────────────────────────────────
pub fn data_style() -> Style {
    Style::default().fg(MAGENTA).bg(BG)
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
    Style::default().fg(MAGENTA).bg(BG)
}

pub fn label_style() -> Style {
    Style::default().fg(MAGENTA).bg(BG).add_modifier(Modifier::BOLD)
}

pub fn text_style() -> Style {
    Style::default().fg(TEXT).bg(BG)
}

pub fn muted_style() -> Style {
    Style::default().fg(MUTED).bg(BG)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn t_palette_1_marathon_neons_exist() {
        assert_eq!(MAGENTA, Color::Rgb(234, 2, 126));
        assert_eq!(ELECTRIC_BLUE, Color::Rgb(54, 1, 251));
        assert_eq!(LIME, Color::Rgb(192, 252, 4));
    }

    #[test]
    fn t_palette_2_legacy_aliases_resolve() {
        assert_eq!(DATA, MAGENTA);
        assert_eq!(SYSTEM, CYAN);
        assert_eq!(ALERT, CORAL);
        assert_eq!(STATE, LIME);
        assert_eq!(AMBER, MAGENTA);
        assert_eq!(VIOLET, ELECTRIC_BLUE);
    }

    #[test]
    fn t_palette_3_semantic_styles_use_new_accents() {
        let ds = data_style();
        assert_eq!(ds.fg, Some(MAGENTA));
        let is = input_style();
        assert_eq!(is.fg, Some(MAGENTA));
    }
}
