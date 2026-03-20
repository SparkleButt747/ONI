use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// NeonGlow — afterglow trail behind the scan-reveal frontier.
pub struct NeonGlow {
    /// Current reveal progress (0.0 to 1.0).
    pub progress: f32,
    /// How many columns wide the glow trail extends behind the frontier.
    pub trail_width: u16,
    /// Glow colour.
    pub color: Color,
}

impl Default for NeonGlow {
    fn default() -> Self {
        Self {
            progress: 0.0,
            trail_width: 8,
            color: palette::MAGENTA,
        }
    }
}

impl Widget for NeonGlow {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.progress >= 1.0 || self.progress <= 0.0 {
            return;
        }

        let frontier_col = (self.progress * area.width as f32) as u16;

        let (gr, gg, gb) = match self.color {
            Color::Rgb(r, g, b) => (r, g, b),
            _ => (234, 2, 126), // MAGENTA fallback
        };

        for y in area.y..area.y + area.height {
            for dx in 0..self.trail_width {
                let col = frontier_col.saturating_sub(dx);
                let x = area.x + col;
                if x < area.x || x >= area.x + area.width {
                    continue;
                }

                let intensity = 1.0 - (dx as f32 / self.trail_width as f32);

                let glow_r = (gr as f32 * intensity * 0.3) as u8;
                let glow_g = (gg as f32 * intensity * 0.3) as u8;
                let glow_b = (gb as f32 * intensity * 0.3) as u8;

                if let Some(cell) = buf.cell_mut((x, y)) {
                    let (fr, fg_g, fb) = match cell.style().fg {
                        Some(Color::Rgb(r, g, b)) => (r, g, b),
                        _ => (200, 197, 187),
                    };
                    let new_r = fr.saturating_add(glow_r);
                    let new_g = fg_g.saturating_add(glow_g);
                    let new_b = fb.saturating_add(glow_b);
                    cell.set_fg(Color::Rgb(new_r, new_g, new_b));
                }
            }
        }
    }
}
