use crate::app::{App, SLASH_COMMANDS};
use oni_core::palette;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

/// Returns SLASH_COMMANDS filtered to entries whose command string starts with `filter`.
pub fn filtered_commands(filter: &str) -> Vec<(&'static str, &'static str)> {
    SLASH_COMMANDS
        .iter()
        .filter(|(cmd, _)| filter.is_empty() || cmd.starts_with(filter))
        .copied()
        .collect()
}

pub fn draw_command_menu(app: &App, frame: &mut Frame, content_area: Rect) {
    let filtered = filtered_commands(&app.slash_menu_filter);
    if filtered.is_empty() {
        return;
    }

    let item_count = filtered.len() as u16;
    // 1 header row + one row per command
    let popup_height = item_count + 1;
    // Fixed width, clamped to available terminal space
    let popup_width: u16 = 44_u16.min(content_area.width.saturating_sub(2));

    // Anchor bottom-left of the popup to just above the input area
    // (content_area.bottom() is the y-coord of the first row *below* content)
    let popup_y = content_area.bottom().saturating_sub(popup_height);
    let popup_x = content_area.x + 1;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Erase whatever was underneath for crisp negative-space separation
    frame.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::with_capacity(popup_height as usize);

    // ── Header ──────────────────────────────────────────────────────────────
    lines.push(Line::from(Span::styled(
        format!(" {:<width$}", "COMMANDS", width = popup_width as usize - 1),
        Style::default()
            .fg(palette::STATE)
            .bg(palette::GHOST)
            .add_modifier(Modifier::BOLD),
    )));

    // ── Command rows ────────────────────────────────────────────────────────
    let selected = app.slash_menu_selected.min(filtered.len().saturating_sub(1));

    for (i, (cmd, desc)) in filtered.iter().enumerate() {
        let is_selected = i == selected;

        let row_bg = if is_selected { palette::STATE } else { palette::GHOST };

        let cmd_style = if is_selected {
            Style::default()
                .fg(palette::BG)
                .bg(row_bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette::DATA).bg(row_bg)
        };

        let desc_style = if is_selected {
            Style::default().fg(palette::BG).bg(row_bg)
        } else {
            Style::default().fg(palette::DIM).bg(row_bg)
        };

        // ALL_CAPS command name left-padded, description right-aligned
        let cmd_upper = cmd.to_uppercase();
        // " /CMD " takes cmd_upper.len() + 2 chars
        // "DESC " takes desc.len() + 1 char
        let left = format!(" {}", cmd_upper);
        let right = format!("  {}", desc);
        let gap_len = (popup_width as usize)
            .saturating_sub(left.len() + right.len());
        let gap = " ".repeat(gap_len);

        lines.push(Line::from(vec![
            Span::styled(left, cmd_style),
            Span::styled(gap, Style::default().bg(row_bg)),
            Span::styled(right, desc_style),
        ]));
    }

    let popup = Paragraph::new(Text::from(lines));
    frame.render_widget(popup, popup_area);
}
