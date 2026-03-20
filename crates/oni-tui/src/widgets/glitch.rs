use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::widgets::Widget;

/// Decorative glitch-noise blocks. Reproducible via seed.
pub struct GlitchBlocks {
    pub seed: u64,
    pub density: f32, // 0.0-1.0
    pub color: Color,
}

impl GlitchBlocks {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,
            density: 0.3,
            color: palette::DATA,
        }
    }

    pub fn density(mut self, density: f32) -> Self {
        self.density = density.clamp(0.0, 1.0);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

// Simple LCG — fast, no deps, reproducible
#[inline]
fn lcg(state: u64) -> u64 {
    state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407)
}

// Glitch character palette
const GLITCH_CHARS: [char; 8] = ['▓', '▒', '░', '▄', '▀', '▌', '▐', '█'];

impl Widget for GlitchBlocks {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let threshold = (self.density * u32::MAX as f32) as u32;
        let mut rng = self.seed;

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                rng = lcg(rng);
                let roll = (rng >> 32) as u32;

                if let Some(cell) = buf.cell_mut((x, y)) {
                    if roll < threshold {
                        // Pick a glitch character
                        rng = lcg(rng);
                        let char_idx = ((rng >> 32) as usize) % GLITCH_CHARS.len();
                        cell.set_char(GLITCH_CHARS[char_idx]);
                        cell.set_fg(self.color);
                        cell.set_bg(palette::BG);
                    } else {
                        cell.set_char(' ');
                        cell.set_bg(palette::BG);
                    }
                }
            }
        }
    }
}
