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

    pub fn is_theme_active(&self, theme_name: &str) -> bool {
        let reference_theme = if !self.original_theme.is_empty() {
            &self.original_theme
        } else {
            &self.current_theme
        };
        if theme_name == reference_theme {
            return true;
        }
        let orig_key = self.resolve_theme_key(reference_theme);
        let theme_key = self.resolve_theme_key(theme_name);
        if orig_key == theme_key {
            return true;
        }
        if let Some(t) = self.theme_set.themes.get(&orig_key) {
            if t.name.as_deref() == Some(theme_name) {
                return true;
            }
        }
        if let Some(t) = self.theme_set.themes.get(reference_theme) {
            if t.name.as_deref() == Some(theme_name) {
                return true;
            }
        }
        false
    }

    pub fn apply_theme(&mut self, theme_name: String) {
        let key = self.resolve_theme_key(&theme_name);
        if !self.theme_set.themes.contains_key(&key) {
            self.ensure_all_themes_loaded();
        }
        if self.theme_set.themes.contains_key(&key) {
            self.current_theme = key;
            for buf in self.buffers.iter_mut() {
                buf.invalidate_all_rendered_spans();
            }
        }
    }

    pub fn save_current_theme(&self) {
        let theme_file = Self::config_dir().join("theme.txt");
        let _ = std::fs::write(theme_file, &self.current_theme);
    }
}

#[cfg(test)]
mod tests {
    use super::App;

    #[test]
    fn test_is_theme_active() {
        let mut app = App::new(&[]);
        app.ensure_all_themes_loaded();
        app.current_theme = "NEdit Dark Complete".to_string();
        app.original_theme = "NEdit Dark Complete".to_string();

        assert!(app.is_theme_active("NEdit Dark Complete"));
        assert!(!app.is_theme_active("Solarized (Dark)"));

        // During live preview, current_theme changes, but original_theme stays active:
        app.current_theme = "Solarized (Dark)".to_string();
        assert!(app.is_theme_active("NEdit Dark Complete"));
        assert!(!app.is_theme_active("Solarized (Dark)"));
    }
}
