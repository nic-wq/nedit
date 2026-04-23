use std::fs;
use std::path::PathBuf;

use walkdir::WalkDir;

use super::{App, FuzzyMode};

impl App {
    pub fn toggle_fuzzy(&mut self, mode: FuzzyMode) {
        if self.is_fuzzy && self.fuzzy_mode == mode {
            self.is_fuzzy = false;
        } else {
            self.is_fuzzy = true;
            self.fuzzy_mode = mode;
            self.fuzzy_query.clear();

            if mode == FuzzyMode::SaveAs && !self.buffers.is_empty() {
                let content = self.buffers[self.current_buffer_idx].content.to_string();
                if let Some(first_line) = content.lines().next() {
                    let trimmed = first_line.trim();
                    if trimmed.starts_with("-- Name: ") {
                        let name = trimmed[9..].trim();
                        self.fuzzy_query = self.slugify(name);
                    }
                }
            }

            if mode == FuzzyMode::Themes {
                self.original_theme = self.current_theme.clone();
            }

            if matches!(mode, FuzzyMode::Files | FuzzyMode::Content) {
                self.ensure_all_files_collected();
            }
            self.update_fuzzy();
        }
    }

    fn slugify(&self, name: &str) -> String {
        name.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .split('_')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("_")
    }

    fn should_skip_walk_entry(&self, entry: &walkdir::DirEntry) -> bool {
        let path = entry.path();

        if path == self.explorer.root {
            return false;
        }

        let name = entry.file_name().to_string_lossy();
        if matches!(name.as_ref(), "proc" | "sys" | "dev" | "run") {
            return true;
        }

        false
    }

    pub(crate) fn invalidate_file_index(&mut self) {
        self.all_files.clear();
        self.all_files_ready = false;
    }

    pub(crate) fn ensure_all_files_collected(&mut self) {
        if self.all_files_ready {
            return;
        }

        self.all_files = WalkDir::new(&self.explorer.root)
            .into_iter()
            .filter_entry(|e| !self.should_skip_walk_entry(e))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();
        self.all_files_ready = true;
    }

    pub fn update_fuzzy(&mut self) {
        let query = self.fuzzy_query.to_lowercase();
        if query.is_empty() && self.fuzzy_mode == FuzzyMode::Content {
            self.fuzzy_results = Vec::new();
            return;
        }

        if self.fuzzy_mode == FuzzyMode::CommandPalette {
            let commands = vec![
                "Save",
                "New File",
                "Open File",
                "Close Tab",
                "Toggle Explorer",
                "Global Search",
                "Local Search",
                "Switch Theme",
                "Workspaces",
                "New Lua Script",
                "Run Lua Script",
                "Edit Lua Script",
                "Delete Lua Script",
                "Open Live Script",
                "Quit",
                "Undo",
                "Redo",
                "Copy",
                "Paste",
                "Cut",
                "Select All",
                "Open Help",
            ];
            self.fuzzy_results = commands
                .into_iter()
                .filter(|c| query.is_empty() || c.to_lowercase().contains(&query))
                .map(PathBuf::from)
                .collect();
            self.fuzzy_idx = 0;
            return;
        }

        if self.fuzzy_mode == FuzzyMode::Workspaces {
            let options = self
                .workspaces
                .iter()
                .map(|w| w.name.clone())
                .chain(
                    self.current_workspace
                        .iter()
                        .map(|_| "Exit Workspace".to_string()),
                )
                .chain(std::iter::once("New Workspace...".to_string()));
            self.fuzzy_results = options
                .filter(|name| query.is_empty() || name.to_lowercase().contains(&query))
                .map(PathBuf::from)
                .collect();
            self.fuzzy_idx = 0;
            return;
        }

        if self.fuzzy_mode == FuzzyMode::RunScript {
            self.fuzzy_results = Vec::new();
            let home_dir = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."));
            let scripts_dir = home_dir.join(".config/nedit/scripts");
            let _ = fs::create_dir_all(&scripts_dir);
            if let Ok(entries) = fs::read_dir(&scripts_dir) {
                for entry in entries.flatten() {
                    if entry
                        .path()
                        .extension()
                        .map(|e| e == "lua")
                        .unwrap_or(false)
                    {
                        let stem = entry
                            .path()
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let name = if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Some(first_line) = content.lines().next() {
                                let trimmed = first_line.trim();
                                if trimmed.starts_with("-- ") {
                                    trimmed[3..].trim().to_string()
                                } else {
                                    stem.clone()
                                }
                            } else {
                                stem.clone()
                            }
                        } else {
                            stem.clone()
                        };
                        if query.is_empty() || name.to_lowercase().contains(&query) {
                            self.fuzzy_results.push(entry.path());
                        }
                    }
                }
            }
            return;
        }

        if self.fuzzy_mode == FuzzyMode::EditScript || self.fuzzy_mode == FuzzyMode::DeleteScript {
            self.fuzzy_results = Vec::new();
            let home_dir = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."));
            let scripts_dir = home_dir.join(".config/nedit/scripts");
            let _ = fs::create_dir_all(&scripts_dir);
            if let Ok(entries) = fs::read_dir(&scripts_dir) {
                for entry in entries.flatten() {
                    if entry
                        .path()
                        .extension()
                        .map(|e| e == "lua")
                        .unwrap_or(false)
                    {
                        let stem = entry
                            .path()
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let name = if let Ok(content) = fs::read_to_string(entry.path()) {
                            if let Some(first_line) = content.lines().next() {
                                let trimmed = first_line.trim();
                                if trimmed.starts_with("-- ") {
                                    trimmed[3..].trim().to_string()
                                } else {
                                    stem.clone()
                                }
                            } else {
                                stem.clone()
                            }
                        } else {
                            stem.clone()
                        };
                        if query.is_empty() || name.to_lowercase().contains(&query) {
                            self.fuzzy_results.push(entry.path());
                        }
                    }
                }
            }
            return;
        }

        if self.fuzzy_mode == FuzzyMode::Move {
            self.fuzzy_results = Vec::new();
            if let Some(dir) = &self.move_dir {
                if let Ok(entries) = fs::read_dir(dir) {
                    if dir.parent().is_some() {
                        self.fuzzy_results.push(PathBuf::from(".."));
                    }
                    for entry in entries.flatten() {
                        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if query.is_empty() || name.to_lowercase().contains(&query) {
                                self.fuzzy_results.push(entry.path());
                            }
                        }
                    }
                }
            }
            return;
        }

        if self.fuzzy_mode == FuzzyMode::DocSelect {
            self.fuzzy_results = vec![
                PathBuf::from("docs.md"),
                PathBuf::from("docs/lua.md"),
                PathBuf::from("docs/binds.md"),
            ];
            return;
        }

        if self.fuzzy_mode == FuzzyMode::NewFolder {
            self.fuzzy_results = Vec::new();
            return;
        }

        if self.fuzzy_mode == FuzzyMode::Files || self.fuzzy_mode == FuzzyMode::Content {
            self.fuzzy_results = Vec::new();
            self.fuzzy_global_results = Vec::new();
            if self.fuzzy_mode == FuzzyMode::Files {
                self.fuzzy_results = self
                    .all_files
                    .iter()
                    .filter(|p| {
                        let name = p
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase();
                        if query.is_empty() {
                            return true;
                        }
                        let mut it = query.chars();
                        let mut curr = it.next();
                        for c in name.chars() {
                            if let Some(target) = curr {
                                if c == target {
                                    curr = it.next();
                                }
                            }
                        }
                        curr.is_none()
                    })
                    .cloned()
                    .take(20)
                    .collect();
            } else if self.fuzzy_mode == FuzzyMode::Content {
                let mut count = 0;
                for path in &self.all_files {
                    if let Ok(content) = fs::read_to_string(path) {
                        for (i, line) in content.lines().enumerate() {
                            if line.to_lowercase().contains(&query) {
                                self.fuzzy_global_results
                                    .push((path.clone(), i, line.to_string()));
                                count += 1;
                                if count >= 20 {
                                    break;
                                }
                            }
                        }
                    }
                    if count >= 20 {
                        break;
                    }
                }
            }
        }

        if self.fuzzy_mode == FuzzyMode::Local {
            if let Some(buffer) = self.buffers.get(self.current_buffer_idx) {
                self.fuzzy_lines = Vec::new();
                for i in 0..buffer.content.len_lines() {
                    let line = buffer.content.line(i).to_string();
                    if query.is_empty() || line.to_lowercase().contains(&query) {
                        self.fuzzy_lines.push((i, line));
                    }
                    if self.fuzzy_lines.len() >= 20 {
                        break;
                    }
                }
            }
        }

        if self.fuzzy_mode == FuzzyMode::Themes {
            let themes: Vec<String> = self.theme_set.themes.keys().cloned().collect();
            self.fuzzy_themes = themes
                .into_iter()
                .filter(|t| query.is_empty() || t.to_lowercase().contains(&query))
                .collect();
        }

        self.fuzzy_idx = 0;

        if self.fuzzy_mode == FuzzyMode::Themes && !self.fuzzy_themes.is_empty() {
            self.current_theme = self.fuzzy_themes[0].clone();
        }
    }
}
