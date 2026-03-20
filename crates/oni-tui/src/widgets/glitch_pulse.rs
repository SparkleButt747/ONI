use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Glitch pulse effect — shifts cells horizontally by ±1-3 positions.
/// Applied as an overlay on error states for 2-3 frames.
pub struct GlitchPulse {
    /// Frame counter (0-2). Effect is visible for 3 frames then gone.
    pub frame: u8,
}

impl Widget for GlitchPulse {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.frame > 2 || area.width < 4 || area.height == 0 {
            return;
        }
        // Shift direction alternates per frame
        let shift: i16 = match self.frame {
            0 => 2,
            1 => -3,
            2 => 1,
            _ => 0,
        };

        let glitch_style = Style::default()
            .fg(palette::CORAL)
            .bg(palette::BG);

        // Apply shift to a few rows (every other row for digital feel)
        for y in area.y..area.y + area.height {
            if (y - area.y) % 2 != 0 {
                continue;
            }
            // Glitch a stripe of cells
            let start = if shift > 0 {
                area.x
            } else {
                area.x + area.width.saturating_sub(shift.unsigned_abs() as u16)
            };
            let end = start + shift.unsigned_abs() as u16;
            for x in start..end.min(area.x + area.width) {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char('\u{2588}'); // █
                    cell.set_style(glitch_style);
                }
            }
        }
    }
}
