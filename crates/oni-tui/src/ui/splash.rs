use crate::app::App;
use oni_core::palette;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

const ONI_LOGO: &str = r#"
  ██████  ███    ██ ██
 ██    ██ ████   ██ ██
 ██    ██ ██ ██  ██ ██
 ██    ██ ██  ██ ██ ██
  ██████  ██   ████ ██
"#;

const TAGLINE: &str = "ONBOARD_NATIVE_INTELLIGENCE";
const VERSION: &str = "V0.1.0";

pub fn draw_splash(app: &App, frame: &mut Frame, area: Rect) {
    // Clean boot screen — logo, version, model info, keybind hints
    let logo_lines: Vec<&str> = ONI_LOGO.lines().filter(|l| !l.is_empty()).collect();

    // Vertically centre the content
    let content_height = logo_lines.len() + 6; // logo + spacing + subtitle + model + blank + hints + cursor
    let top_pad = area.height.saturating_sub(content_height as u16) / 2;

    let mut lines: Vec<Line> = Vec::new();

    for _ in 0..top_pad {
        lines.push(Line::default());
    }

    // Logo
    for line in &logo_lines {
        lines.push(Line::from(Span::styled(
            line.to_string(),
            Style::default()
                .fg(palette::MAGENTA)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::default());

    // Subtitle + version
    lines.push(Line::from(vec![
        Span::styled(
            format!("  {} ", TAGLINE),
            Style::default()
                .fg(palette::MUTED)
                .bg(palette::BG),
        ),
        Span::styled(
            format!("\u{2502} {}", VERSION),
            Style::default()
                .fg(palette::DIM)
                .bg(palette::BG),
        ),
    ]));

    // Model info
    let model_upper = app.current_model_name.to_uppercase();
    lines.push(Line::from(Span::styled(
        format!("  MODEL: {} \u{2502} TIER: {}", model_upper, app.current_tier.display_name()),
        Style::default()
            .fg(palette::DIM)
            .bg(palette::BG),
    )));

    lines.push(Line::default());

    // Keybind hints
    lines.push(Line::from(Span::styled(
        "  /help commands \u{00B7} /tier model \u{00B7} /quit exit",
        Style::default()
            .fg(palette::DIM)
            .bg(palette::BG),
    )));

    // Cursor ready hint
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "  TYPE A MESSAGE TO BEGIN  \u{258c}",
        Style::default()
            .fg(palette::TEXT)
            .bg(palette::BG),
    )));

    let paragraph = Paragraph::new(Text::from(lines))
        .alignment(Alignment::Left)
        .style(Style::default().bg(palette::BG));

    frame.render_widget(paragraph, area);
}
