use ratatui::style::Color;

use crate::app::App;

pub struct UIColors {
    pub bg: Color,
    pub fg: Color,
    pub sel: Color,
    pub accent: Color,
    pub surface: Color,
}

pub fn get_colors(app: &App) -> UIColors {
    let theme = app
        .theme_set
        .themes
        .get(&app.current_theme)
        .unwrap_or_else(|| &app.theme_set.themes["base16-ocean.dark"]);

    UIColors {
        bg: theme
            .settings
            .background
            .map(|c| Color::Rgb(c.r, c.g, c.b))
            .unwrap_or(Color::Rgb(30, 30, 46)),
        fg: theme
            .settings
            .foreground
            .map(|c| Color::Rgb(c.r, c.g, c.b))
            .unwrap_or(Color::Rgb(205, 214, 244)),
        sel: theme
            .settings
            .selection
            .map(|c| Color::Rgb(c.r, c.g, c.b))
            .unwrap_or(Color::Rgb(69, 71, 90)),
        accent: Color::Rgb(137, 180, 250),
        surface: Color::Rgb(49, 50, 68),
    }
}
