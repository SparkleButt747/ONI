use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

/// Large numeric/short-text display using 3x5 block character grids.
pub struct BigText {
    pub text: String,
    pub style: Style,
}

impl BigText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: Style::default().fg(palette::DATA).bg(palette::BG),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

// 3-wide x 5-tall bitmap for each digit 0-9 and a few extras.
// Each inner array is 5 rows; each row is 3 bools (left to right).
const DIGIT_MAPS: [[[bool; 3]; 5]; 11] = [
    // 0
    [
        [true, true, true],
        [true, false, true],
        [true, false, true],
        [true, false, true],
        [true, true, true],
    ],
    // 1
    [
        [false, true, false],
        [true, true, false],
        [false, true, false],
        [false, true, false],
        [true, true, true],
    ],
    // 2
    [
        [true, true, true],
        [false, false, true],
        [true, true, true],
        [true, false, false],
        [true, true, true],
    ],
    // 3
    [
        [true, true, true],
        [false, false, true],
        [false, true, true],
        [false, false, true],
        [true, true, true],
    ],
    // 4
    [
        [true, false, true],
        [true, false, true],
        [true, true, true],
        [false, false, true],
        [false, false, true],
    ],
    // 5
    [
        [true, true, true],
        [true, false, false],
        [true, true, true],
        [false, false, true],
        [true, true, true],
    ],
    // 6
    [
        [true, true, true],
        [true, false, false],
        [true, true, true],
        [true, false, true],
        [true, true, true],
    ],
    // 7
    [
        [true, true, true],
        [false, false, true],
        [false, false, true],
        [false, false, true],
        [false, false, true],
    ],
    // 8
    [
        [true, true, true],
        [true, false, true],
        [true, true, true],
        [true, false, true],
        [true, true, true],
    ],
    // 9
    [
        [true, true, true],
        [true, false, true],
        [true, true, true],
        [false, false, true],
        [true, true, true],
    ],
    // index 10 — '.' / unknown (single dot at bottom)
    [
        [false, false, false],
        [false, false, false],
        [false, false, false],
        [false, false, false],
        [false, true, false],
    ],
];

const CHAR_WIDTH: u16 = 3;
const CHAR_HEIGHT: u16 = 5;
const CHAR_GAP: u16 = 1;
const PIXEL_FULL: char = '█';

fn char_to_map_idx(c: char) -> Option<usize> {
    match c {
        '0'..='9' => Some(c as usize - '0' as usize),
        '.' => Some(10),
        _ => None,
    }
}

impl Widget for BigText {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height < CHAR_HEIGHT {
            return;
        }

        let mut cursor_x = area.x;

        for ch in self.text.chars() {
            let map_idx = char_to_map_idx(ch);

            // Check if there's room for the next glyph
            if cursor_x + CHAR_WIDTH > area.x + area.width {
                break;
            }

            if let Some(idx) = map_idx {
                let grid = &DIGIT_MAPS[idx];
                for (row_idx, row) in grid.iter().enumerate() {
                    let y = area.y + row_idx as u16;
                    if y >= area.y + area.height {
                        break;
                    }
                    for (col_idx, &on) in row.iter().enumerate() {
                        let x = cursor_x + col_idx as u16;
                        if x >= area.x + area.width {
                            break;
                        }
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            if on {
                                cell.set_char(PIXEL_FULL);
                                cell.set_style(self.style);
                            } else {
                                cell.set_char(' ');
                                cell.set_bg(palette::BG);
                            }
                        }
                    }
                }
            } else {
                // Render unknown chars as a space glyph
                for row_idx in 0..CHAR_HEIGHT as usize {
                    let y = area.y + row_idx as u16;
                    if y >= area.y + area.height {
                        break;
                    }
                    for col_idx in 0..CHAR_WIDTH as usize {
                        let x = cursor_x + col_idx as u16;
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            cell.set_char(' ');
                            cell.set_bg(palette::BG);
                        }
                    }
                }
            }

            cursor_x += CHAR_WIDTH + CHAR_GAP;
        }
    }
}
