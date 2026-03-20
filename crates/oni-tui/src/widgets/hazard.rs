use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Hazard divider — amber/dark repeating stripe pattern.
/// Marathon industrial safety signage aesthetic.
pub struct HazardDivider;

impl Widget for HazardDivider {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let amber = Style::default().fg(palette::AMBER).bg(palette::BG);
        let dark = Style::default().fg(palette::BORDER).bg(palette::BG);
        // Repeating pattern: 4 amber blocks, 1 dark, 4 amber blocks, 1 dark...
        for x in area.x..area.x + area.width {
            let pos = (x - area.x) as usize;
            let in_amber = (pos % 5) < 4; // 4 amber, 1 gap
            let (ch, style) = if in_amber {
                ('\u{2588}', amber) // █
            } else {
                ('\u{2591}', dark) // ░
            };
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char(ch);
                cell.set_style(style);
            }
        }
    }
}
