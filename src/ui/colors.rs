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

impl UIColors {
    /// Returns a readable muted secondary text color with guaranteed contrast
    /// against `self.bg` (never dark or unreadable).
    pub fn text_muted(&self) -> Color {
        match (self.bg, self.fg) {
            (Color::Rgb(br, bg, bb), Color::Rgb(fr, fg, fb)) => {
                let bg_lum = (br as u32 * 299 + bg as u32 * 587 + bb as u32 * 114) / 1000;
                if bg_lum < 128 {
                    // Dark background: ensure muted color is bright enough (>= 140 luminance)
                    let mr = (fr as u32 * 65 + br as u32 * 35) / 100;
                    let mg = (fg as u32 * 65 + bg as u32 * 35) / 100;
                    let mb = (fb as u32 * 65 + bb as u32 * 35) / 100;
                    let cur_lum = (mr * 299 + mg * 587 + mb * 114) / 1000;
                    if cur_lum < 140 {
                        Color::Rgb(166, 173, 200)
                    } else {
                        Color::Rgb(mr as u8, mg as u8, mb as u8)
                    }
                } else {
                    // Light background: ensure muted color is dark enough
                    let mr = (fr as u32 * 70 + br as u32 * 30) / 100;
                    let mg = (fg as u32 * 70 + bg as u32 * 30) / 100;
                    let mb = (fb as u32 * 70 + bb as u32 * 30) / 100;
                    let cur_lum = (mr * 299 + mg * 587 + mb * 114) / 1000;
                    if cur_lum > 110 {
                        Color::Rgb(80, 84, 98)
                    } else {
                        Color::Rgb(mr as u8, mg as u8, mb as u8)
                    }
                }
            }
            _ => Color::Rgb(166, 173, 200),
        }
    }

    /// Background color for interactive buttons distinct from `bg`.
    pub fn button_bg(&self) -> Color {
        match self.bg {
            Color::Rgb(r, g, b) => {
                let lum = (r as u32 * 299 + g as u32 * 587 + b as u32 * 114) / 1000;
                if lum < 128 {
                    Color::Rgb(r.saturating_add(30), g.saturating_add(30), b.saturating_add(45))
                } else {
                    Color::Rgb(r.saturating_sub(30), g.saturating_sub(30), b.saturating_sub(30))
                }
            }
            _ => self.surface,
        }
    }

    /// Primary / danger button background
    pub fn button_danger_bg(&self) -> Color {
        match self.error {
            Color::Rgb(r, g, b) => Color::Rgb(r.saturating_sub(40), g.saturating_sub(40), b.saturating_sub(40)),
            _ => self.error,
        }
    }
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
