use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Default, Clone)]
pub struct IconConfig {
    #[serde(default)]
    pub files: HashMap<String, String>,
    #[serde(default)]
    pub extensions: HashMap<String, String>,
    #[serde(default)]
    pub commands: HashMap<String, String>,
}

pub struct IconRegistry {
    pub custom: IconConfig,
}

impl IconRegistry {
    pub fn load() -> Self {
        let mut custom = IconConfig::default();
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nedit")
            .join("icons");

        if let Ok(entries) = fs::read_dir(config_dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("toml") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(config) = toml::from_str::<IconConfig>(&content) {
                            custom.files.extend(config.files);
                            custom.extensions.extend(config.extensions);
                            custom.commands.extend(config.commands);
                        }
                    }
                }
            }
        }

        Self { custom }
    }

    pub fn get_icon(&self, path: &Path, is_dir: bool, expanded: bool) -> &str {
        if is_dir {
            if expanded {
                return "󰉖 ";
            } else {
                return "󰉋 ";
            }
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        // 1. Check custom files
        if let Some(icon) = self.custom.files.get(&filename) {
            return icon;
        }

        // 2. Check custom extensions
        if let Some(icon) = self.custom.extensions.get(&extension) {
            return icon;
        }

        // 3. Check default files
        match filename.as_str() {
            "dockerfile" => return "󰡨 ",
            "makefile" => return " ",
            "cargo.toml" => return " ",
            "package.json" => return " ",
            "license" => return "󰘥 ",
            "gitignore" => return " ",
            "docs.md" => return "󰘥 ",
            "lua.md" => return "󰢱 ",
            "binds.md" => return "󰘳 ",
            _ => {}
        }

        // 4. Check default extensions
        match extension.as_str() {
            "rs" => " ",
            "md" => " ",
            "py" => " ",
            "js" => " ",
            "ts" => " ",
            "jsx" => " ",
            "tsx" => " ",
            "html" => " ",
            "css" => " ",
            "json" => " ",
            "lua" => " ",
            "go" => " ",
            "c" => " ",
            "cpp" => " ",
            "h" => " ",
            "hpp" => " ",
            "sh" => " ",
            "bash" => " ",
            "zsh" => " ",
            "sql" => " ",
            "yaml" | "yml" => " ",
            "toml" => " ",
            "txt" => "󰈔 ",
            "png" | "jpg" | "jpeg" | "gif" | "svg" => "󰋩 ",
            "pdf" => "󰈦 ",
            "zip" | "tar" | "gz" | "7z" => "󰊄 ",
            "exe" | "bin" => "󰈐 ",
            _ => "󰈔 ",
        }
    }

    pub fn get_command_icon(&self, command: &str) -> &str {
        if let Some(icon) = self.custom.commands.get(command) {
            return icon;
        }

        match command {
            "Save" => "󰆓 ",
            "New File" => "󰝒 ",
            "Open File" => "󰈞 ",
            "Close Tab" => "󰅖 ",
            "Toggle Explorer" => "󰙅 ",
            "Global Search" => "󰈗 ",
            "Local Search" => "󰩊 ",
            "Switch Theme" => "󰔎 ",
            "Workspaces" => "󰉋 ",
            "Open Lua Script" => "󰢱 ",
            "Run Lua Script" => "󰐊 ",
            "Edit Lua Script" => "󰏫 ",
            "Delete Lua Script" => "󰆴 ",
            "Open Live Script" => "󰢱 ",
            "Undo Last Script" => "󰕌 ",
            "Quit" => "󰈆 ",
            "Undo" => "󰕌 ",
            "Redo" => "󰕍 ",
            "Copy" => "󰆏 ",
            "Paste" => "󰆑 ",
            "Cut" => "󰆐 ",
            "Select All" => "󰒅 ",
            "Open Help" => "󰘥 ",
            "Rename" => "󰏫 ",
            "Move" => "󰪹 ",
            "Delete" => "󰆴 ",
            "Exit Workspace" => "󰈆 ",
            "New Workspace..." => "󰉋 ",
            _ => "󰘳 ",
        }
    }
}
