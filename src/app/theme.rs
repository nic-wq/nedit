use super::App;

impl App {
    pub fn resolve_theme_key(&self, display_name: &str) -> String {
        if self.theme_set.themes.contains_key(display_name) {
            return display_name.to_string();
        }
        for (key, theme) in &self.theme_set.themes {
            if theme.name.as_deref() == Some(display_name) {
                return key.clone();
            }
        }
        display_name.to_string()
    }

    pub fn apply_theme(&mut self, theme_name: String) {
        let key = self.resolve_theme_key(&theme_name);
        if !self.theme_set.themes.contains_key(&key) {
            self.ensure_all_themes_loaded();
        }
        if self.theme_set.themes.contains_key(&key) {
            self.current_theme = key;
        }
    }

    pub fn save_current_theme(&self) {
        let theme_file = Self::config_dir().join("theme.txt");
        let _ = std::fs::write(theme_file, &self.current_theme);
    }
}
