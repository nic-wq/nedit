use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Config {
    #[serde(default = "default_true")]
    pub autocomplete_enabled: bool,
    #[serde(default)]
    pub keybinds: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn load() -> Self {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let config_path = home_dir.join(".config/nedit/config.toml");

        let mut config = Self::default();

        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(value) = toml::from_str::<toml::Value>(&content) {
                if let Some(table) = value.as_table() {
                    for (k, v) in table {
                        if k == "keybinds" {
                            if let Some(kb_table) = v.as_table() {
                                for (kb_k, kb_v) in kb_table {
                                    if let Some(s) = kb_v.as_str() {
                                        config.keybinds.insert(kb_k.clone(), s.to_string());
                                    }
                                }
                            }
                        } else if k == "autocomplete_enabled" {
                            if let Some(b) = v.as_bool() {
                                config.autocomplete_enabled = b;
                            }
                        } else if let Some(s) = v.as_str() {
                            config.keybinds.insert(k.clone(), s.to_string());
                        }
                    }
                }
                return config;
            }
        }

        config
    }

    pub fn default() -> Self {
        let mut keybinds = HashMap::new();
        keybinds.insert("quit".to_string(), "ctrl+q".to_string());
        keybinds.insert("new_file".to_string(), "ctrl+n".to_string());
        keybinds.insert("open_file".to_string(), "ctrl+b".to_string());
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

        Self {
            autocomplete_enabled: true,
            keybinds,
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
}
