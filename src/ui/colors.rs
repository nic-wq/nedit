use ratatui::style::Color;
use syntect::highlighting::Theme;

use crate::app::App;

pub struct UIColors {
    pub bg: Color,
    pub fg: Color,
    pub sel: Color,
    pub accent: Color,
    pub surface: Color,
    pub error: Color,
    pub indent_guide: Color,
    pub active_indent_guide: Color,
}

fn map_color(color: syntect::highlighting::Color) -> Color {
    Color::Rgb(color.r, color.g, color.b)
}

pub fn get_colors(app: &App) -> UIColors {
    let theme = app.theme_set.themes.get(&app.current_theme);

    UIColors {
        // We use hardcoded fallback colors (Catppuccin-like palette) to ensure the UI
        // remains usable and beautiful even if the selected theme is missing certain keys.
        bg: theme
            .and_then(|theme| theme.settings.background)
            .map(map_color)
            .unwrap_or(Color::Rgb(30, 30, 46)),
        fg: theme
            .and_then(|theme| theme.settings.foreground)
            .map(map_color)
            .unwrap_or(Color::Rgb(205, 214, 244)),
        sel: theme
            .and_then(|theme| theme.settings.selection)
            .map(map_color)
            .unwrap_or(Color::Rgb(69, 71, 90)),
        accent: theme
            .and_then(theme_accent)
            .map(map_color)
            .unwrap_or(Color::Rgb(137, 180, 250)),
        surface: theme
            .and_then(theme_surface)
            .map(map_color)
            .unwrap_or(Color::Rgb(49, 50, 68)),
        error: theme
            .and_then(theme_error)
            .map(map_color)
            .unwrap_or(Color::Rgb(243, 139, 168)),
        indent_guide: theme
            .and_then(theme_indent_guide)
            .map(map_color)
            .unwrap_or(Color::Rgb(45, 46, 66)),
        active_indent_guide: theme
            .and_then(theme_accent)
            .map(map_color)
            .unwrap_or(Color::Rgb(137, 180, 250)),
    }
}

fn theme_indent_guide(theme: &Theme) -> Option<syntect::highlighting::Color> {
    theme
        .settings
        .gutter
        .or(theme.settings.line_highlight)
        .or(theme.settings.selection)
}

fn theme_accent(theme: &Theme) -> Option<syntect::highlighting::Color> {
    theme
        .settings
        .accent
        .or(theme.settings.caret)
        .or(theme.settings.selection_foreground)
        .or(theme.settings.foreground)
}

fn theme_surface(theme: &Theme) -> Option<syntect::highlighting::Color> {
    theme
        .settings
        .gutter
        .or(theme.settings.line_highlight)
        .or(theme.settings.selection)
        .or(theme.settings.background)
}

fn theme_error(theme: &Theme) -> Option<syntect::highlighting::Color> {
    theme
        .settings
        .highlight
        .or(theme.settings.find_highlight)
        .or(theme.settings.accent)
}
