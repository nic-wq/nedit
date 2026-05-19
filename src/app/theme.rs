use super::App;

impl App {
    pub fn apply_theme(&mut self, theme_name: String) {
        // We load themes lazily only when requested to speed up initial startup, 
        // as the user might only use one theme for a long time.
        if !self.theme_set.themes.contains_key(&theme_name) {
            self.ensure_all_themes_loaded();
        }

        if self.theme_set.themes.contains_key(&theme_name) {
            self.current_theme = theme_name;
        }
    }

    pub fn save_current_theme(&self) {
        let theme_file = Self::config_dir().join("theme.txt");
        let _ = std::fs::write(theme_file, &self.current_theme);
    }
}
