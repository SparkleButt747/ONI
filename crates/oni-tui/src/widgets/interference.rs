use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// Interference — subtle horizontal scanlines that drift downward.
pub struct Interference {
    /// Frame counter for drift.
    pub frame: u64,
    /// Spacing between scanlines (rows).
    pub spacing: u16,
    /// Opacity (0.0 = invisible, 1.0 = fully dark).
    pub opacity: f32,
}

impl Default for Interference {
    fn default() -> Self {
        Self {
            frame: 0,
            spacing: 4,
            opacity: 0.15,
        }
    }
}

impl Widget for Interference {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.spacing == 0 {
            return;
        }

        let offset = (self.frame % self.spacing as u64) as u16;

        for y in area.y..area.y + area.height {
            let local_y = y - area.y;
            if (local_y + offset) % self.spacing != 0 {
                continue;
            }

            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    let (r, g, b) = match cell.style().fg {
                        Some(Color::Rgb(r, g, b)) => (r, g, b),
                        _ => continue,
                    };
                    let factor = 1.0 - self.opacity;
                    let new_r = (r as f32 * factor) as u8;
                    let new_g = (g as f32 * factor) as u8;
                    let new_b = (b as f32 * factor) as u8;
                    let bg = match cell.style().bg {
                        Some(bg) => bg,
                        None => palette::BG,
                    };
                    cell.set_fg(Color::Rgb(new_r, new_g, new_b));
                    cell.set_bg(bg);
                }
            }
        }
    }
}
