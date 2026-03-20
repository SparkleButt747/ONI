use oni_core::palette;
use ratatui::style::Color;

/// Returns the current border color for active task pulse.
/// Alternates between BORDER and AMBER on a 2-second cycle.
/// `tick` is the frame counter from the main event loop.
pub fn active_border_color(tick: u64, fps: u32) -> Color {
    let cycle_frames = fps as u64 * 2; // 2-second cycle
    if cycle_frames == 0 {
        return palette::AMBER;
    }
    if (tick % cycle_frames) < (cycle_frames / 2) {
        palette::AMBER
    } else {
        palette::BORDER
    }
}
