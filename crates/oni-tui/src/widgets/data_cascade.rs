use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// DataCascade — vertical data rain for screen transitions.
pub struct DataCascade {
    pub frame: u64,
    pub density: f32,
    pub color: Color,
}

impl Default for DataCascade {
    fn default() -> Self {
        Self {
            frame: 0,
            density: 0.3,
            color: palette::LIME,
        }
    }
}

fn cascade_hash(col: u16, seed: u64) -> u64 {
    let mut h = col as u64 * 2654435761 + seed;
    h ^= h >> 16;
    h = h.wrapping_mul(0x45d9f3b);
    h ^= h >> 16;
    h
}

const CASCADE_CHARS: [char; 10] = [
    '\u{2593}', '\u{2592}', '\u{2591}', '\u{2588}', '\u{2584}',
    '\u{2580}', '\u{258C}', '\u{2590}', '\u{25A0}', '\u{25A1}',
];

impl Widget for DataCascade {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        for col_offset in 0..area.width {
            let hash = cascade_hash(col_offset, 42);
            if (hash % 100) as f32 / 100.0 > self.density {
                continue;
            }

            let speed = 1 + (hash % 3) as u64;
            let head_row = ((self.frame * speed) % (area.height as u64 * 2)) as u16;
            let trail_len = 3 + (hash % 5) as u16;

            for dy in 0..trail_len {
                let row = head_row.wrapping_sub(dy);
                if row >= area.height {
                    continue;
                }

                let x = area.x + col_offset;
                let y = area.y + row;

                let char_idx =
                    cascade_hash(col_offset, self.frame + dy as u64) as usize % CASCADE_CHARS.len();
                let intensity = 1.0 - (dy as f32 / trail_len as f32);

                let (r, g, b) = match self.color {
                    Color::Rgb(r, g, b) => (r, g, b),
                    _ => (192, 252, 4), // LIME fallback
                };
                let cr = (r as f32 * intensity) as u8;
                let cg = (g as f32 * intensity) as u8;
                let cb = (b as f32 * intensity) as u8;

                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(CASCADE_CHARS[char_idx]);
                    cell.set_fg(Color::Rgb(cr, cg, cb));
                    cell.set_bg(palette::BG);
                }
            }
        }
    }
}
