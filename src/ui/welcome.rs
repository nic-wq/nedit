use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use unicode_width::UnicodeWidthStr;

use crate::app::App;

use super::UIColors;

const LOGO_LINES: [&str; 6] = [
    "███╗   ██╗███████╗██████╗ ██╗████████╗",
    "████╗  ██║██╔════╝██╔══██╗██║╚══██╔══╝",
    "██╔██╗ ██║█████╗  ██║  ██║██║   ██║",
    "██║╚██╗██║██╔══╝  ██║  ██║██║   ██║",
    "██║ ╚████║███████╗██████╔╝██║   ██║",
    "╚═╝  ╚═══╝╚══════╝╚═════╝ ╚═╝   ╚═╝",
];

pub(super) fn draw_welcome_screen(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Length(10)])
        .split(area);

    draw_logo(f, chunks[0], colors);
    draw_shortcuts(f, app, chunks[1], colors);
}

fn draw_logo(f: &mut Frame, area: Rect, colors: &UIColors) {
    let logo_width = LOGO_LINES
        .iter()
        .map(|line| UnicodeWidthStr::width(*line))
        .max()
        .unwrap_or(0) as u16;

    let logo_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(logo_width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(area);

    let logo = LOGO_LINES
        .into_iter()
        .map(Line::from)
        .collect::<Vec<Line<'_>>>();

    f.render_widget(
        Paragraph::new(logo)
            .style(Style::default().fg(colors.accent))
            .alignment(Alignment::Left),
        logo_chunks[1],
    );

    let version_area = Rect {
        y: area.y + 7,
        height: 1,
        ..area
    };

    f.render_widget(
        Paragraph::new(format!("v{}", env!("CARGO_PKG_VERSION")))
            .style(Style::default().fg(colors.accent))
            .alignment(Alignment::Center),
        version_area,
    );
}

fn draw_shortcuts(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    let shortcuts = vec![
        Line::from(vec![
            Span::styled(
                app.config.get_keybind("new_file").to_uppercase(),
                Style::default().fg(colors.accent),
            ),
            Span::raw(format!("  {}", app.i18n.t("new_file"))),
        ]),
        Line::from(vec![
            Span::styled(
                app.config.get_keybind("toggle_explorer").to_uppercase(),
                Style::default().fg(colors.accent),
            ),
            Span::raw(format!("  {}", app.i18n.t("file_explorer"))),
        ]),
        Line::from(vec![
            Span::styled(
                app.config.get_keybind("open_file").to_uppercase(),
                Style::default().fg(colors.accent),
            ),
            Span::raw(format!("  {}", app.i18n.t("open_file_fuzzy"))),
        ]),
        Line::from(vec![
            Span::styled(
                app.config.get_keybind("global_search").to_uppercase(),
                Style::default().fg(colors.accent),
            ),
            Span::raw(format!("  {}", app.i18n.t("global_search"))),
        ]),
        Line::from(vec![
            Span::styled(
                app.config.get_keybind("theme_select").to_uppercase(),
                Style::default().fg(colors.accent),
            ),
            Span::raw(format!("  {}", app.i18n.t("select_theme"))),
        ]),
    ];

    f.render_widget(Paragraph::new(shortcuts).alignment(Alignment::Center), area);
}
