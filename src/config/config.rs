use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Corner of the screen where notification toasts appear.
/// Set with `notification_position` in `config.toml`, e.g. `"top-right"`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NotificationPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    #[default]
    BottomRight,
}

impl NotificationPosition {
    /// Parse a user-provided value, forgiving separators and case.
    /// Unknown values fall back to the default (bottom-right).
    pub fn parse(s: &str) -> Self {
        match s
            .trim()
            .to_lowercase()
            .replace(['-', '_', ' '], "")
            .as_str()
        {
            "topleft" => Self::TopLeft,
            "topright" | "top" => Self::TopRight,
            "bottomleft" => Self::BottomLeft,
            "bottomright" | "bottom" => Self::BottomRight,
            _ => Self::BottomRight,
        }
    }

    pub fn as_config_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::BottomRight => Self::BottomLeft,
            Self::BottomLeft => Self::TopLeft,
            Self::TopLeft => Self::TopRight,
            Self::TopRight => Self::BottomRight,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
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
    pub preview_enabled: bool,
    #[serde(default = "default_preview_max_size")]
    pub preview_max_size: usize,
    #[serde(default = "default_true")]
    pub highlight_matching_bracket: bool,
    #[serde(default)]
    pub notification_position: NotificationPosition,
}

fn default_true() -> bool {
    true
}

fn default_preview_max_size() -> usize {
    3 * 1024 * 1024 // 3 MB
}

fn default_theme() -> String {
    "NEdit Dark Complete".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nedit");
        config_dir.join("config.toml")
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        fs::write(path, toml_str)
    }

    pub fn load() -> Self {
        let config_path = Self::config_path();

        if let Ok(content) = fs::read_to_string(&config_path) {
            return Self::from_toml_with_defaults(&content).unwrap_or_else(|_| Self::new());
        }

        Self::new()
    }

    pub fn new() -> Self {
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
        keybinds.insert("toggle_explorer".to_string(), "ctrl+b".to_string());
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
        keybinds.insert("set_as_root".to_string(), "ctrl+enter".to_string());

        Self {
            autocomplete_enabled: true,
            keybinds,
            theme: default_theme(),
            show_indent_guides: true,
            preview_enabled: true,
            preview_max_size: default_preview_max_size(),
            highlight_matching_bracket: true,
            notification_position: NotificationPosition::default(),
        }
    }

    pub fn get_keybind(&self, action: &str) -> String {
        self.keybinds.get(action).cloned().unwrap_or_else(|| {
            Self::new()
                .keybinds
                .get(action)
                .cloned()
                .unwrap_or_default()
        })
    }

    pub fn get_command_shortcut(&self, command: &str) -> Option<String> {
        let action = match command {
            "Save" => "save",
            "New File" => "new_file",
            "Open File" | "File Name Search" => "open_file",
            "Close Tab" => "close_tab",
            "Toggle Explorer" => "toggle_explorer",
            "Global Search" | "Content Search" => "global_search",
            "Local Search" => "local_search",
            "Switch Theme" => "theme_select",
            "Quit" => "quit",
            "Undo" => "undo",
            "Redo" => "redo",
            "Copy" => "copy",
            "Paste" => "paste",
            "Cut" => "cut",
            "Select All" => "select_all",
            "Open Help" => "open_help",
            "Command Palette" => "command_palette",
            "Settings" => "open_settings",
            _ => return None,
        };
        let kb = self.get_keybind(action);
        if kb.is_empty() {
            None
        } else {
            Some(kb.to_uppercase())
        }
    }

    fn from_toml_with_defaults(content: &str) -> Result<Self, toml::de::Error> {
        let mut config = Self::new();
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
            .get("preview_enabled")
            .and_then(toml::Value::as_bool)
        {
            config.preview_enabled = enabled;
        }

        if let Some(size) = value
            .get("preview_max_size")
            .and_then(toml::Value::as_integer)
        {
            config.preview_max_size = size as usize;
        }

        if let Some(enabled) = value
            .get("highlight_matching_bracket")
            .and_then(toml::Value::as_bool)
            .or_else(|| find_bool_recursive(&value, "highlight_matching_bracket"))
        {
            config.highlight_matching_bracket = enabled;
        }

        if let Some(pos) = value
            .get("notification_position")
            .and_then(toml::Value::as_str)
        {
            config.notification_position = NotificationPosition::parse(pos);
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
            "set_as_root",
            "open_settings",
        ]
    }
}

fn find_bool_recursive(value: &toml::Value, key: &str) -> Option<bool> {
    match value {
        toml::Value::Table(table) => {
            for (k, v) in table {
                if k == key {
                    if let Some(b) = v.as_bool() {
                        return Some(b);
                    }
                }
                if let Some(b) = find_bool_recursive(v, key) {
                    return Some(b);
                }
            }
            None
        }
        toml::Value::Array(array) => {
            for v in array {
                if let Some(b) = find_bool_recursive(v, key) {
                    return Some(b);
                }
            }
            None
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::NotificationPosition;

    #[test]
    fn notification_position_parses_all_corners() {
        use NotificationPosition::*;
        assert_eq!(NotificationPosition::parse("top-left"), TopLeft);
        assert_eq!(NotificationPosition::parse("top_right"), TopRight);
        assert_eq!(NotificationPosition::parse("BOTTOMLEFT"), BottomLeft);
        assert_eq!(NotificationPosition::parse("bottom-right"), BottomRight);
        assert_eq!(NotificationPosition::parse("top"), TopRight);
        assert_eq!(NotificationPosition::parse("bottom"), BottomRight);
    }

    #[test]
    fn notification_position_falls_back_to_default() {
        assert_eq!(
            NotificationPosition::parse("center"),
            NotificationPosition::default()
        );
        assert_eq!(
            NotificationPosition::parse(""),
            NotificationPosition::default()
        );
        assert_eq!(
            NotificationPosition::default(),
            NotificationPosition::BottomRight
        );
    }

    #[test]
    fn config_loads_notification_position_from_toml() {
        let config =
            super::Config::from_toml_with_defaults("notification_position = \"top-left\"\n")
                .unwrap();
        assert_eq!(config.notification_position, NotificationPosition::TopLeft);
        let config = super::Config::from_toml_with_defaults("").unwrap();
        assert_eq!(
            config.notification_position,
            NotificationPosition::BottomRight
        );
    }

    #[test]
    fn test_get_command_shortcut() {
        let config = super::Config::new();
        assert_eq!(config.get_command_shortcut("Save"), Some("CTRL+S".to_string()));
        assert_eq!(config.get_command_shortcut("Close Tab"), Some("CTRL+W".to_string()));
        assert_eq!(config.get_command_shortcut("New File"), Some("CTRL+N".to_string()));
        assert_eq!(config.get_command_shortcut("Open File"), Some("CTRL+O".to_string()));
        assert_eq!(config.get_command_shortcut("File Name Search"), Some("CTRL+O".to_string()));
        assert_eq!(config.get_command_shortcut("Toggle Explorer"), Some("CTRL+B".to_string()));
        assert_eq!(config.get_command_shortcut("Global Search"), Some("CTRL+G".to_string()));
        assert_eq!(config.get_command_shortcut("Content Search"), Some("CTRL+G".to_string()));
        assert_eq!(config.get_command_shortcut("Local Search"), Some("CTRL+F".to_string()));
        assert_eq!(config.get_command_shortcut("Command Palette"), Some("CTRL+P".to_string()));
        assert_eq!(config.get_command_shortcut("Nonexistent Command"), None);
    }

    #[test]
    fn notification_position_cycles_cleanly() {
        use NotificationPosition::*;
        assert_eq!(BottomRight.next(), BottomLeft);
        assert_eq!(BottomLeft.next(), TopLeft);
        assert_eq!(TopLeft.next(), TopRight);
        assert_eq!(TopRight.next(), BottomRight);
    }

    #[test]
    fn config_saves_and_loads_roundtrip() {
        let mut config = super::Config::new();
        config.autocomplete_enabled = false;
        config.preview_max_size = 10 * 1024 * 1024;
        config.notification_position = NotificationPosition::TopLeft;

        let toml_str = toml::to_string_pretty(&config).unwrap();
        let loaded = super::Config::from_toml_with_defaults(&toml_str).unwrap();

        assert_eq!(loaded.autocomplete_enabled, false);
        assert_eq!(loaded.preview_max_size, 10 * 1024 * 1024);
        assert_eq!(loaded.notification_position, NotificationPosition::TopLeft);
    }
}
