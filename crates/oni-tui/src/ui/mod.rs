pub mod chat;
pub mod command_menu;
pub mod permission_prompt;
pub mod diff_view;
pub mod error_state;
pub mod input;
pub mod mission_control;
pub mod preferences;
pub mod response_label;
pub mod sidebar;
pub mod splash;
pub mod status;
pub mod thinking;

use crate::app::{App, ViewMode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::Widget;

pub fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    // Full-screen critical error takes over the entire frame
    if let Some(ref err) = app.critical_error.clone() {
        error_state::draw_error_state(frame, area, err, app.glitch_frame);
        return;
    }

    // Layout: status bar | main content | input | footer
    let rows = Layout::vertical([
        Constraint::Length(1),  // Status bar (Marathon industrial chrome)
        Constraint::Fill(1),    // Main content area
        Constraint::Length(3),  // Input area
        Constraint::Length(1),  // Footer (tier + ctx bar)
    ])
    .split(area);

    status::draw_status_bar(app, frame, rows[0]);
    draw_main_content(app, frame, rows[1]);
    input::draw_input(app, frame, rows[2]);
    status::draw_footer(app, frame, rows[3]);

    // Draw command menu popup on top of everything if visible
    if app.slash_menu_visible {
        command_menu::draw_command_menu(app, frame, rows[1]);
    }
}

fn draw_main_content(app: &mut App, frame: &mut Frame, area: ratatui::layout::Rect) {
    if app.view_mode == ViewMode::MissionControl {
        mission_control::draw_mission_control(app, frame, area);
        return;
    }
    if app.view_mode == ViewMode::Preferences {
        preferences::draw_preferences(app, frame, area);
        return;
    }
    if app.is_thinking {
        thinking::draw_thinking(app, frame, area);
    } else if !app.boot_complete && app.messages.is_empty() {
        splash::draw_splash(app, frame, area);
    } else if app.messages.is_empty() {
        splash::draw_splash(app, frame, area);
    } else {
        // Chat mode — split off a 2-char RESPONSE label on the right edge
        if area.width > 4 {
            let cols = Layout::horizontal([
                Constraint::Fill(1),   // Chat content
                Constraint::Length(2), // RESPONSE vertical label
            ])
            .split(area);

            chat::draw_chat(app, frame, cols[0]);
            response_label::ResponseLabel.render(cols[1], frame.buffer_mut());
        } else {
            chat::draw_chat(app, frame, area);
        }
    }
}
