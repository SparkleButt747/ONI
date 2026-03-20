use crate::app::App;
use oni_core::palette;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn draw_input(app: &mut App, frame: &mut Frame, area: Rect) {
    // Thin BORDER separator line at top of input area
    if area.height > 1 {
        let sep_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        };
        let sep = Paragraph::new(Line::from(Span::styled(
            "\u{2500}".repeat(area.width as usize),
            Style::default().fg(palette::BORDER).bg(palette::BG),
        )));
        frame.render_widget(sep, sep_area);
    }

    // Prompt on the row below the separator
    let prompt_y = if area.height > 1 { area.y + 1 } else { area.y };
    let prompt_width: u16 = 6; // " ONI > " = 6 chars

    let prompt_line = Line::from(vec![
        Span::styled(
            " ONI ",
            Style::default()
                .fg(palette::BG)
                .bg(palette::AMBER)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " ",
            Style::default().fg(palette::AMBER).bg(palette::BG),
        ),
    ]);

    frame.render_widget(
        Paragraph::new(prompt_line)
            .style(Style::default().bg(palette::BG)),
        Rect {
            x: area.x,
            y: prompt_y,
            width: prompt_width,
            height: 1,
        },
    );

    // Textarea to the right of the prompt
    let input_area = Rect {
        x: area.x + prompt_width,
        y: prompt_y,
        width: area.width.saturating_sub(prompt_width),
        height: area.height.saturating_sub(if area.height > 1 { 1 } else { 0 }),
    };

    frame.render_widget(&app.input, input_area);

    // Show prompt suggestion when input is empty and not thinking
    if app.input.lines()[0].is_empty() && !app.prompt_suggestion.is_empty() && !app.is_thinking {
        let suggestion_area = Rect {
            x: input_area.x,
            y: input_area.y,
            width: input_area.width.min(app.prompt_suggestion.len() as u16),
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                app.prompt_suggestion.clone(),
                Style::default().fg(palette::DIM).bg(palette::BG),
            ))),
            suggestion_area,
        );
    }
}
