use crate::app::App;
use oni_core::palette;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Top status bar: SYSTEM_ONI tag | session ID | model name
pub fn draw_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    let model_name = app.current_model_name.to_uppercase();

    let line = Line::from(vec![
        Span::styled(
            format!(" SYSTEM_ONI [{}] ", app.active_agent.to_uppercase()),
            Style::default()
                .fg(palette::BG)
                .bg(palette::AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " | ",
            Style::default().fg(palette::BORDER).bg(palette::BG),
        ),
        Span::styled(
            format!(" {} ", app.session_id),
            Style::default()
                .fg(palette::LIME)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " | ",
            Style::default().fg(palette::BORDER).bg(palette::BG),
        ),
        Span::styled(
            format!(" MODEL: {} ", model_name),
            Style::default()
                .fg(palette::BG)
                .bg(palette::AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", app.current_tier.display_name()),
            Style::default().fg(palette::AMBER).bg(palette::BG),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(palette::BG)),
        area,
    );
}

/// Bottom footer: TIER chip + CTX gauge
pub fn draw_footer(app: &App, frame: &mut Frame, area: Rect) {
    let ctx_budget = match app.current_tier {
        oni_core::types::ModelTier::Heavy | oni_core::types::ModelTier::Medium => 32768u64,
        oni_core::types::ModelTier::General => 16384u64,
        oni_core::types::ModelTier::Fast => 8192u64,
        oni_core::types::ModelTier::Embed => 2048u64,
    };
    let ctx_used = app.total_tokens.min(ctx_budget);
    let ctx_pct = (ctx_used as f32 / ctx_budget as f32 * 100.0) as u16;
    let gauge_width = 20usize;
    let filled = (ctx_pct as usize * gauge_width / 100).min(gauge_width);
    let gauge_color = if ctx_pct >= 80 {
        palette::CORAL
    } else if ctx_pct >= 60 {
        palette::WARNING
    } else {
        palette::AMBER
    };

    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", app.current_tier.display_name()),
            Style::default()
                .fg(palette::BG)
                .bg(palette::AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                " CTX [{}{}] {}% ",
                "\u{2588}".repeat(filled),
                "\u{2591}".repeat(gauge_width - filled),
                ctx_pct
            ),
            Style::default().fg(gauge_color).bg(palette::BG),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(line).style(Style::default().bg(palette::BG)),
        area,
    );
}
