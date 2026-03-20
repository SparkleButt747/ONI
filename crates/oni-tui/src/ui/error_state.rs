use crate::widgets::glitch_pulse::GlitchPulse;
use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

/// Tiled background texture for the error state — fills entire frame with dim repeating text.
struct ErrorTexture;

impl Widget for ErrorTexture {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        const PAT: &[u8] = b"FAIL_";
        let pat_len = PAT.len();
        let style = Style::default()
            .fg(palette::DIM)
            .bg(palette::BG);

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let idx = ((x - area.x) as usize + (y - area.y) as usize * 7) % pat_len;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(PAT[idx] as char);
                    cell.set_style(style);
                }
            }
        }
    }
}

/// Render a dramatic full-screen error state.
///
/// Layout (vertically centred in the frame):
///   - 2 blank lines
///   - `[ CRITICAL_FAILURE ]` banner in bold ALERT
///   - blank line
///   - error message in ALERT
///   - blank line
///   - details in DIM (wrapped)
///   - blank line
///   - recovery hint at bottom in STATE
pub fn draw_error_state(frame: &mut Frame, area: Rect, error: &str, glitch_frame: Option<u8>) {
    // Fill the whole area with the tiled texture first
    frame.render_widget(ErrorTexture, area);

    // GlitchPulse overlay during the first 3 frames of error transition
    if let Some(f) = glitch_frame {
        if f < 3 {
            frame.render_widget(GlitchPulse { frame: f }, area);
            return; // Only show glitch for these frames, not the static error yet
        }
    }

    // Split the error message into a headline and optional detail.
    // Convention: first line is the headline, rest is detail.
    let mut headline = error;
    let mut detail = "";
    if let Some(nl) = error.find('\n') {
        headline = &error[..nl];
        detail = error[nl + 1..].trim();
    }

    // Build recovery hint based on common error patterns
    let hint = recovery_hint(headline);

    let mut text_lines: Vec<Line> = Vec::new();

    // Blank padding
    text_lines.push(Line::default());
    text_lines.push(Line::default());

    // Banner — intense inverse CORAL
    text_lines.push(Line::from(Span::styled(
        " CRITICAL_FAILURE ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::CORAL)
            .add_modifier(Modifier::BOLD),
    )));

    text_lines.push(Line::default());

    // Error headline — uppercase, bold CORAL
    text_lines.push(Line::from(Span::styled(
        headline.to_uppercase(),
        Style::default()
            .fg(palette::CORAL)
            .bg(palette::BG)
            .add_modifier(Modifier::BOLD),
    )));

    text_lines.push(Line::default());

    // Detail lines in MUTED
    if !detail.is_empty() {
        for dl in detail.lines() {
            text_lines.push(Line::from(Span::styled(
                dl.to_owned(),
                Style::default()
                    .fg(palette::MUTED)
                    .bg(palette::BG),
            )));
        }
        text_lines.push(Line::default());
    }

    // Separator
    text_lines.push(Line::from(Span::styled(
        "\u{2500}".repeat(33),
        Style::default().fg(palette::BORDER).bg(palette::BG),
    )));

    text_lines.push(Line::default());

    // Recovery hint in LIME (actionable)
    text_lines.push(Line::from(Span::styled(
        hint,
        Style::default()
            .fg(palette::LIME)
            .bg(palette::BG)
            .add_modifier(Modifier::BOLD),
    )));

    text_lines.push(Line::default());

    // Quit hint
    text_lines.push(Line::from(Span::styled(
        "CTRL+C TO EXIT",
        Style::default()
            .fg(palette::DIM)
            .bg(palette::BG),
    )));

    let text_height = text_lines.len() as u16;
    let vert_pad = area.height.saturating_sub(text_height) / 2;

    let centred_area = Rect {
        x: area.x,
        y: area.y + vert_pad,
        width: area.width,
        height: area.height.saturating_sub(vert_pad),
    };

    let paragraph = Paragraph::new(Text::from(text_lines))
        .alignment(Alignment::Center)
        .style(Style::default().bg(palette::BG));

    frame.render_widget(paragraph, centred_area);
}

fn recovery_hint(error: &str) -> String {
    let lower = error.to_lowercase();
    if lower.contains("connection refused") || lower.contains("failed to connect") || lower.contains("connect") {
        "RECOVERY: check that your LLM server is running (see oni.toml [server])".to_string()
    } else if lower.contains("model") || lower.contains("not found") {
        "RECOVERY: check oni.toml [server.tiers] — verify GGUF paths and models_dir".to_string()
    } else if lower.contains("permission") || lower.contains("access denied") {
        "RECOVERY: check file permissions and re-run with appropriate access".to_string()
    } else if lower.contains("disk") || lower.contains("no space") {
        "RECOVERY: free disk space and restart".to_string()
    } else {
        "RECOVERY: check logs and restart ONI".to_string()
    }
}
