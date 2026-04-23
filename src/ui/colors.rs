use ratatui::style::Color;

use crate::app::App;

pub struct UIColors {
    pub bg: Color,
    pub fg: Color,
    pub sel: Color,
    pub accent: Color,
    pub surface: Color,
    pub error: Color,
}

pub fn get_colors(app: &App) -> UIColors {
    let theme = app.theme_set.themes.get(&app.current_theme);

    UIColors {
        bg: theme
            .and_then(|theme| theme.settings.background)
            .map(|c| Color::Rgb(c.r, c.g, c.b))
            .unwrap_or(Color::Rgb(30, 30, 46)),
        fg: theme
            .and_then(|theme| theme.settings.foreground)
            .map(|c| Color::Rgb(c.r, c.g, c.b))
            .unwrap_or(Color::Rgb(205, 214, 244)),
        sel: theme
            .and_then(|theme| theme.settings.selection)
            .map(|c| Color::Rgb(c.r, c.g, c.b))
            .unwrap_or(Color::Rgb(69, 71, 90)),
        accent: Color::Rgb(137, 180, 250),
        surface: Color::Rgb(49, 50, 68),
        error: Color::Rgb(243, 139, 168),
    }
}
