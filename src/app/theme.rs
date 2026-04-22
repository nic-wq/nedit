use super::App;

impl App {
    pub fn apply_theme(&mut self, theme_name: String) {
        if self.theme_set.themes.contains_key(&theme_name) {
            self.current_theme = theme_name;
        }
    }

    pub fn save_current_theme(&self) {
        let home_dir = std::env::var("HOME")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        let theme_file = home_dir.join(".config/nedit/theme.txt");
        let _ = std::fs::write(theme_file, &self.current_theme);
    }
}
