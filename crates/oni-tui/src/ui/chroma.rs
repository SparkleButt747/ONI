use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Widget;

// Chroma stripe — DESIGN_SYSTEM accent palette mapped across width
const SEGMENTS: &[(Color, &str)] = &[
    (Color::Rgb(255, 77, 46),  "CORAL"),   // --acc-coral
    (Color::Rgb(245, 166, 35), "AMBER"),   // --acc-amber
    (Color::Rgb(232, 197, 71), "WARNING"), // --acc-warning
    (Color::Rgb(180, 224, 51), "LIME"),    // --acc-lime
    (Color::Rgb(0, 212, 200),  "CYAN"),    // --acc-cyan
    (Color::Rgb(123, 94, 167), "VIOLET"),  // --acc-violet
    (Color::Rgb(255, 77, 46),  "CORAL"),   // wrap
];

const FILL_CHAR: char = '▀';

pub struct ChromaStripe;

impl Widget for ChromaStripe {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let total = area.width as usize;
        let n = SEGMENTS.len();
        // Each segment gets an equal share; last segment absorbs any remainder
        let seg_width = total / n;

        for (i, (color, _)) in SEGMENTS.iter().enumerate() {
            let x_start = area.x + (i * seg_width) as u16;
            let x_end = if i == n - 1 {
                area.x + area.width
            } else {
                x_start + seg_width as u16
            };

            let style = Style::default().fg(*color).bg(Color::Rgb(10, 10, 9));

            for x in x_start..x_end {
                if let Some(cell) = buf.cell_mut((x, area.y)) {
                    cell.set_char(FILL_CHAR);
                    cell.set_style(style);
                }
            }
        }
    }
}
