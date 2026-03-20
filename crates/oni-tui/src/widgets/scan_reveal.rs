use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

/// Scan entry animation — reveals content left-to-right in steps.
/// Each tick reveals more columns. Feels like data being loaded.
pub struct ScanReveal {
    /// How many columns have been revealed (0 = nothing visible).
    pub revealed_cols: u16,
}

impl Widget for ScanReveal {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Mask cells beyond the reveal frontier by setting them to space
        // with dim background. Content before the frontier is left untouched.
        if self.revealed_cols >= area.width {
            return; // Fully revealed, nothing to mask
        }
        let mask_start = area.x + self.revealed_cols;
        let style = ratatui::style::Style::default()
            .fg(oni_core::palette::DIM)
            .bg(oni_core::palette::BG);
        for y in area.y..area.y + area.height {
            for x in mask_start..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_style(style);
                }
            }
        }
    }
}
