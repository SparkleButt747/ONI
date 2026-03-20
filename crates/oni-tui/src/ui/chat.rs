use crate::app::{App, DisplayMessage};
use crate::ui::diff_view;
use oni_core::palette;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui::Frame;

/// Tiled dim background texture — fills empty space with repeating pattern text.
struct ChatTexture {
    pattern: &'static str,
}

impl Widget for ChatTexture {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let pat_chars: Vec<char> = self.pattern.chars().collect();
        let pat_len = pat_chars.len();
        if pat_len == 0 {
            return;
        }
        let style = Style::default()
            .fg(palette::DIM)
            .bg(palette::BG);

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                // Stagger each row by 5 chars to break up the grid look
                let idx = ((x - area.x) as usize + (y - area.y) as usize * 5) % pat_len;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(pat_chars[idx]);
                    cell.set_style(style);
                }
            }
        }
    }
}

/// Resolve a real character through the glitch effect. Characters on the left resolve
/// first as `progress` increases from 0.0 to 1.0.
fn glitch_resolve_char(real_char: char, col: u16, total_cols: u16, progress: f32) -> char {
    if total_cols == 0 || real_char == ' ' {
        return real_char;
    }
    let col_progress = col as f32 / total_cols.max(1) as f32;
    if col_progress < progress {
        real_char
    } else {
        const BLOCKS: [char; 8] = ['\u{2593}', '\u{2592}', '\u{2591}', '\u{2584}', '\u{2580}', '\u{258C}', '\u{2590}', '\u{2588}'];
        let seed = (col as usize * 7 + real_char as usize * 13) % BLOCKS.len();
        BLOCKS[seed]
    }
}

/// Overlay widget that applies the glitch-to-text resolve effect on a sub-region of the buffer.
/// Only affects rows between `start_row` and `end_row` (relative to the area).
struct GlitchResolve {
    progress: f32,
    start_row: u16,
    end_row: u16,
}

impl Widget for GlitchResolve {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.progress >= 1.0 {
            return;
        }
        let glitch_style = Style::default().fg(palette::CYAN).bg(palette::BG);
        let abs_start = area.y + self.start_row;
        let abs_end = (area.y + self.end_row).min(area.y + area.height);
        for y in abs_start..abs_end {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    let real_char = cell.symbol().chars().next().unwrap_or(' ');
                    let resolved = glitch_resolve_char(real_char, x - area.x, area.width, self.progress);
                    if resolved != real_char {
                        cell.set_char(resolved);
                        cell.set_style(glitch_style);
                    }
                }
            }
        }
    }
}

pub fn draw_chat(app: &mut App, frame: &mut Frame, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    // Track the line index where the last assistant message starts
    let mut last_assistant_line_start: Option<usize> = None;

    for msg in &app.messages {
        match msg {
            DisplayMessage::User(text) => {
                // Pad to full width so PANEL background spans the entire line
                let content = format!("> {}", text);
                let pad = if content.len() < area.width as usize {
                    " ".repeat(area.width as usize - content.len())
                } else {
                    String::new()
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        "> ",
                        Style::default()
                            .fg(palette::AMBER)
                            .bg(palette::PANEL)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}{}", text, pad),
                        Style::default().fg(palette::TEXT).bg(palette::PANEL),
                    ),
                ]));
                lines.push(Line::default());
            }
            DisplayMessage::Assistant(text) => {
                last_assistant_line_start = Some(lines.len());
                lines.push(Line::from(Span::styled(
                    " ONI ",
                    Style::default()
                        .fg(palette::BG)
                        .bg(palette::CYAN)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.extend(render_markdown(text));
                lines.push(Line::default());
            }
            DisplayMessage::ToolExec { name, status } => {
                let status_upper = status.to_uppercase();
                let (icon, status_color) = if status_upper.contains("DONE") {
                    ("\u{2713}", palette::LIME)
                } else if status_upper.contains("ERR") || status_upper.contains("FAIL") {
                    ("\u{2717}", palette::CORAL)
                } else {
                    ("\u{2022}", palette::MUTED)
                };
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", icon),
                        Style::default().fg(status_color).bg(palette::BG),
                    ),
                    Span::styled(
                        name.to_uppercase(),
                        Style::default().fg(palette::CYAN).bg(palette::BG),
                    ),
                    Span::styled(
                        format!("  {}", status_upper),
                        Style::default().fg(status_color).bg(palette::BG),
                    ),
                ]));
            }
            DisplayMessage::ToolDetail(detail) => {
                if app.verbose_tool_output {
                    // Expanded view — rich preview
                    match detail.name.as_str() {
                        "write_file" => {
                            let path = detail
                                .args
                                .get("path")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            let content = detail
                                .args
                                .get("content")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            lines.extend(diff_view::render_write_result(path, content));
                            lines.push(Line::default());
                        }
                        "bash" => {
                            let command = detail
                                .args
                                .get("command")
                                .and_then(|v| v.as_str())
                                .unwrap_or("?");
                            lines.extend(diff_view::render_bash_result(command, &detail.result));
                            lines.push(Line::default());
                        }
                        _ => {
                            lines.push(Line::from(Span::styled(
                                format!("   [{}] DONE", detail.name.to_uppercase()),
                                Style::default().fg(palette::MUTED).bg(palette::BG),
                            )));
                        }
                    }
                } else {
                    // Collapsed view — single line summary
                    lines.push(diff_view::render_collapsed_tool(&detail.name, &detail.args, &detail.result));
                }
            }
            DisplayMessage::Error(text) => {
                lines.push(Line::from(Span::styled(
                    format!(" ERROR: {}", text),
                    Style::default().fg(palette::CORAL).bg(palette::BG).add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::default());
            }
            DisplayMessage::System(text) => {
                lines.push(Line::from(Span::styled(
                    " SYS ",
                    Style::default()
                        .fg(palette::BG)
                        .bg(palette::MUTED)
                        .add_modifier(Modifier::BOLD),
                )));
                for line in text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("   {}", line),
                        Style::default()
                            .fg(palette::MUTED)
                            .bg(palette::BG),
                    )));
                }
                lines.push(Line::default());
            }
            // ── Orchestrator display (sub-agent prefixes per DESIGN_SYSTEM) ──
            DisplayMessage::Plan(steps) => {
                lines.push(Line::from(vec![
                    Span::styled(
                        " [\u{03A3}] ",
                        Style::default()
                            .fg(palette::VIOLET)
                            .bg(palette::BG)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " MIMIR ",
                        Style::default()
                            .fg(palette::BG)
                            .bg(palette::VIOLET)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        "  STRATEGY FORGED",
                        Style::default()
                            .fg(palette::VIOLET)
                            .bg(palette::BG),
                    ),
                ]));
                for (i, step) in steps.iter().enumerate() {
                    lines.push(Line::from(vec![
                        Span::styled(
                            format!("   {}. ", i + 1),
                            Style::default()
                                .fg(palette::VIOLET)
                                .bg(palette::BG),
                        ),
                        Span::styled(
                            step.clone(),
                            Style::default()
                                .fg(palette::TEXT)
                                .bg(palette::BG),
                        ),
                    ]));
                }
                lines.push(Line::default());
            }
            DisplayMessage::Step { current, total, description } => {
                lines.push(Line::from(vec![
                    Span::styled(
                        " [\u{03A8}] ",
                        Style::default()
                            .fg(palette::CYAN)
                            .bg(palette::BG)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " FENRIR ",
                        Style::default()
                            .fg(palette::BG)
                            .bg(palette::CYAN)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  STEP {}/{}: {}", current, total, description),
                        Style::default()
                            .fg(palette::CYAN)
                            .bg(palette::BG),
                    ),
                ]));
            }
            DisplayMessage::CriticVerdict { accepted, reason } => {
                if *accepted {
                    lines.push(Line::from(vec![
                        Span::styled(
                            " [\u{2298}] ",
                            Style::default()
                                .fg(palette::LIME)
                                .bg(palette::BG)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            " SKULD ",
                            Style::default()
                                .fg(palette::BG)
                                .bg(palette::LIME)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "  ACCEPTED",
                            Style::default()
                                .fg(palette::LIME)
                                .bg(palette::BG)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));
                } else {
                    lines.push(Line::from(vec![
                        Span::styled(
                            " [\u{2298}] ",
                            Style::default()
                                .fg(palette::CORAL)
                                .bg(palette::BG)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            " SKULD ",
                            Style::default()
                                .fg(palette::BG)
                                .bg(palette::CORAL)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  REJECTED: {}", reason),
                            Style::default()
                                .fg(palette::CORAL)
                                .bg(palette::BG),
                        ),
                    ]));
                }
            }
            DisplayMessage::Replanning { cycle, reason } => {
                lines.push(Line::from(vec![
                    Span::styled(
                        " [\u{03A3}] ",
                        Style::default()
                            .fg(palette::AMBER)
                            .bg(palette::BG)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " MIMIR:REPLAN ",
                        Style::default()
                            .fg(palette::BG)
                            .bg(palette::AMBER)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("  CYCLE {} — {}", cycle, reason),
                        Style::default()
                            .fg(palette::AMBER)
                            .bg(palette::BG),
                    ),
                ]));
                lines.push(Line::default());
            }
        }
    }

    // Scroll handling: respect user's manual scroll offset
    let total_lines = lines.len() as u16;
    let visible_lines = area.height;
    let max_scroll = total_lines.saturating_sub(visible_lines);
    let scroll = if app.scroll_locked_to_bottom {
        max_scroll
    } else {
        max_scroll.saturating_sub(app.scroll_offset)
    };

    // If there's empty space below messages, draw the texture first
    if total_lines < visible_lines {
        let empty_lines = visible_lines - total_lines;
        let texture_area = Rect {
            x: area.x,
            y: area.y + total_lines,
            width: area.width,
            height: empty_lines,
        };
        frame.render_widget(ChatTexture { pattern: "PASSPHRASE_FAIL_" }, texture_area);
    }

    // Calculate the visible row range of the last assistant message (post-scroll)
    let last_asst_start = last_assistant_line_start.unwrap_or(0) as u16;
    let last_asst_end = total_lines; // extends to the end of all lines

    let paragraph = Paragraph::new(Text::from(lines))
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0))
        .style(Style::default().bg(palette::BG));

    frame.render_widget(paragraph, area);

    // Glitch-to-text resolve effect — only on the last assistant message
    if app.reveal_progress < 1.0 {
        if let Some(_) = last_assistant_line_start {
            // Convert line indices to visible row positions (accounting for scroll)
            let vis_start = last_asst_start.saturating_sub(scroll);
            let vis_end = last_asst_end.saturating_sub(scroll).min(area.height);
            if vis_start < area.height && vis_end > vis_start {
                frame.render_widget(
                    GlitchResolve {
                        progress: app.reveal_progress,
                        start_row: vis_start,
                        end_row: vis_end,
                    },
                    area,
                );
            }
        }
    }
}

/// Lightweight markdown-to-ratatui renderer. No external deps — handles the subset
/// of markdown that ONI responses actually use.
fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;

    for raw_line in text.lines() {
        // --- Fenced code block toggle ---
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                // Opening fence — extract language tag
                let lang = raw_line.trim_start().strip_prefix("```").unwrap_or("").trim();
                if !lang.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("   {}", lang.to_uppercase()),
                        Style::default()
                            .fg(palette::DIM)
                            .bg(palette::PANEL),
                    )));
                }
            }
            continue;
        }

        if in_code_block {
            lines.push(Line::from(Span::styled(
                format!("   {}", raw_line),
                Style::default()
                    .fg(palette::TEXT)
                    .bg(palette::PANEL),
            )));
            continue;
        }

        // --- Blank line ---
        if raw_line.trim().is_empty() {
            lines.push(Line::default());
            continue;
        }

        // --- Headers (inverse section style) ---
        if let Some(rest) = raw_line.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                format!(" {} ", rest.to_uppercase()),
                Style::default()
                    .fg(palette::BG)
                    .bg(palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(rest) = raw_line.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                format!(" {} ", rest.to_uppercase()),
                Style::default()
                    .fg(palette::BG)
                    .bg(palette::CYAN)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }
        if let Some(rest) = raw_line.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                format!(" {} ", rest.to_uppercase()),
                Style::default()
                    .fg(palette::BG)
                    .bg(palette::AMBER)
                    .add_modifier(Modifier::BOLD),
            )));
            continue;
        }

        // --- Bullet lists (`- ` or `* `) ---
        if let Some(rest) = raw_line.strip_prefix("- ").or_else(|| raw_line.strip_prefix("* ")) {
            let content = render_inline(rest);
            let mut spans = vec![Span::styled(
                "   • ",
                Style::default().fg(palette::TEXT).bg(palette::BG),
            )];
            spans.extend(content);
            lines.push(Line::from(spans));
            continue;
        }

        // --- Numbered lists (`1. `, `2. `, etc.) ---
        if let Some(idx) = raw_line.find(". ") {
            let prefix = &raw_line[..idx];
            if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_digit()) {
                let number = prefix;
                let rest = &raw_line[idx + 2..];
                let content = render_inline(rest);
                let mut spans = vec![Span::styled(
                    format!("   {}. ", number),
                    Style::default().fg(palette::TEXT).bg(palette::BG),
                )];
                spans.extend(content);
                lines.push(Line::from(spans));
                continue;
            }
        }

        // --- Indented continuation (4 spaces) ---
        if raw_line.starts_with("    ") {
            lines.push(Line::from(Span::styled(
                format!("      {}", raw_line.trim_start()),
                Style::default()
                    .fg(palette::TEXT)
                    .bg(palette::PANEL),
            )));
            continue;
        }

        // --- Normal text (with inline bold/code handling) ---
        let content = render_inline(raw_line);
        let mut spans = vec![Span::styled(
            "   ",
            Style::default().fg(palette::TEXT).bg(palette::BG),
        )];
        spans.extend(content);
        lines.push(Line::from(spans));
    }

    lines
}

/// Parse a single line for inline `**bold**` and `\`code\`` markers.
/// Returns a vec of styled Spans. All output is `'static` (owned strings).
fn render_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        // Inline code: `...`
        if let Some(start) = remaining.find('`') {
            if start > 0 {
                spans.push(Span::styled(
                    remaining[..start].to_owned(),
                    Style::default().fg(palette::TEXT).bg(palette::BG),
                ));
            }
            let after = &remaining[start + 1..];
            if let Some(end) = after.find('`') {
                spans.push(Span::styled(
                    after[..end].to_owned(),
                    Style::default()
                        .fg(palette::CYAN)
                        .bg(palette::PANEL),
                ));
                remaining = &after[end + 1..];
                continue;
            }
            // No closing backtick — treat rest as plain
            spans.push(Span::styled(
                remaining.to_owned(),
                Style::default().fg(palette::TEXT).bg(palette::BG),
            ));
            break;
        }

        // Bold: **...**
        if let Some(start) = remaining.find("**") {
            if start > 0 {
                spans.push(Span::styled(
                    remaining[..start].to_owned(),
                    Style::default().fg(palette::TEXT).bg(palette::BG),
                ));
            }
            let after = &remaining[start + 2..];
            if let Some(end) = after.find("**") {
                spans.push(Span::styled(
                    after[..end].to_owned(),
                    Style::default()
                        .fg(palette::WHITE)
                        .bg(palette::BG)
                        .add_modifier(Modifier::BOLD),
                ));
                remaining = &after[end + 2..];
                continue;
            }
            // No closing ** — treat rest as plain
            spans.push(Span::styled(
                remaining.to_owned(),
                Style::default().fg(palette::TEXT).bg(palette::BG),
            ));
            break;
        }

        // Plain text — no more markers
        spans.push(Span::styled(
            remaining.to_owned(),
            Style::default().fg(palette::TEXT).bg(palette::BG),
        ));
        break;
    }

    spans
}
