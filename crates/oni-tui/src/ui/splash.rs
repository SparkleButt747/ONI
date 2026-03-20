use crate::app::App;
use crate::widgets::Spectrum;
use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget};
use ratatui::Frame;

/// Tiled background texture — repeating "ONI_ONBOARD_NATIVE_INTELLIGENCE_" in DIM.
struct TiledText;

impl Widget for TiledText {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let pattern = "ONI  ONBOARD  NATIVE  INTELLIGENCE   ";
        let style = Style::default().fg(palette::BORDER).bg(palette::BG);
        let chars: Vec<char> = pattern.chars().collect();
        let len = chars.len();
        // Only fill every other row for breathing room
        for y in area.y..area.y + area.height {
            if (y - area.y) % 2 != 0 {
                continue; // skip odd rows — creates vertical spacing
            }
            for x in area.x..area.x + area.width {
                let idx = ((y as usize * area.width as usize) + x as usize) % len;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(chars[idx]);
                    cell.set_style(style);
                }
            }
        }
    }
}

const ONI_LOGO: &str = r#"
  ██████  ███    ██ ██
 ██    ██ ████   ██ ██
 ██    ██ ██ ██  ██ ██
 ██    ██ ██  ██ ██ ██
  ██████  ██   ████ ██
"#;

const TAGLINE: &str = "ONBOARD_NATIVE_INTELLIGENCE";
const VERSION: &str = "V0.1.0";

// Frame thresholds for each boot stage (logo always shown from frame 0)
const FRAME_SUBTITLE: u16 = 6;
const FRAME_HAZARD_1: u16 = 9;
const FRAME_INIT_1: u16 = 11;
const FRAME_INIT_2: u16 = 12;
const FRAME_INIT_3: u16 = 13;
const FRAME_INIT_4: u16 = 14;
const FRAME_HAZARD_2: u16 = 16;
const FRAME_KEYBINDS: u16 = 18;
const FRAME_CURSOR: u16 = 21;

/// Hazard wipe bar — repeating `█░` pattern across the full width.
fn hazard_line(width: u16) -> Line<'static> {
    let pattern = "█░";
    let pat_chars: Vec<char> = pattern.chars().collect();
    let pat_len = pat_chars.len();
    let content: String = (0..width as usize)
        .map(|i| pat_chars[i % pat_len])
        .collect();
    Line::from(Span::styled(
        content,
        Style::default()
            .fg(palette::AMBER)
            .bg(palette::BG),
    ))
}

pub fn draw_splash(app: &App, frame: &mut Frame, area: Rect) {
    let frame_n = app.boot_frame;

    // Split: main content, spectrum footer (3 rows)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    let content_area = chunks[0];
    let footer_area = chunks[1];

    // --- Background: tiled text pattern at DIM ---
    frame.render_widget(TiledText, content_area);

    // --- Build boot sequence lines progressively ---
    let logo_lines: Vec<&str> = ONI_LOGO.lines().filter(|l| !l.is_empty()).collect();
    let total_static_height = logo_lines.len()       // logo
        + 1                                          // blank
        + 1                                          // subtitle
        + 1                                          // blank
        + 2                                          // hazard 1 + blank
        + 4                                          // 4 init lines
        + 1                                          // blank
        + 2                                          // hazard 2 + blank
        + 1                                          // keybinds
        + 1;                                         // cursor hint

    let top_pad = content_area
        .height
        .saturating_sub(total_static_height as u16)
        / 2;

    let mut lines: Vec<Line> = Vec::new();

    for _ in 0..top_pad {
        lines.push(Line::default());
    }

    // Frame 0+: ONI logo (bold amber) — always shown (frame_n starts at 0)
    for line in &logo_lines {
        lines.push(Line::from(Span::styled(
            line.to_string(),
            Style::default()
                .fg(palette::AMBER)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        )));
    }
    lines.push(Line::default());

    // Frame 6-8: Subtitle
    if frame_n >= FRAME_SUBTITLE {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} · ", TAGLINE),
                Style::default()
                    .fg(palette::MUTED)
                    .bg(palette::BG),
            ),
            Span::styled(
                VERSION.to_string(),
                Style::default()
                    .fg(palette::DIM)
                    .bg(palette::BG),
            ),
        ]));
        lines.push(Line::default());
    }

    // Frame 9-10: Hazard wipe 1
    if frame_n >= FRAME_HAZARD_1 {
        lines.push(hazard_line(content_area.width.saturating_sub(2)));
        lines.push(Line::default());
    }

    // Frame 11-14: Init check lines appearing one by one
    let model_upper = app.current_model_name.to_uppercase();
    let file_count = app.boot_file_count;

    let server_display = if app.server_url.is_empty() {
        "CONNECTED".to_string()
    } else {
        format!("CONNECTED — {}", app.server_url)
    };
    let tools_display = app.tool_names.iter()
        .map(|t| t.to_uppercase())
        .collect::<Vec<_>>()
        .join(" · ");

    let init_lines: &[(&str, String)] = &[
        ("INIT", format!("✓ LLM SERVER — {}", server_display)),
        ("INIT", format!("✓ MODEL LOADED — {}", model_upper)),
        ("INIT", format!("✓ PROJECT INDEX — {} FILES", file_count)),
        ("INIT", format!("✓ TOOLS — {}", tools_display)),
    ];

    let init_frames = [FRAME_INIT_1, FRAME_INIT_2, FRAME_INIT_3, FRAME_INIT_4];

    for (i, (label, value)) in init_lines.iter().enumerate() {
        if frame_n >= init_frames[i] {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", label),
                    Style::default()
                        .fg(palette::MUTED)
                        .bg(palette::BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {}", value),
                    Style::default().fg(palette::LIME).bg(palette::BG),
                ),
            ]));
        }
    }

    // Frame 16-17: Hazard wipe 2
    if frame_n >= FRAME_HAZARD_2 {
        lines.push(Line::default());
        lines.push(hazard_line(content_area.width.saturating_sub(2)));
        lines.push(Line::default());
    }

    // Frame 18-20: Keybind hints
    if frame_n >= FRAME_KEYBINDS {
        lines.push(Line::from(Span::styled(
            "  /HELP COMMANDS · /TIER MODEL · /QUIT EXIT",
            Style::default()
                .fg(palette::DIM)
                .bg(palette::BG),
        )));
    }

    // Frame 21+: Cursor ready hint
    if frame_n >= FRAME_CURSOR {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "  TYPE A MESSAGE TO BEGIN  \u{258c}",
            Style::default()
                .fg(palette::TEXT)
                .bg(palette::BG),
        )));
    }

    let paragraph = Paragraph::new(Text::from(lines))
        .alignment(Alignment::Left)
        .style(Style::default().bg(palette::BG));

    frame.render_widget(paragraph, content_area);

    // --- Spectrum footer decoration ---
    let width = footer_area.width as usize;
    let values: Vec<u16> = (0..width)
        .map(|i| {
            let t = i as f32 / width.max(1) as f32;
            let v =
                (((t * std::f32::consts::TAU * 2.0).sin() + 1.0) / 2.0 * 80.0) as u16;
            v
        })
        .collect();

    let spectrum = Spectrum::new(values);
    frame.render_widget(spectrum, footer_area);
}
