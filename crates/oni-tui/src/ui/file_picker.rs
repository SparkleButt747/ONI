use oni_core::palette;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

const MAX_VISIBLE: usize = 10;

/// Filter files by case-insensitive substring match.
pub fn filter_files<'a>(files: &'a [String], query: &str) -> Vec<&'a String> {
    let q = query.to_lowercase();
    files
        .iter()
        .filter(|f| f.to_lowercase().contains(&q))
        .take(MAX_VISIBLE)
        .collect()
}

/// Render the file picker popup above the input area.
pub fn draw_file_picker(
    files: &[String],
    query: &str,
    selected: usize,
    frame: &mut Frame,
    anchor: Rect,
) {
    let filtered = filter_files(files, query);
    if filtered.is_empty() {
        return;
    }

    let height = (filtered.len() as u16 + 2).min(MAX_VISIBLE as u16 + 2);
    let width = 50u16.min(anchor.width);
    let popup_area = Rect {
        x: anchor.x + 6,
        y: anchor.y.saturating_sub(height),
        width,
        height,
    };

    frame.render_widget(Clear, popup_area);

    let lines: Vec<Line> = filtered
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let style = if i == selected {
                Style::default()
                    .fg(palette::LIME)
                    .bg(palette::PANEL)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette::TEXT).bg(palette::BG)
            };
            Line::from(Span::styled(format!("  {} ", f), style))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette::BORDER))
        .title(Span::styled(
            " FILES ",
            Style::default()
                .fg(palette::MAGENTA)
                .add_modifier(Modifier::BOLD),
        ))
        .style(Style::default().bg(palette::BG));

    frame.render_widget(Paragraph::new(lines).block(block), popup_area);
}

/// Collect project files for the autocomplete, skipping hidden dirs and build artifacts.
pub fn collect_project_files(dir: &str) -> Vec<String> {
    let mut files = Vec::new();
    let walker = walkdir::WalkDir::new(dir)
        .max_depth(4)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') && name != "target" && name != "node_modules"
        });
    for entry in walker.flatten() {
        if entry.file_type().is_file() {
            if let Ok(rel) = entry.path().strip_prefix(dir) {
                files.push(rel.to_string_lossy().to_string());
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_picker_1_filters_by_substring() {
        let files = vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "tests/test.rs".to_string(),
            "Cargo.toml".to_string(),
        ];
        let filtered = filter_files(&files, "src/");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn t_picker_2_empty_query_shows_all() {
        let files = vec!["a.rs".to_string(), "b.rs".to_string()];
        let filtered = filter_files(&files, "");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn t_picker_3_case_insensitive() {
        let files = vec!["Cargo.toml".to_string()];
        let filtered = filter_files(&files, "cargo");
        assert_eq!(filtered.len(), 1);
    }
}
