use crate::app::App;
use crate::theme;
use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;
use throbber_widgets_tui::Throbber;

/// Tiled "PROCESSING_" texture filling the thinking region.
struct ProcessingTexture;

impl Widget for ProcessingTexture {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        const PAT: &[u8] = b"PROCESSING_";
        let pat_len = PAT.len();
        let style = Style::default()
            .fg(palette::DIM)
            .bg(palette::BG);

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let idx = ((x - area.x) as usize + (y - area.y) as usize * 4) % pat_len;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(PAT[idx] as char);
                    cell.set_style(style);
                }
            }
        }
    }
}

pub fn draw_thinking(app: &mut App, frame: &mut Frame, area: Rect) {
    // Draw existing messages first if any
    if !app.messages.is_empty() {
        let msg_area = Rect {
            height: area.height.saturating_sub(3),
            ..area
        };
        super::chat::draw_chat(app, frame, msg_area);
    }

    // The bottom 3 rows are the thinking zone
    let thinking_zone = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(3),
        width: area.width,
        height: 3,
    };

    // Fill thinking zone with PROCESSING_ texture
    frame.render_widget(ProcessingTexture, thinking_zone);

    // "PROCESSING" label with model name
    let label_area = Rect {
        x: thinking_zone.x,
        y: thinking_zone.y,
        width: thinking_zone.width,
        height: 1,
    };
    let model_upper = app.current_model_name.to_uppercase();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                " PROCESSING ",
                Style::default()
                    .fg(palette::BG)
                    .bg(palette::AMBER)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("  {} ", model_upper),
                Style::default()
                    .fg(palette::MUTED)
                    .bg(palette::BG),
            ),
        ]))
        .alignment(Alignment::Center),
        label_area,
    );

    // Throbber spinner on the third row, indented
    let throbber_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(1),
        width: area.width.saturating_sub(4),
        height: 1,
    };

    let throbber = Throbber::default()
        .label(" PROCESSING ")
        .style(theme::data())
        .throbber_style(
            Style::default()
                .fg(palette::AMBER)
                .bg(palette::BG),
        );

    frame.render_stateful_widget(throbber, throbber_area, &mut app.throbber_state);
}
