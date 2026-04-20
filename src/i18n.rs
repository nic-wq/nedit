use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

const DOC_MAIN: &str = include_str!("../docs/docs.md");

pub struct I18n {
    pub translations: HashMap<String, String>,
    pub defaults: HashMap<String, String>,
}

impl I18n {
    pub fn load() -> Self {
        let mut defaults = HashMap::new();
        // General UI
        defaults.insert("welcome_to_nedit".to_string(), "Welcome to NEdit".to_string());
        defaults.insert("select_themes".to_string(), "to select themes".to_string());
        defaults.insert("for_help".to_string(), "for help".to_string());
        defaults.insert("explorer".to_string(), "Explorer".to_string());
        defaults.insert("no_name".to_string(), "[No Name]".to_string());
        defaults.insert("read_only".to_string(), "[READ ONLY]".to_string());
        defaults.insert("theme".to_string(), "Theme".to_string());
        defaults.insert("row".to_string(), "Row".to_string());
        defaults.insert("col".to_string(), "Col".to_string());
        
        // Welcome screen
        defaults.insert("new_file".to_string(), "New File".to_string());
        defaults.insert("file_explorer".to_string(), "File Explorer".to_string());
        defaults.insert("open_file_fuzzy".to_string(), "Open File (Fuzzy)".to_string());
        defaults.insert("global_search".to_string(), "Global Search".to_string());
        defaults.insert("select_theme".to_string(), "Select Theme".to_string());
        
        // Fuzzy Finder Titles
        defaults.insert("global_search_content".to_string(), "Global Search (Content)".to_string());
        defaults.insert("local_search_file".to_string(), "Local Search (Current File)".to_string());
        defaults.insert("fuzzy_finder_files".to_string(), "Fuzzy Finder (Files)".to_string());
        defaults.insert("select_color_theme".to_string(), "Select Color Theme".to_string());
        defaults.insert("save_as".to_string(), "Save As".to_string());
        defaults.insert("rename".to_string(), "Rename File".to_string());
        defaults.insert("delete_confirm".to_string(), "Delete File?".to_string());
        defaults.insert("file_options".to_string(), "File Options".to_string());
        defaults.insert("workspaces".to_string(), "Workspaces".to_string());
        defaults.insert("add_workspace_name".to_string(), "Workspace Name".to_string());
        defaults.insert("add_workspace_path".to_string(), "Workspace Path".to_string());
        defaults.insert("command_palette".to_string(), "Command Palette".to_string());
        defaults.insert("move_file".to_string(), "Move File".to_string());
        
        // Full Documentation (Default English)
        defaults.insert("full_docs".to_string(), DOC_MAIN.to_string());

        let home_dir = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
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
        self.translations.get(key)
            .or_else(|| self.defaults.get(key))
            .map(|s| s.as_str())
            .unwrap_or(key)
    }
}
