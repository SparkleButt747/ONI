use crate::app::{App, LearnedRule};
use oni_core::palette;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

fn confidence_tag(confidence: f32) -> (&'static str, Color) {
    if confidence >= 0.80 {
        ("ACTIVE", palette::LIME)
    } else if confidence >= 0.50 {
        ("LEARNING", palette::AMBER)
    } else {
        ("WEAK", palette::DIM)
    }
}

fn rule_color(confidence: f32) -> Color {
    if confidence >= 0.80 {
        palette::LIME
    } else if confidence >= 0.50 {
        palette::AMBER
    } else {
        palette::DIM
    }
}

fn divider_line(width: u16) -> Line<'static> {
    let s = "─".repeat(width as usize);
    Line::from(Span::styled(s, Style::default().fg(palette::GHOST).bg(palette::BG)))
}

fn render_rule<'a>(rule: &'a LearnedRule, width: u16) -> Vec<Line<'a>> {
    let color = rule_color(rule.confidence);
    let (tag, tag_color) = confidence_tag(rule.confidence);

    let pct_str = format!("{:.0}%", rule.confidence * 100.0);
    let meta_str = format!("{} · {} OBS", rule.context, rule.observations);

    // Description line
    let desc_line = Line::from(Span::styled(
        format!(" {}", rule.description),
        Style::default().fg(palette::TEXT).bg(palette::BG).add_modifier(Modifier::BOLD),
    ));

    // Meta + confidence + tag on one line.
    // We right-align the pct + tag within the available width.
    // Meta is left, pct + tag is right.
    let right_part = format!("{}  {} ", pct_str, tag);
    let left_part = format!(" {} ", meta_str);
    let gap = (width as usize)
        .saturating_sub(left_part.len() + right_part.len());
    let padding = " ".repeat(gap);

    let meta_line = Line::from(vec![
        Span::styled(left_part, Style::default().fg(palette::MUTED).bg(palette::BG)),
        Span::styled(padding, Style::default().bg(palette::BG)),
        Span::styled(
            format!("{:.0}%", rule.confidence * 100.0),
            Style::default()
                .fg(color)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "  ",
            Style::default().bg(palette::BG),
        ),
        Span::styled(
            format!("{} ", tag),
            Style::default()
                .fg(palette::BG)
                .bg(tag_color)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    vec![desc_line, meta_line]
}

pub fn draw_preferences(app: &App, frame: &mut Frame, area: Rect) {
    let width = area.width;
    let rules = &app.learned_rules;

    // Count active and below-threshold rules
    let active = rules.iter().filter(|r| r.confidence >= 0.80).count();
    let learning = rules.iter().filter(|r| r.confidence >= 0.50 && r.confidence < 0.80).count();
    let weak = rules.iter().filter(|r| r.confidence < 0.50).count();
    let total = rules.len();

    let mut lines: Vec<Line> = Vec::new();

    // Header — inverse AMBER section label
    let header_left = " LEARNED_RULES ";
    let header_right = format!(
        " {} RULES \u{00b7} {} ACTIVE \u{00b7} {} LEARNING \u{00b7} {} WEAK ",
        total, active, learning, weak
    );
    let gap_len = (width as usize)
        .saturating_sub(header_left.len() + header_right.len());
    let gap = " ".repeat(gap_len);

    lines.push(Line::from(vec![
        Span::styled(
            header_left,
            Style::default()
                .fg(palette::BG)
                .bg(palette::AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(gap, Style::default().bg(palette::BG)),
        Span::styled(
            header_right,
            Style::default().fg(palette::MUTED).bg(palette::BG),
        ),
    ]));

    lines.push(divider_line(width));

    for rule in rules {
        let rule_lines = render_rule(rule, width);
        lines.extend(rule_lines);
        lines.push(divider_line(width));
    }

    // Empty state
    if rules.is_empty() {
        lines.push(Line::from(Span::styled(
            " NO_RULES_LEARNED — INTERACT WITH ONI TO BUILD PREFERENCE MODEL",
            Style::default().fg(palette::DIM).bg(palette::BG),
        )));
        lines.push(divider_line(width));
    }

    // Spacer to push footer toward bottom
    let content_len = lines.len() as u16;
    let footer_height: u16 = 1;
    let spacer = area.height.saturating_sub(content_len + footer_height);
    for _ in 0..spacer {
        lines.push(Line::default());
    }

    // Footer keybinds
    lines.push(Line::from(vec![
        Span::styled(
            " /PREFS·RESET ",
            Style::default()
                .fg(palette::DIM)
                .bg(palette::BG),
        ),
        Span::styled(
            "\u{00b7} ",
            Style::default().fg(palette::BORDER).bg(palette::BG),
        ),
        Span::styled(
            "/PREFS·EXPORT ",
            Style::default()
                .fg(palette::DIM)
                .bg(palette::BG),
        ),
        Span::styled(
            "\u{00b7} ",
            Style::default().fg(palette::BORDER).bg(palette::BG),
        ),
        Span::styled(
            "/CHAT RETURN",
            Style::default()
                .fg(palette::AMBER)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    let paragraph = Paragraph::new(Text::from(lines))
        .style(Style::default().bg(palette::BG));

    frame.render_widget(paragraph, area);
}
