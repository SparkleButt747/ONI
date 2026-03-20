use crate::app::App;
use crate::widgets::border_pulse::active_border_color;
use crate::widgets::HazardDivider;
use oni_core::palette;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget};

pub fn draw_mission_control(app: &App, frame: &mut Frame, area: Rect) {
    // Fill background
    let bg = Paragraph::new("").style(Style::default().bg(palette::BG));
    frame.render_widget(bg, area);

    // Main layout: top panel (agents + stats) | hazard | tool log | status bar
    let rows = Layout::vertical([
        Constraint::Length(14), // Sub-agents + stats (hero panel)
        Constraint::Length(1),  // HazardDivider
        Constraint::Fill(1),   // Tool call log (fills remaining space)
        Constraint::Length(2), // Session status bar
    ])
    .split(area);

    draw_hero_panel(app, frame, rows[0]);
    HazardDivider.render(rows[1], frame.buffer_mut());
    draw_tool_log(app, frame, rows[2]);
    draw_status_bar(app, frame, rows[3]);
}

/// Top panel — two columns: sub-agents (left, hero) | stats + gauges (right)
fn draw_hero_panel(app: &App, frame: &mut Frame, area: Rect) {
    let cols = Layout::horizontal([
        Constraint::Percentage(60), // Sub-agents — the hero
        Constraint::Percentage(40), // Stats sidebar
    ])
    .split(area);

    draw_sub_agents(app, frame, cols[0]);
    draw_stats_sidebar(app, frame, cols[1]);
}

/// Sub-agent status panel — the core of the system. Each agent gets 4 rows.
fn draw_sub_agents(app: &App, frame: &mut Frame, area: Rect) {
    if area.height < 4 {
        return;
    }

    let tick = app.boot_frame as u64;
    let mut lines: Vec<Line> = Vec::new();

    // Section label
    lines.push(Line::from(Span::styled(
        " SUB_AGENTS ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::CYAN)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::default()); // breathing room

    // --- MIMIR ---
    let mimir_status = app.sub_agent_status.mimir;
    let mimir_bc = agent_border_color(mimir_status, tick);
    let mimir_sc = status_style(mimir_status);

    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(palette::BG)),
        Span::styled("│", Style::default().fg(mimir_bc).bg(palette::BG)),
        Span::styled(
            " [\u{03A3}] MIMIR ",
            Style::default()
                .fg(palette::VIOLET)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("─── {}", mimir_status),
            mimir_sc,
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(palette::BG)),
        Span::styled("│", Style::default().fg(palette::BORDER).bg(palette::BG)),
        Span::styled(
            format!("   Planner · {} · Heavy", app.model_config.heavy),
            Style::default().fg(palette::MUTED).bg(palette::BG),
        ),
    ]));
    lines.push(Line::default());

    // --- FENRIR ---
    let fenrir_status = app.sub_agent_status.fenrir;
    let fenrir_bc = agent_border_color(fenrir_status, tick);
    let fenrir_sc = status_style(fenrir_status);

    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(palette::BG)),
        Span::styled("│", Style::default().fg(fenrir_bc).bg(palette::BG)),
        Span::styled(
            " [\u{03A8}] FENRIR ",
            Style::default()
                .fg(palette::CYAN)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("─── {}", fenrir_status),
            fenrir_sc,
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(palette::BG)),
        Span::styled("│", Style::default().fg(palette::BORDER).bg(palette::BG)),
        Span::styled(
            format!("   Executor · {} · Medium", app.model_config.medium),
            Style::default().fg(palette::MUTED).bg(palette::BG),
        ),
    ]));
    lines.push(Line::default());

    // --- SKULD ---
    let skuld_status = app.sub_agent_status.skuld;
    let skuld_bc = agent_border_color(skuld_status, tick);
    let skuld_sc = status_style(skuld_status);

    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(palette::BG)),
        Span::styled("│", Style::default().fg(skuld_bc).bg(palette::BG)),
        Span::styled(
            " [\u{2298}] SKULD ",
            Style::default()
                .fg(palette::CORAL)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("─── {}", skuld_status),
            skuld_sc,
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(palette::BG)),
        Span::styled("│", Style::default().fg(palette::BORDER).bg(palette::BG)),
        Span::styled(
            format!("   Critic · {} · General", app.model_config.general),
            Style::default().fg(palette::MUTED).bg(palette::BG),
        ),
    ]));

    let para = Paragraph::new(Text::from(lines)).style(Style::default().bg(palette::BG));
    frame.render_widget(para, area);
}

/// Right sidebar: compact inline stats + context gauge.
fn draw_stats_sidebar(app: &App, frame: &mut Frame, area: Rect) {
    if area.height < 4 || area.width < 10 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Section label
    lines.push(Line::from(Span::styled(
        " DIAGNOSTICS ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::AMBER)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::default());

    // Compact stat rows — label left, value right-aligned
    let tok_s = if app.last_tokens_per_sec > 0.0 {
        format!("{:.1}", app.last_tokens_per_sec)
    } else {
        "—".into()
    };

    let burn = if app.burn_rate > 0.0 {
        format!("{:.0} t/min", app.burn_rate)
    } else {
        "—".into()
    };

    let stats: &[(&str, String, ratatui::style::Color)] = &[
        ("TURNS", app.turn_count.to_string(), palette::AMBER),
        ("TOKENS", format_number(app.total_tokens), palette::CYAN),
        ("TOK/S", tok_s, palette::LIME),
        ("CALLS", app.tool_history.len().to_string(), palette::CYAN),
        ("BURN", burn, palette::WARNING),
    ];

    let value_width = area.width.saturating_sub(12) as usize; // label takes ~10 chars

    for (label, value, color) in stats {
        let padding = value_width.saturating_sub(value.len());
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<8}", label),
                Style::default().fg(palette::MUTED).bg(palette::BG),
            ),
            Span::styled(
                format!("{}{}", " ".repeat(padding), value),
                Style::default().fg(*color).bg(palette::BG),
            ),
        ]));
    }

    lines.push(Line::default());

    // Context window gauge
    let ctx_budget = match app.current_tier {
        oni_core::types::ModelTier::Heavy | oni_core::types::ModelTier::Medium => 32768u64,
        oni_core::types::ModelTier::General => 16384u64,
        oni_core::types::ModelTier::Fast => 8192u64,
        oni_core::types::ModelTier::Embed => 2048u64,
    };
    let ctx_used = app.total_tokens.min(ctx_budget);
    let ctx_pct = (ctx_used as f32 / ctx_budget as f32 * 100.0) as u16;
    let gauge_width = (area.width as usize).saturating_sub(10).min(20);
    let filled = (ctx_pct as usize * gauge_width / 100).min(gauge_width);
    let gauge_color = if ctx_pct >= 80 {
        palette::CORAL
    } else if ctx_pct >= 60 {
        palette::WARNING
    } else {
        palette::AMBER
    };

    lines.push(Line::from(Span::styled(
        "  CTX_WINDOW",
        Style::default()
            .fg(palette::MUTED)
            .bg(palette::BG),
    )));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default().bg(palette::BG)),
        Span::styled(
            format!(
                "[{}{}] {}%",
                "\u{2588}".repeat(filled),
                "\u{2591}".repeat(gauge_width - filled),
                ctx_pct
            ),
            Style::default().fg(gauge_color).bg(palette::BG),
        ),
    ]));

    // Session elapsed
    let elapsed = app.session_start.elapsed().as_secs();
    let elapsed_str = if elapsed >= 3600 {
        format!("{}h {}m", elapsed / 3600, (elapsed % 3600) / 60)
    } else if elapsed >= 60 {
        format!("{}m {}s", elapsed / 60, elapsed % 60)
    } else {
        format!("{}s", elapsed)
    };

    lines.push(Line::default());
    lines.push(Line::from(vec![
        Span::styled(
            format!("  SESSION  {}", elapsed_str),
            Style::default().fg(palette::DIM).bg(palette::BG),
        ),
    ]));

    let para = Paragraph::new(Text::from(lines)).style(Style::default().bg(palette::BG));
    frame.render_widget(para, area);
}

/// Tool call history — most recent first. Compact diagnostic log format.
fn draw_tool_log(app: &App, frame: &mut Frame, area: Rect) {
    if area.height == 0 {
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Section header
    lines.push(Line::from(Span::styled(
        " TOOL_CALL_LOG ",
        Style::default()
            .fg(palette::BG)
            .bg(palette::CYAN)
            .add_modifier(Modifier::BOLD),
    )));

    if app.tool_history.is_empty() {
        lines.push(Line::from(Span::styled(
            "  NO_CALLS_YET",
            Style::default().fg(palette::DIM).bg(palette::BG),
        )));
    } else {
        let visible = (area.height as usize).saturating_sub(1);
        for record in app.tool_history.iter().rev().take(visible) {
            let latency = if record.latency_ms > 0 {
                if record.latency_ms >= 1000 {
                    format!("{:.1}s", record.latency_ms as f64 / 1000.0)
                } else {
                    format!("{}ms", record.latency_ms)
                }
            } else {
                String::new()
            };

            let status_style = if record.status.contains("OK") || record.status.contains("DONE") {
                Style::default().fg(palette::LIME).bg(palette::BG)
            } else if record.status.contains("ERR") || record.status.contains("FAIL") {
                Style::default()
                    .fg(palette::CORAL)
                    .bg(palette::BG)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette::MUTED).bg(palette::BG)
            };

            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", record.timestamp),
                    Style::default().fg(palette::DIM).bg(palette::BG),
                ),
                Span::styled(
                    format!("{:<14}", record.name),
                    Style::default()
                        .fg(palette::AMBER)
                        .bg(palette::BG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{:<20}", record.args_summary),
                    Style::default().fg(palette::MUTED).bg(palette::BG),
                ),
                Span::styled(format!("{:<6}", record.status), status_style),
                Span::styled(
                    format!(" {}", latency),
                    Style::default().fg(palette::DIM).bg(palette::BG),
                ),
            ]));
        }
    }

    let para = Paragraph::new(Text::from(lines)).style(Style::default().bg(palette::BG));
    frame.render_widget(para, area);
}

/// Bottom status bar — model, tier, keybinds. Single row, compact.
fn draw_status_bar(app: &App, frame: &mut Frame, area: Rect) {
    if area.height == 0 {
        return;
    }

    // Thin separator line
    let sep_area = Rect { height: 1, ..area };
    let sep = Paragraph::new(Line::from(Span::styled(
        "─".repeat(area.width as usize),
        Style::default().fg(palette::BORDER).bg(palette::BG),
    )));
    frame.render_widget(sep, sep_area);

    if area.height < 2 {
        return;
    }

    let info_area = Rect {
        y: area.y + 1,
        height: 1,
        ..area
    };

    let info_line = Line::from(vec![
        Span::styled(
            format!("  {} ", app.current_model_name.to_uppercase()),
            Style::default()
                .fg(palette::AMBER)
                .bg(palette::BG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("· {} ", app.current_tier.display_name()),
            Style::default().fg(palette::MUTED).bg(palette::BG),
        ),
        Span::styled(
            "│ ",
            Style::default().fg(palette::BORDER).bg(palette::BG),
        ),
        Span::styled(
            "ESC return  TAB next_panel  /help commands",
            Style::default().fg(palette::DIM).bg(palette::BG),
        ),
    ]);

    let para = Paragraph::new(info_line);
    frame.render_widget(para, info_area);
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn status_style(status: &str) -> Style {
    match status {
        "ACTIVE" => Style::default()
            .fg(palette::AMBER)
            .bg(palette::BG)
            .add_modifier(Modifier::BOLD),
        "DONE" => Style::default().fg(palette::LIME).bg(palette::BG),
        _ => Style::default().fg(palette::MUTED).bg(palette::BG),
    }
}

fn agent_border_color(status: &str, tick: u64) -> ratatui::style::Color {
    if status == "ACTIVE" {
        active_border_color(tick, 30)
    } else {
        palette::BORDER
    }
}

/// Format large numbers with comma separators for readability.
fn format_number(n: u64) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}
