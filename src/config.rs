use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crossterm::event::{KeyCode, KeyModifiers, KeyEvent};

#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_true")]
    pub autocomplete_enabled: bool,
    #[serde(default)]
    pub keybinds: HashMap<String, String>,
}

fn default_true() -> bool { true }

impl Config {
    pub fn load() -> Self {
        let home_dir = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
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
                            // Support top-level keybinds too
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
        keybinds.insert("live_script_next".to_string(), "shift+alt+right".to_string());
        keybinds.insert("live_script_prev".to_string(), "shift+alt+left".to_string());
        
        Self {
            autocomplete_enabled: true,
            keybinds,
        }
    }

    pub fn get_keybind(&self, action: &str) -> String {
        self.keybinds.get(action).cloned().unwrap_or_else(|| {
            Self::default().keybinds.get(action).cloned().unwrap_or_default()
        })
    }

    pub fn matches(&self, event: KeyEvent, action: &str) -> bool {
        let bind = self.get_keybind(action);
        if bind.is_empty() { return false; }
        
        let parts: Vec<&str> = bind.split('+').collect();
        let mut target_modifiers = KeyModifiers::NONE;
        let mut target_code = KeyCode::Null;

        for part in parts {
            match part.to_lowercase().as_str() {
                "ctrl" => target_modifiers |= KeyModifiers::CONTROL,
                "alt" => target_modifiers |= KeyModifiers::ALT,
                "shift" => target_modifiers |= KeyModifiers::SHIFT,
                "backtab" => target_code = KeyCode::BackTab,
                "tab" => target_code = KeyCode::Tab,
                "enter" => target_code = KeyCode::Enter,
                "esc" => target_code = KeyCode::Esc,
                "up" => target_code = KeyCode::Up,
                "down" => target_code = KeyCode::Down,
                "left" => target_code = KeyCode::Left,
                "right" => target_code = KeyCode::Right,
                "f1" => target_code = KeyCode::F(1),
                "f2" => target_code = KeyCode::F(2),
                "f3" => target_code = KeyCode::F(3),
                "f4" => target_code = KeyCode::F(4),
                "f5" => target_code = KeyCode::F(5),
                "f6" => target_code = KeyCode::F(6),
                "f7" => target_code = KeyCode::F(7),
                "f8" => target_code = KeyCode::F(8),
                "f9" => target_code = KeyCode::F(9),
                "f10" => target_code = KeyCode::F(10),
                "f11" => target_code = KeyCode::F(11),
                "f12" => target_code = KeyCode::F(12),
                c if c.len() == 1 => {
                    target_code = KeyCode::Char(c.chars().next().unwrap());
                }
                _ => {}
            }
        }

        // Special case for SHIFT + char
        if target_modifiers.contains(KeyModifiers::SHIFT) {
            if let KeyCode::Char(c) = target_code {
                if c.is_ascii_lowercase() {
                    // In crossterm, SHIFT+a is Char('A')
                }
            }
        }

        // Simplified matching (crossterm normalizes some things)
        let event_code = match event.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            c => c,
        };

        let target_code_normalized = match target_code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            c => c,
        };

        let important_modifiers = KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT;
        event_code == target_code_normalized && (event.modifiers & important_modifiers) == (target_modifiers & important_modifiers)
    }
}
