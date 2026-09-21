use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct I18n {
    pub translations: HashMap<String, String>,
    pub defaults: HashMap<String, String>,
}

impl I18n {
    pub fn load() -> Self {
        let mut defaults = HashMap::new();

        // General UI & Status Bar
        defaults.insert("welcome_to_nedit".to_string(), "Welcome to NEdit".to_string());
        defaults.insert("select_themes".to_string(), "to select themes".to_string());
        defaults.insert("for_help".to_string(), "for help".to_string());
        defaults.insert("explorer".to_string(), "Explorer".to_string());
        defaults.insert("no_name".to_string(), "[No Name]".to_string());
        defaults.insert("read_only".to_string(), "[READ ONLY]".to_string());
        defaults.insert("theme".to_string(), "Theme".to_string());
        defaults.insert("row".to_string(), "Row".to_string());
        defaults.insert("col".to_string(), "Col".to_string());

        // Modals & Titles
        defaults.insert("new_file".to_string(), "New File".to_string());
        defaults.insert("new_folder".to_string(), "New Folder".to_string());
        defaults.insert("file_explorer".to_string(), "File Explorer".to_string());
        defaults.insert("open_file_fuzzy".to_string(), "Open File (Fuzzy)".to_string());
        defaults.insert("global_search".to_string(), "Global Search".to_string());
        defaults.insert("select_theme".to_string(), "Select Theme".to_string());
        defaults.insert("global_search_content".to_string(), "Global Search (Content)".to_string());
        defaults.insert("local_search_file".to_string(), "Local Search (Current File)".to_string());
        defaults.insert("fuzzy_finder_files".to_string(), "Fuzzy Finder (Files)".to_string());
        defaults.insert("select_color_theme".to_string(), "Select Color Theme".to_string());
        defaults.insert("save_as".to_string(), "Save As".to_string());
        defaults.insert("rename".to_string(), "Rename File".to_string());
        defaults.insert("delete_confirm".to_string(), "Delete File?".to_string());
        defaults.insert("file_options".to_string(), "File Options".to_string());
        defaults.insert("command_palette".to_string(), "Command Palette".to_string());
        defaults.insert("move_file".to_string(), "Move File".to_string());
        defaults.insert("unsaved_changes".to_string(), "Unsaved Changes".to_string());
        defaults.insert("external_change".to_string(), "External Change".to_string());
        defaults.insert("file_changed".to_string(), "File Changed".to_string());
        defaults.insert("file_changed_externally".to_string(), "File changed on disk. Reload?".to_string());
        defaults.insert("select_documentation".to_string(), "Select Documentation".to_string());

        // Modal Prompts & Descriptions
        defaults.insert("new_file_folder_title".to_string(), "New File / Folder".to_string());
        defaults.insert("new_file_folder_prompt".to_string(), "Enter path (trailing / creates a folder):".to_string());
        defaults.insert("rename_item_title".to_string(), "Rename Item".to_string());
        defaults.insert("rename_item_prompt".to_string(), "Enter new name:".to_string());
        defaults.insert("save_buffer_as_title".to_string(), "Save Buffer As".to_string());
        defaults.insert("save_buffer_as_prompt".to_string(), "Enter destination file path:".to_string());
        defaults.insert("delete_prompt".to_string(), "Are you sure you want to delete?".to_string());
        defaults.insert("delete_warning".to_string(), "This action cannot be undone.".to_string());
        defaults.insert("unsaved_prompt".to_string(), "Save changes to file before closing?".to_string());
        defaults.insert("unsaved_quit_prompt".to_string(), "Quit application? Save changes before exit?".to_string());
        defaults.insert("unsaved_script_prompt".to_string(), "Close Live Script? Modifications will be lost.".to_string());
        defaults.insert("unsaved_warning".to_string(), "Unsaved modifications will be permanently lost.".to_string());
        defaults.insert("external_change_prompt".to_string(), "File changed on disk by another application:".to_string());
        defaults.insert("external_change_warning".to_string(), "Choose whether to reload from disk or keep editor version.".to_string());

        // Modal Action Buttons
        defaults.insert("save".to_string(), "Save".to_string());
        defaults.insert("discard".to_string(), "Discard".to_string());
        defaults.insert("confirm".to_string(), "Confirm".to_string());
        defaults.insert("delete".to_string(), "Delete".to_string());
        defaults.insert("cancel".to_string(), "Cancel".to_string());
        defaults.insert("reload".to_string(), "Reload".to_string());
        defaults.insert("keep".to_string(), "Keep".to_string());
        defaults.insert("set_as_root".to_string(), "Set as Root".to_string());

        // Context Menu Actions
        defaults.insert("copy".to_string(), "Copy".to_string());
        defaults.insert("cut".to_string(), "Cut".to_string());
        defaults.insert("paste".to_string(), "Paste".to_string());
        defaults.insert("paste_replace".to_string(), "Paste (Replace)".to_string());
        defaults.insert("delete_selection".to_string(), "Delete Selection".to_string());
        defaults.insert("select_all".to_string(), "Select All".to_string());
        defaults.insert("select_word".to_string(), "Select Word".to_string());
        defaults.insert("open".to_string(), "Open".to_string());
        defaults.insert("copy_path".to_string(), "Copy Path".to_string());

        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let config_dir = home_dir.join(".config/nedit");

        let mut translations = HashMap::new();
        let possible_files = vec!["language.toml", "language.txt", "lang.toml"];
        for file in possible_files {
            let lang_file = config_dir.join(file);
            if let Ok(content) = fs::read_to_string(&lang_file) {
                if let Ok(map) = toml::from_str::<HashMap<String, toml::Value>>(&content) {
                    for (k, v) in map {
                        if k == "messages" {
                            if let Some(msg_table) = v.as_table() {
                                for (mk, mv) in msg_table {
                                    if let Some(s) = mv.as_str() {
                                        translations.insert(mk.clone(), s.to_string());
                                    }
                                }
                            }
                        } else if let Some(s) = v.as_str() {
                            translations.insert(k, s.to_string());
                        }
                    }
                    break;
                }
            }
        }

        Self {
            translations,
            defaults,
        }
    }

    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.translations
            .get(key)
            .or_else(|| self.defaults.get(key))
            .map(|s| s.as_str())
            .unwrap_or(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_translations_cover_all_standard_keys() {
        let i18n = I18n::load();
        assert_eq!(i18n.t("save"), "Save");
        assert_eq!(i18n.t("delete"), "Delete");
        assert_eq!(i18n.t("cancel"), "Cancel");
        assert_eq!(i18n.t("confirm"), "Confirm");
        assert_eq!(i18n.t("copy"), "Copy");
        assert_eq!(i18n.t("cut"), "Cut");
        assert_eq!(i18n.t("paste"), "Paste");
        assert_eq!(i18n.t("paste_replace"), "Paste (Replace)");
        assert_eq!(i18n.t("delete_prompt"), "Are you sure you want to delete?");
        assert_eq!(i18n.t("delete_warning"), "This action cannot be undone.");
        assert_eq!(i18n.t("select_documentation"), "Select Documentation");
    }

    #[test]
    fn test_custom_translations_override_defaults() {
        let mut i18n = I18n::load();
        i18n.translations.insert("save".to_string(), "Salvar".to_string());
        i18n.translations.insert("delete".to_string(), "Excluir".to_string());
        assert_eq!(i18n.t("save"), "Salvar");
        assert_eq!(i18n.t("delete"), "Excluir");
        // Non-overridden key still returns default
        assert_eq!(i18n.t("cancel"), "Cancel");
    }

    #[test]
    fn test_unknown_key_falls_back_to_key_itself() {
        let i18n = I18n::load();
        assert_eq!(i18n.t("some_unknown_nonexistent_key"), "some_unknown_nonexistent_key");
    }

    #[test]
    fn test_documentation_is_not_translatable_via_i18n() {
        let i18n = I18n::load();
        // Documentation is intentionally NOT part of the i18n system
        assert!(!i18n.defaults.contains_key("full_docs"));
        assert!(!i18n.translations.contains_key("full_docs"));
    }
}
