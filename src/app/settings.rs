use super::{App, FuzzyMode, NotificationType};
use crate::app::types::SettingId;

impl App {
    pub fn open_settings(&mut self) {
        self.toggle_fuzzy(FuzzyMode::Settings);
    }

    pub fn get_setting_value(&self, id: SettingId) -> String {
        match id {
            SettingId::Autocomplete => {
                if self.config.autocomplete_enabled {
                    format!("[ {} ]", self.i18n.t("enabled"))
                } else {
                    format!("[ {} ]", self.i18n.t("disabled"))
                }
            }
            SettingId::IndentGuides => {
                if self.config.show_indent_guides {
                    format!("[ {} ]", self.i18n.t("enabled"))
                } else {
                    format!("[ {} ]", self.i18n.t("disabled"))
                }
            }
            SettingId::FilePreview => {
                if self.config.preview_enabled {
                    format!("[ {} ]", self.i18n.t("enabled"))
                } else {
                    format!("[ {} ]", self.i18n.t("disabled"))
                }
            }
            SettingId::BracketMatching => {
                if self.config.highlight_matching_bracket {
                    format!("[ {} ]", self.i18n.t("enabled"))
                } else {
                    format!("[ {} ]", self.i18n.t("disabled"))
                }
            }
            SettingId::NotificationPosition => {
                format!("[ {} ]", self.config.notification_position.as_config_str())
            }
            SettingId::Theme => {
                format!("[ {} ]", self.config.theme)
            }
            SettingId::PreviewMaxSize => {
                let mb = (self.config.preview_max_size / (1024 * 1024)).max(1);
                format!("[ {} MB ]", mb)
            }
        }
    }

    pub fn toggle_or_edit_setting(&mut self, id: SettingId) {
        if id.is_free_choice() {
            self.setting_input_target = Some(id);
            self.fuzzy_mode = FuzzyMode::SettingInput;
            let current_mb = (self.config.preview_max_size / (1024 * 1024)).max(1);
            self.set_fuzzy_query(current_mb.to_string());
            self.needs_redraw = true;
            return;
        }

        match id {
            SettingId::Autocomplete => {
                self.config.autocomplete_enabled = !self.config.autocomplete_enabled;
                let status = if self.config.autocomplete_enabled {
                    self.i18n.t("enabled")
                } else {
                    self.i18n.t("disabled")
                };
                self.show_notification(
                    format!("{}: Autocomplete = {}", self.i18n.t("setting_updated"), status),
                    NotificationType::Info,
                );
            }
            SettingId::IndentGuides => {
                self.config.show_indent_guides = !self.config.show_indent_guides;
                let status = if self.config.show_indent_guides {
                    self.i18n.t("enabled")
                } else {
                    self.i18n.t("disabled")
                };
                self.show_notification(
                    format!("{}: Indent Guides = {}", self.i18n.t("setting_updated"), status),
                    NotificationType::Info,
                );
            }
            SettingId::FilePreview => {
                self.config.preview_enabled = !self.config.preview_enabled;
                if !self.config.preview_enabled {
                    if let Some(idx) = self.preview_buffer_idx {
                        self.clear_preview(idx);
                    }
                }
                let status = if self.config.preview_enabled {
                    self.i18n.t("enabled")
                } else {
                    self.i18n.t("disabled")
                };
                self.show_notification(
                    format!("{}: Preview = {}", self.i18n.t("setting_updated"), status),
                    NotificationType::Info,
                );
            }
            SettingId::BracketMatching => {
                self.config.highlight_matching_bracket = !self.config.highlight_matching_bracket;
                let status = if self.config.highlight_matching_bracket {
                    self.i18n.t("enabled")
                } else {
                    self.i18n.t("disabled")
                };
                self.show_notification(
                    format!("{}: Bracket Matching = {}", self.i18n.t("setting_updated"), status),
                    NotificationType::Info,
                );
            }
            SettingId::NotificationPosition => {
                self.config.notification_position = self.config.notification_position.next();
                self.show_notification(
                    format!(
                        "{}: Notification Position = {}",
                        self.i18n.t("setting_updated"),
                        self.config.notification_position.as_config_str()
                    ),
                    NotificationType::Info,
                );
            }
            SettingId::Theme => {
                self.ensure_all_themes_loaded();
                let mut theme_names: Vec<String> = self.theme_set.themes.keys().cloned().collect();
                theme_names.sort();
                if !theme_names.is_empty() {
                    let current_key = self.resolve_theme_key(&self.config.theme);
                    let next_idx = theme_names
                        .iter()
                        .position(|t| t == &current_key)
                        .map(|i| (i + 1) % theme_names.len())
                        .unwrap_or(0);
                    let next_theme = theme_names[next_idx].clone();
                    self.apply_theme(next_theme.clone());
                    self.config.theme = next_theme.clone();
                    self.save_current_theme();
                    self.show_notification(
                        format!("{}: Theme = {}", self.i18n.t("setting_updated"), next_theme),
                        NotificationType::Info,
                    );
                }
            }
            SettingId::PreviewMaxSize => {}
        }

        if let Err(e) = self.config.save() {
            self.show_notification(
                format!("Error saving config: {}", e),
                NotificationType::Error,
            );
        }
        self.update_fuzzy(false);
        self.needs_redraw = true;
    }

    pub fn apply_setting_input(&mut self) {
        if let Some(target) = self.setting_input_target.take() {
            if target == SettingId::PreviewMaxSize {
                let input = self.fuzzy_query.trim().to_lowercase();
                let num_str = input.trim_end_matches("mb").trim_end_matches('m').trim();
                if let Ok(mb) = num_str.parse::<usize>() {
                    if mb > 0 {
                        self.config.preview_max_size = mb * 1024 * 1024;
                        if let Err(e) = self.config.save() {
                            self.show_notification(
                                format!("Error saving config: {}", e),
                                NotificationType::Error,
                            );
                        } else {
                            self.show_notification(
                                format!(
                                    "{}: Preview Max Size = {} MB",
                                    self.i18n.t("setting_updated"),
                                    mb
                                ),
                                NotificationType::Info,
                            );
                        }
                        self.toggle_fuzzy(FuzzyMode::Settings);
                        self.needs_redraw = true;
                        return;
                    }
                }
                self.setting_input_target = Some(target);
                self.show_notification(
                    "Invalid size (enter a positive number in MB, e.g. 5)".to_string(),
                    NotificationType::Error,
                );
                self.needs_redraw = true;
                return;
            }
        }
        self.toggle_fuzzy(FuzzyMode::Settings);
        self.needs_redraw = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_boolean_setting() {
        let mut app = App::new(&[]);
        assert!(app.config.autocomplete_enabled);

        app.toggle_or_edit_setting(SettingId::Autocomplete);
        assert!(!app.config.autocomplete_enabled);

        app.toggle_or_edit_setting(SettingId::Autocomplete);
        assert!(app.config.autocomplete_enabled);
    }

    #[test]
    fn test_cycle_notification_position_setting() {
        let mut app = App::new(&[]);
        assert_eq!(
            app.config.notification_position,
            crate::config::NotificationPosition::BottomRight
        );

        app.toggle_or_edit_setting(SettingId::NotificationPosition);
        assert_eq!(
            app.config.notification_position,
            crate::config::NotificationPosition::BottomLeft
        );
    }

    #[test]
    fn test_free_choice_setting_opens_setting_input() {
        let mut app = App::new(&[]);
        app.toggle_or_edit_setting(SettingId::PreviewMaxSize);

        assert_eq!(app.fuzzy_mode, FuzzyMode::SettingInput);
        assert_eq!(app.setting_input_target, Some(SettingId::PreviewMaxSize));
        assert_eq!(app.fuzzy_query, "3");

        // Now edit and apply
        app.set_fuzzy_query("7".to_string());
        app.apply_setting_input();

        assert_eq!(app.fuzzy_mode, FuzzyMode::Settings);
        assert_eq!(app.config.preview_max_size, 7 * 1024 * 1024);
    }
}
