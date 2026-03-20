use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// Dense bar chart for token rates or timing data.
/// Bars are rendered using Unicode block elements, no labels.
pub struct Spectrum {
    pub values: Vec<u16>, // bar heights 0-100
    pub max_height: u16,
    pub color: Color,
}

impl Spectrum {
    pub fn new(values: Vec<u16>) -> Self {
        Self {
            values,
            max_height: 100,
            color: palette::DATA,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn max_height(mut self, max_height: u16) -> Self {
        self.max_height = max_height;
        self
    }
}

// Unicode vertical block elements from 1/8 to full block
const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

impl Widget for Spectrum {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.values.is_empty() {
            return;
        }

        let max = self.max_height.max(1) as f32;
        let total_rows = area.height as f32;

        for (col_idx, &raw_val) in self.values.iter().enumerate().take(area.width as usize) {
            let x = area.x + col_idx as u16;
            let normalized = (raw_val as f32 / max).clamp(0.0, 1.0);
            // How many sub-rows (each row = 8 eighths) are filled
            let filled_eighths = (normalized * total_rows * 8.0).round() as u32;

            for row in 0..area.height {
                let y = area.y + (area.height - 1 - row); // bottom-up
                let row_base_eighths = row as u32 * 8;
                let cell = buf.cell_mut((x, y));

                if let Some(cell) = cell {
                    if filled_eighths >= row_base_eighths + 8 {
                        // Full block
                        cell.set_char(BLOCKS[8]);
                        cell.set_fg(self.color);
                        cell.set_bg(palette::BG);
                    } else if filled_eighths > row_base_eighths {
                        // Partial block
                        let partial = (filled_eighths - row_base_eighths) as usize;
                        cell.set_char(BLOCKS[partial.min(8)]);
                        cell.set_fg(self.color);
                        cell.set_bg(palette::BG);
                    } else {
                        cell.set_char(' ');
                        cell.set_bg(palette::BG);
                    }
                }
            }
        }

        // Fill any remaining columns with empty space
        for col_idx in self.values.len()..area.width as usize {
            let x = area.x + col_idx as u16;
            for row in 0..area.height {
                let y = area.y + row;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(' ');
                    cell.set_bg(palette::BG);
                }
            }
        }
    }
}
