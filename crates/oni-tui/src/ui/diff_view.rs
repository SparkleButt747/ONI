use oni_core::palette;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use serde_json;

/// Dim lime-tinted background for added lines.
const BG_ADD: Color = Color::Rgb(12, 20, 4);
/// Dim coral-tinted background for removed lines.
const BG_DEL: Color = Color::Rgb(24, 6, 4);

/// Parse a unified diff string and return styled ratatui Lines.
/// Colours:
///   - `+` lines: DATA green fg, dim green bg
///   - `-` lines: ALERT red fg, dim red bg
///   - `@@` hunk headers: STATE amber fg
///   - context lines: DIM
///   - `---`/`+++` file headers: STATE bold
pub fn render_diff(diff_text: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    for raw in diff_text.lines() {
        let line = raw.to_owned();

        if line.starts_with("+++") || line.starts_with("---") {
            // File header
            lines.push(Line::from(Span::styled(
                format!("   {}", line),
                Style::default()
                    .fg(palette::AMBER)
                    .bg(palette::BG)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if line.starts_with("@@") {
            // Hunk header
            lines.push(Line::from(Span::styled(
                format!("   {}", line),
                Style::default()
                    .fg(palette::MUTED)
                    .bg(palette::BG),
            )));
        } else if line.starts_with('+') {
            // Added line
            lines.push(Line::from(vec![
                Span::styled(
                    " + ",
                    Style::default()
                        .fg(palette::DATA)
                        .bg(BG_ADD)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    line[1..].to_owned(),
                    Style::default().fg(palette::LIME).bg(BG_ADD),
                ),
            ]));
        } else if line.starts_with('-') {
            // Removed line
            lines.push(Line::from(vec![
                Span::styled(
                    " - ",
                    Style::default()
                        .fg(palette::ALERT)
                        .bg(BG_DEL)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    line[1..].to_owned(),
                    Style::default().fg(palette::ALERT).bg(BG_DEL),
                ),
            ]));
        } else {
            // Context line
            lines.push(Line::from(Span::styled(
                format!("   {}", line),
                Style::default()
                    .fg(palette::MUTED)
                    .bg(palette::BG),
            )));
        }
    }

    lines
}

/// Render a write_file tool result inline in chat.
/// Shows a mini header with filename + byte count, then a code-block style
/// view of the written content (first N lines, truncated if long).
pub fn render_write_result(path: &str, content: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    let line_count = content.lines().count();
    let byte_count = content.len();

    // Header: filename + stats in STATE amber
    lines.push(Line::from(vec![
        Span::styled(
            " WRITE ".to_string(),
            Style::default()
                .fg(palette::BG)
                .bg(palette::AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", path),
            Style::default()
                .fg(palette::AMBER)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("[{} lines / {} bytes]", line_count, byte_count),
            Style::default().fg(palette::DIM).bg(palette::BG),
        ),
    ]));

    // Show up to 20 lines of content with dim green tint (added style)
    const MAX_PREVIEW_LINES: usize = 20;
    let preview_lines: Vec<&str> = content.lines().take(MAX_PREVIEW_LINES).collect();
    for l in &preview_lines {
        lines.push(Line::from(vec![
            Span::styled(
                " + ",
                Style::default()
                    .fg(palette::LIME)
                    .bg(BG_ADD),
            ),
            Span::styled(
                (*l).to_owned(),
                Style::default().fg(palette::LIME).bg(BG_ADD),
            ),
        ]));
    }

    if line_count > MAX_PREVIEW_LINES {
        lines.push(Line::from(Span::styled(
            format!(
                "   ... +{} more lines",
                line_count - MAX_PREVIEW_LINES
            ),
            Style::default().fg(palette::DIM).bg(palette::BG),
        )));
    }

    lines
}

/// Render a collapsed single-line tool summary.
pub fn render_collapsed_tool(name: &str, args: &serde_json::Value, result: &str) -> Line<'static> {
    let summary = match name {
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let line_count = content.lines().count();
            format!("Wrote {} [{} lines]", path, line_count)
        }
        "bash" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("?");
            let display_cmd = if cmd.len() > 60 {
                format!(
                    "{}...",
                    &cmd[..cmd
                        .char_indices()
                        .take(57)
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(57)]
                )
            } else {
                cmd.to_string()
            };
            let exit_ok = !result.contains("[exit code:") || result.contains("[exit code: 0]");
            if exit_ok {
                format!("$ {}", display_cmd)
            } else {
                format!("$ {} [FAILED]", display_cmd)
            }
        }
        "edit_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Edited {}", path)
        }
        "read_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Read {}", path)
        }
        "search_files" => {
            let pattern = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
            format!("Searched for \"{}\"", pattern)
        }
        _ => format!("{} done", name),
    };

    Line::from(vec![
        Span::styled(
            "  \u{2713} ",
            Style::default().fg(palette::LIME).bg(palette::BG),
        ),
        Span::styled(
            format!("{:<16}", name.to_uppercase()),
            Style::default().fg(palette::CYAN).bg(palette::BG),
        ),
        Span::styled(
            summary,
            Style::default().fg(palette::MUTED).bg(palette::BG),
        ),
    ])
}

/// Render a bash tool result inline in chat.
/// Shows the command in STATE colour and output in a code-block style.
pub fn render_bash_result(command: &str, output: &str) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // Command header
    lines.push(Line::from(vec![
        Span::styled(
            " EXEC ".to_string(),
            Style::default()
                .fg(palette::BG)
                .bg(palette::AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" $ {}", command),
            Style::default()
                .fg(palette::AMBER)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    if output.trim().is_empty() {
        lines.push(Line::from(Span::styled(
            "   [no output]",
            Style::default().fg(palette::DIM).bg(palette::BG),
        )));
        return lines;
    }

    const MAX_OUTPUT_LINES: usize = 30;
    let output_lines: Vec<&str> = output.lines().collect();
    let shown = output_lines.len().min(MAX_OUTPUT_LINES);

    // Detect stderr / error lines to tint differently
    for l in &output_lines[..shown] {
        let is_err = l.starts_with("[stderr]") || l.starts_with("[exit code:");
        let style = if is_err {
            Style::default().fg(palette::CORAL).bg(palette::BG)
        } else {
            Style::default()
                .fg(palette::MUTED)
                .bg(palette::BG)
        };
        lines.push(Line::from(Span::styled(
            format!("   {}", l),
            style,
        )));
    }

    if output_lines.len() > MAX_OUTPUT_LINES {
        lines.push(Line::from(Span::styled(
            format!(
                "   ... +{} more lines",
                output_lines.len() - MAX_OUTPUT_LINES
            ),
            Style::default().fg(palette::DIM).bg(palette::BG),
        )));
    }

    lines
}
