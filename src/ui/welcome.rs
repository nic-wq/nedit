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
        .constraints([Constraint::Length(9), Constraint::Length(12)])
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

    // We use a horizontal layout with flexible margins (Constraint::Min(0)) 
    // to ensure the logo is always horizontally centered regardless of terminal width.
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
    let items = vec![
        (
            app.config.get_keybind("command_palette").to_uppercase(),
            app.i18n.t("command_palette"),
        ),
        (
            app.config.get_keybind("new_file").to_uppercase(),
            app.i18n.t("new_file"),
        ),
        (
            app.config.get_keybind("open_file").to_uppercase(),
            app.i18n.t("file_name_search"),
        ),
        (
            app.config.get_keybind("toggle_explorer").to_uppercase(),
            app.i18n.t("toggle_explorer"),
        ),
        (
            app.config.get_keybind("global_search").to_uppercase(),
            app.i18n.t("content_search"),
        ),
        (
            app.config.get_keybind("open_help").to_uppercase(),
            app.i18n.t("help_documentation"),
        ),
    ];

    let max_shortcut_w = items
        .iter()
        .map(|(sc, _)| UnicodeWidthStr::width(sc.as_str()))
        .max()
        .unwrap_or(0);
    let max_desc_w = items
        .iter()
        .map(|(_, desc)| UnicodeWidthStr::width(*desc))
        .max()
        .unwrap_or(0);
    let gap = 3;
    let total_width = (max_shortcut_w + gap + max_desc_w) as u16;

    let shortcut_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(total_width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(area);

    let lines: Vec<Line> = items
        .into_iter()
        .map(|(sc, desc)| {
            let sc_w = UnicodeWidthStr::width(sc.as_str());
            let padding = " ".repeat(max_shortcut_w.saturating_sub(sc_w) + gap);
            Line::from(vec![
                Span::styled(sc, Style::default().fg(colors.accent)),
                Span::raw(format!("{}{}", padding, desc)),
            ])
        })
        .collect();

    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Left),
        shortcut_chunks[1],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use crate::ui::colors::get_colors;

    #[test]
    fn welcome_shortcuts_are_column_aligned() {
        let app = App::new(&[]);
        let width = 80;
        let height = 24;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_shortcuts(f, &app, Rect::new(0, 0, width, height), &colors);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();

        // All 6 shortcuts must start at the exact same column:
        let mut ctrl_positions = Vec::new();
        for y in 0..height {
            let row_str: String = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
            if let Some(pos) = row_str.find("CTRL+") {
                ctrl_positions.push(pos);
            }
        }
        assert_eq!(ctrl_positions.len(), 6);
        let first_ctrl_x = ctrl_positions[0];
        for pos in &ctrl_positions {
            assert_eq!(*pos, first_ctrl_x, "All shortcuts must start at the exact same column");
        }

        // All 6 descriptions must start at the exact same column:
        let descs = [
            "Command Palette",
            "New File",
            "File Name Search",
            "Toggle File Explorer",
            "Content Search",
            "Help & Documentation",
        ];
        let mut desc_positions = Vec::new();
        for desc in &descs {
            for y in 0..height {
                let row_str: String = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
                if let Some(pos) = row_str.find(desc) {
                    desc_positions.push(pos);
                    break;
                }
            }
        }
        assert_eq!(desc_positions.len(), 6);
        let first_desc_x = desc_positions[0];
        for pos in &desc_positions {
            assert_eq!(*pos, first_desc_x, "All descriptions must start at the exact same column");
        }
        assert_eq!(first_desc_x, first_ctrl_x + 6 + 3);
    }
}
