use crate::app::App;
use oni_core::palette;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn draw_sidebar(app: &App, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // Section: MODEL
    lines.push(Line::from(Span::styled(
        " MODEL ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::AMBER)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!(" {} ", app.current_tier.display_name()),
        Style::default()
            .fg(palette::TEXT)
            .bg(palette::BG)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!(" {} ", app.current_model_name.to_uppercase()),
        Style::default().fg(palette::MUTED).bg(palette::BG),
    )));
    lines.push(Line::default());

    // Section: STATS
    lines.push(Line::from(Span::styled(
        " STATS ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::AMBER)
            .add_modifier(Modifier::BOLD),
    )));

    let tok_s = if app.last_tokens_per_sec > 0.0 {
        format!("{:.1}", app.last_tokens_per_sec)
    } else {
        "--".into()
    };

    let stat_style = Style::default().fg(palette::MUTED).bg(palette::BG);
    let val_style = Style::default().fg(palette::TEXT).bg(palette::BG);

    lines.push(Line::from(vec![
        Span::styled(" TOKENS  ", stat_style),
        Span::styled(format!("{}", app.total_tokens), val_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" TOK/S   ", stat_style),
        Span::styled(tok_s, val_style),
    ]));
    lines.push(Line::from(vec![
        Span::styled(" TURNS   ", stat_style),
        Span::styled(format!("{}", app.turn_count), val_style),
    ]));
    lines.push(Line::default());

    // Section: TOOLS
    lines.push(Line::from(Span::styled(
        " TOOLS ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::CYAN)
            .add_modifier(Modifier::BOLD),
    )));

    for name in &app.tool_names {
        lines.push(Line::from(Span::styled(
            format!(" {} ", name.to_uppercase()),
            Style::default().fg(palette::LIME).bg(palette::BG),
        )));
    }
    lines.push(Line::default());

    // Section: TIERS
    lines.push(Line::from(Span::styled(
        " TIERS ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::AMBER)
            .add_modifier(Modifier::BOLD),
    )));

    let tiers = [
        ("H", "HEAVY", oni_core::types::ModelTier::Heavy),
        ("C", "CODE", oni_core::types::ModelTier::Medium),
        ("G", "GENERAL", oni_core::types::ModelTier::General),
        ("F", "FAST", oni_core::types::ModelTier::Fast),
    ];

    for (key, label, tier) in &tiers {
        let is_active = *tier == app.current_tier;
        let indicator = if is_active { ">" } else { " " };
        let style = if is_active {
            Style::default()
                .fg(palette::LIME)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette::MUTED).bg(palette::BG)
        };
        lines.push(Line::from(Span::styled(
            format!(" {}{} {} ", indicator, key, label),
            style,
        )));
    }
    lines.push(Line::default());

    // Section: KEYBINDS
    lines.push(Line::from(Span::styled(
        " KEYS ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::AMBER)
            .add_modifier(Modifier::BOLD),
    )));
    let keys = [
        ("ENTER", "SEND"),
        ("CTRL+C", "QUIT"),
        ("/", "COMMANDS"),
    ];
    for (key, action) in &keys {
        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", key),
                Style::default()
                    .fg(palette::AMBER)
                    .bg(palette::BG),
            ),
            Span::styled(
                format!("{} ", action),
                Style::default().fg(palette::MUTED).bg(palette::BG),
            ),
        ]));
    }

    // Fill remaining space with dim texture
    let remaining = area.height.saturating_sub(lines.len() as u16);
    for _ in 0..remaining {
        lines.push(Line::default());
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(palette::BG));

    frame.render_widget(paragraph, area);
}
