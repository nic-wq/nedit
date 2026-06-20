use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_true")]
    pub autocomplete_enabled: bool,
    #[serde(default)]
    pub keybinds: HashMap<String, String>,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_true")]
    pub show_indent_guides: bool,
    #[serde(default = "default_true")]
    pub show_scope_breadcrumbs: bool,
}

fn default_true() -> bool {
    true
}

fn default_theme() -> String {
    "NEdit Dark".to_string()
}

impl Config {
    pub fn load() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nedit");
        let config_path = config_dir.join("config.toml");

        if let Ok(content) = fs::read_to_string(&config_path) {
            return Self::from_toml_with_defaults(&content).unwrap_or_else(|_| Self::default());
        }

        Self::default()
    }

    pub fn default() -> Self {
        let mut keybinds = HashMap::new();
        // We provide a comprehensive set of default keybindings so the editor is 
        // immediately usable "out of the box" without requiring initial configuration.
        keybinds.insert("quit".to_string(), "ctrl+q".to_string());
        keybinds.insert("new_file".to_string(), "ctrl+n".to_string());
        keybinds.insert("open_file".to_string(), "ctrl+o".to_string());
        keybinds.insert("command_palette".to_string(), "ctrl+p".to_string());
        keybinds.insert("global_search".to_string(), "ctrl+g".to_string());
        keybinds.insert("local_search".to_string(), "ctrl+f".to_string());
        keybinds.insert("save".to_string(), "ctrl+s".to_string());
        keybinds.insert("toggle_explorer".to_string(), "ctrl+e".to_string());
        keybinds.insert("theme_select".to_string(), "ctrl+alt+t".to_string());
        keybinds.insert("toggle_focus".to_string(), "shift+backtab".to_string());
        keybinds.insert("close_tab".to_string(), "ctrl+w".to_string());
        keybinds.insert("undo".to_string(), "ctrl+z".to_string());
        keybinds.insert("redo".to_string(), "ctrl+y".to_string());
        keybinds.insert("copy".to_string(), "ctrl+c".to_string());
        keybinds.insert("paste".to_string(), "ctrl+v".to_string());
        keybinds.insert("cut".to_string(), "ctrl+x".to_string());
        keybinds.insert("select_all".to_string(), "ctrl+a".to_string());
        keybinds.insert("select_line".to_string(), "ctrl+l".to_string());
        keybinds.insert("open_help".to_string(), "ctrl+h".to_string());
        keybinds.insert("run_live_script".to_string(), "f9".to_string());
        keybinds.insert(
            "live_script_next".to_string(),
            "shift+alt+right".to_string(),
        );
        keybinds.insert("live_script_prev".to_string(), "shift+alt+left".to_string());
        keybinds.insert("set_as_root".to_string(), "ctrl+enter".to_string());

        Self {
            autocomplete_enabled: true,
            keybinds,
            theme: default_theme(),
            show_indent_guides: true,
            show_scope_breadcrumbs: true,
        }
    }

    pub fn get_keybind(&self, action: &str) -> String {
        self.keybinds.get(action).cloned().unwrap_or_else(|| {
            Self::default()
                .keybinds
                .get(action)
                .cloned()
                .unwrap_or_default()
        })
    }

    fn from_toml_with_defaults(content: &str) -> Result<Self, toml::de::Error> {
        let mut config = Self::default();
        let value = toml::from_str::<toml::Value>(content)?;

        if let Some(enabled) = value
            .get("autocomplete_enabled")
            .and_then(toml::Value::as_bool)
        {
            config.autocomplete_enabled = enabled;
        }

        if let Some(theme) = value.get("theme").and_then(toml::Value::as_str) {
            config.theme = theme.to_string();
        }

        if let Some(enabled) = value
            .get("show_indent_guides")
            .and_then(toml::Value::as_bool)
        {
            config.show_indent_guides = enabled;
        }

        if let Some(enabled) = value
            .get("show_scope_breadcrumbs")
            .and_then(toml::Value::as_bool)
        {
            config.show_scope_breadcrumbs = enabled;
        }

        if let Some(keybinds) = value.get("keybinds").and_then(toml::Value::as_table) {
            for (action, key) in keybinds {
                if let Some(key) = key.as_str() {
                    config.keybinds.insert(action.clone(), key.to_string());
                }
            }
        }

        if let Some(table) = value.as_table() {
            for action in Self::default_keybind_actions() {
                if let Some(key) = table.get(*action).and_then(toml::Value::as_str) {
                    config.keybinds.insert(action.to_string(), key.to_string());
                }
            }
        }

        Ok(config)
    }

    fn default_keybind_actions() -> &'static [&'static str] {
        &[
            "quit",
            "new_file",
            "open_file",
            "command_palette",
            "global_search",
            "local_search",
            "save",
            "toggle_explorer",
            "theme_select",
            "toggle_focus",
            "close_tab",
            "undo",
            "redo",
            "copy",
            "paste",
            "cut",
            "select_all",
            "select_line",
            "open_help",
            "run_live_script",
            "live_script_next",
            "live_script_prev",
            "set_as_root",
        ]
    }
}
