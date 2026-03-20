use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

/// Narrow 2-column widget that renders "RESPONSE" vertically in green on the
/// right edge of the chat area. The text runs top-to-bottom, one character per
/// row, centred in the 2-char column.
pub struct ResponseLabel;

impl Widget for ResponseLabel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height == 0 {
            return;
        }

        const LABEL: &[u8] = b"RESPONSE";

        let label_len = LABEL.len() as u16;

        // Vertical centre within the available height
        let top_pad = area.height.saturating_sub(label_len) / 2;

        let style = Style::default()
            .fg(palette::DATA)
            .bg(palette::BG)
            .add_modifier(Modifier::BOLD);

        let dim_style = Style::default()
            .fg(palette::GHOST)
            .bg(palette::BG);

        // Fill column with ghost background first
        for row in 0..area.height {
            let y = area.y + row;
            for col in 0..area.width {
                let x = area.x + col;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(dim_style);
                }
            }
        }

        // Write each letter of RESPONSE going down
        for (i, &byte) in LABEL.iter().enumerate() {
            let row = top_pad + i as u16;
            if row >= area.height {
                break;
            }
            let y = area.y + row;
            // Write to the left cell of the 2-char column (centre char)
            let x = area.x;
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_char(byte as char);
                cell.set_style(style);
            }
        }
    }
}
