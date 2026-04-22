use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::Config;

impl Config {
    pub fn matches(&self, event: KeyEvent, action: &str) -> bool {
        let bind = self.get_keybind(action);
        if bind.is_empty() {
            return false;
        }

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

        let event_code = match event.code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            c => c,
        };

        let target_code_normalized = match target_code {
            KeyCode::Char(c) => KeyCode::Char(c.to_ascii_lowercase()),
            c => c,
        };

        let important_modifiers = KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT;
        event_code == target_code_normalized
            && (event.modifiers & important_modifiers) == (target_modifiers & important_modifiers)
    }
}
