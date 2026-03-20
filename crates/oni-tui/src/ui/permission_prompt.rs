use crate::app::PendingProposal;
use oni_core::palette;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Render the permission prompt inline.
/// Shows: tool name, summary, and tiered option bar.
pub fn draw_permission_prompt(proposal: &PendingProposal, frame: &mut Frame, area: Rect) {
    if area.height < 4 {
        return;
    }

    let prompt_area = Rect {
        x: area.x + 2,
        y: area.y,
        width: area.width.saturating_sub(4),
        height: 4,
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(
                " CONFIRM ",
                Style::default()
                    .fg(palette::BG)
                    .bg(palette::MAGENTA)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {}  ", proposal.name.to_uppercase()),
                Style::default()
                    .fg(palette::CYAN)
                    .bg(palette::BG)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            format!("  {}", proposal.summary),
            Style::default().fg(palette::TEXT).bg(palette::BG),
        )),
        Line::default(),
        Line::from(vec![
            Span::styled("  [Y] ", Style::default().fg(palette::LIME).bg(palette::BG).add_modifier(Modifier::BOLD)),
            Span::styled("ONCE  ", Style::default().fg(palette::TEXT).bg(palette::BG)),
            Span::styled("[S] ", Style::default().fg(palette::LIME).bg(palette::BG).add_modifier(Modifier::BOLD)),
            Span::styled("SESSION  ", Style::default().fg(palette::TEXT).bg(palette::BG)),
            Span::styled("[A] ", Style::default().fg(palette::LIME).bg(palette::BG).add_modifier(Modifier::BOLD)),
            Span::styled("ALWAYS  ", Style::default().fg(palette::TEXT).bg(palette::BG)),
            Span::styled("[D] ", Style::default().fg(palette::ELECTRIC_BLUE).bg(palette::BG).add_modifier(Modifier::BOLD)),
            Span::styled("DIFF  ", Style::default().fg(palette::TEXT).bg(palette::BG)),
            Span::styled("[N] ", Style::default().fg(palette::CORAL).bg(palette::BG).add_modifier(Modifier::BOLD)),
            Span::styled("DENY", Style::default().fg(palette::TEXT).bg(palette::BG)),
        ]),
    ];

    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(palette::BG)),
        prompt_area,
    );
}
