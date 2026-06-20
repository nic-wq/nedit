use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::{App, FuzzyMode};

impl App {
    pub(crate) fn format_search_dir_for_query(&self, path: &Path, prefer_home: bool) -> String {
        if prefer_home {
            if let Ok(home) = std::env::var("HOME") {
                let home_path = PathBuf::from(&home);
                if path == home_path {
                    return "~".to_string();
                }
                if let Ok(relative) = path.strip_prefix(&home_path) {
                    let relative = relative.to_string_lossy();
                    if relative.is_empty() {
                        return "~".to_string();
                    }
                    return format!("~/{}", relative);
                }
            }
        }

        if let Ok(relative) = path.strip_prefix(&self.explorer.root) {
            let relative = relative.to_string_lossy();
            if relative.is_empty() {
                ".".to_string()
            } else {
                relative.to_string()
            }
        } else {
            path.to_string_lossy().to_string()
        }
    }

    fn expand_search_dir(&self, dir: &str) -> PathBuf {
        if dir == "~" {
            return std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| self.explorer.root.clone());
        }

        if let Some(suffix) = dir.strip_prefix("~/") {
            return std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| self.explorer.root.clone())
                .join(suffix);
        }

        let candidate = Path::new(dir);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.explorer.root.join(candidate)
        }
    }

    fn scoped_dir_suggestions(&self) -> Option<Vec<PathBuf>> {
        let trimmed = self.fuzzy_query.trim();
        let scoped_query = trimmed.strip_prefix('@')?;
        if scoped_query.chars().any(char::is_whitespace) {
            return None;
        }

        let (base_dir, needle, prefer_home) = if scoped_query.is_empty() {
            (self.explorer.root.clone(), String::new(), false)
        } else if scoped_query.ends_with('/') {
            (self.expand_search_dir(scoped_query), String::new(), scoped_query.starts_with('~'))
        } else if let Some((parent, fragment)) = scoped_query.rsplit_once('/') {
            (
                self.expand_search_dir(parent),
                fragment.to_lowercase(),
                scoped_query.starts_with('~'),
            )
        } else {
            (
                self.explorer.root.clone(),
                scoped_query.to_lowercase(),
                scoped_query.starts_with('~'),
            )
        };

        let Ok(entries) = fs::read_dir(&base_dir) else {
            return Some(Vec::new());
        };

        let mut suggestions: Vec<PathBuf> = entries
            .flatten()
            .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                needle.is_empty() || name.starts_with(&needle)
            })
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| !Self::should_skip_dir_name(name))
                    .unwrap_or(true)
            })
            .collect();

        suggestions.sort_by_key(|path| self.format_search_dir_for_query(path, prefer_home));
        suggestions.truncate(self.fuzzy_limit);
        Some(suggestions)
    }

    fn resolve_file_search_scope(&self) -> Option<(String, PathBuf)> {
        let trimmed = self.fuzzy_query.trim();
        let scoped_query = trimmed.strip_prefix('@')?;

        let mut parts = scoped_query.splitn(2, char::is_whitespace);
        let dir_part = parts
            .next()
            .map(str::trim)
            .filter(|part| !part.is_empty())?;
        let search_term = parts.next().map(str::trim)?;

        let search_root = self.expand_search_dir(dir_part);
        search_root
            .is_dir()
            .then(|| (search_term.to_lowercase(), search_root))
    }

    fn resolve_content_search_scope(&self) -> (String, PathBuf, bool) {
        let trimmed = self.fuzzy_query.trim();
        let Some(scoped_query) = trimmed.strip_prefix('@') else {
            return (trimmed.to_lowercase(), self.explorer.root.clone(), false);
        };

        let mut parts = scoped_query.splitn(2, char::is_whitespace);
        let Some(dir_part) = parts.next().map(str::trim).filter(|part| !part.is_empty()) else {
            return (trimmed.to_lowercase(), self.explorer.root.clone(), false);
        };
        let Some(search_term) = parts.next().map(str::trim) else {
            return (trimmed.to_lowercase(), self.explorer.root.clone(), false);
        };

        let search_root = self.expand_search_dir(dir_part);

        if search_root.is_dir() {
            (search_term.to_lowercase(), search_root, true)
        } else {
            (trimmed.to_lowercase(), self.explorer.root.clone(), false)
        }
    }

    fn scoped_search_files(root: PathBuf) -> impl Iterator<Item = PathBuf> {
        let root_for_filter = root.clone();
        WalkDir::new(root)
            .into_iter()
            .filter_entry(move |entry| {
                let path = entry.path();
                if path == root_for_filter {
                    return true;
                }
                let name = entry.file_name().to_string_lossy();
                !Self::should_skip_dir_name(name.as_ref())
            })
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.path().to_path_buf())
    }

    fn search_content_in_files(
        files: impl IntoIterator<Item = PathBuf>,
        query: &str,
        limit: usize,
    ) -> Vec<(PathBuf, usize, String)> {
        let mut results = Vec::new();
        for path in files {
            if let Ok(content) = fs::read_to_string(&path) {
                for (i, line) in content.lines().enumerate() {
                    if line.to_lowercase().contains(query) {
                        results.push((path.clone(), i, line.to_string()));
                        if results.len() >= limit {
                            return results;
                        }
                    }
                }
            }
        }
        results
    }

    fn fuzzy_file_name_matches(path: &Path, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
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
    }

    pub fn toggle_fuzzy(&mut self, mode: FuzzyMode) {
        if self.is_fuzzy && self.fuzzy_mode == mode {
            self.is_fuzzy = false;
        } else {
            self.is_fuzzy = true;
            self.fuzzy_mode = mode;
            self.fuzzy_query.clear();
            self.fuzzy_limit = 20;

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
                self.ensure_all_themes_loaded();
            }

            if matches!(mode, FuzzyMode::Files | FuzzyMode::Content) {
                self.ensure_all_files_collected();
            }
            self.update_fuzzy(true);
        }
    }

    // We slugify names to ensure they are safe for the filesystem across different OSs
    // while maintaining a readable and consistent naming convention for user-generated scripts.
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

    pub(crate) fn invalidate_file_index(&mut self) {
        self.all_files = std::sync::Arc::new(Vec::new());
        self.all_files_ready = false;
    }

    pub fn poll_background_tasks(&mut self) {
        if let Some(rx) = &self.syntax_set_receiver {
            let mut disconnected = false;
            loop {
                match rx.try_recv() {
                    Ok(syntax_set) => {
                        self.syntax_set = Some(syntax_set);
                        self.needs_redraw = true;
                        for buf in self.buffers.iter_mut() {
                            buf.invalidate_all_rendered_spans();
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
            if disconnected {
                self.syntax_set_receiver = None;
            }
        }

        if let Some(rx) = &self.indexed_files_receiver {
            if let Ok(files) = rx.try_recv() {
                self.all_files = std::sync::Arc::new(files);
                self.all_files_ready = true;
                self.indexed_files_receiver = None;
                self.update_fuzzy(true);
                self.needs_redraw = true;
            }
        }
        if let Some(rx) = &self.explorer_refresh_receiver {
            if let Ok((items, max_width)) = rx.try_recv() {
                self.explorer.items = items;
                self.explorer.max_item_width = max_width;
                self.explorer_refresh_receiver = None;

                // Restore selection
                if let Some(path) = self.pending_explorer_selection.take() {
                    if let Some(idx) = self.explorer.items.iter().position(|i| i.path == path) {
                        self.explorer.selected_idx = idx;
                    }
                }

                // If a new refresh was requested while the previous one was running, trigger it now
                if self.explorer_needs_refresh {
                    self.explorer_needs_refresh = false;
                    self.refresh_explorer();
                }
                self.needs_redraw = true;
            }
        }
        if let Some(rx) = &self.content_search_receiver {
            let mut message_received = false;
            let mut latest_results = None;
            while let Ok((query, results)) = rx.try_recv() {
                message_received = true;
                if query == self.fuzzy_query.to_lowercase() {
                    latest_results = Some(results);
                }
            }
            if message_received {
                self.content_search_receiver = None;
                if let Some(results) = latest_results {
                    self.fuzzy_global_results = results;
                } else {
                    // The query changed while we were searching.
                    // Trigger a new search for the current query.
                    self.update_fuzzy(true);
                }
                self.needs_redraw = true;
            }
        }
    }

    pub fn ensure_all_files_collected(&mut self) {
        if self.all_files_ready || self.indexed_files_receiver.is_some() {
            return;
        }

        let (tx, rx) = std::sync::mpsc::channel();
        self.indexed_files_receiver = Some(rx);

        let root = self.explorer.root.clone();
        let explorer_root = self.explorer.root.clone();

        std::thread::spawn(move || {
            let files: Vec<PathBuf> = WalkDir::new(&root)
                .into_iter()
                .filter_entry(|e| {
                    let path = e.path();
                    if path == explorer_root {
                        return true;
                    }
                    let name = e.file_name().to_string_lossy();
                    !Self::should_skip_dir_name(name.as_ref())
                })
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.path().to_path_buf())
                .collect();
            let _ = tx.send(files);
        });
    }

    pub fn load_more_fuzzy(&mut self) {
        self.fuzzy_limit += 50;
        self.update_fuzzy(false);
    }

    pub fn update_fuzzy(&mut self, reset_idx: bool) {
        let query = self.fuzzy_query.to_lowercase();
        if query.is_empty() && self.fuzzy_mode == FuzzyMode::Content {
            self.fuzzy_results = Vec::new();
            if reset_idx {
                self.fuzzy_idx = 0;
            }
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
                "New Lua Script",
                "Run Lua Script",
                "Edit Lua Script",
                "Delete Lua Script",
                "Open Live Script",
                "Undo Last Script",
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
            if reset_idx {
                self.fuzzy_idx = 0;
            }
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
            self.fuzzy_global_results = Vec::new();
            if self.fuzzy_mode == FuzzyMode::Files {
                if let Some(suggestions) = self.scoped_dir_suggestions() {
                    self.fuzzy_results = suggestions;
                    if reset_idx {
                        self.fuzzy_idx = 0;
                    }
                    return;
                }

                if let Some((query, search_root)) = self.resolve_file_search_scope() {
                    self.fuzzy_results = Self::scoped_search_files(search_root)
                        .filter(|path| Self::fuzzy_file_name_matches(path, &query))
                        .take(self.fuzzy_limit)
                        .collect();
                } else {
                    self.fuzzy_results = self
                        .all_files
                        .iter()
                        .filter(|p| Self::fuzzy_file_name_matches(p, &query))
                        .cloned()
                        .take(self.fuzzy_limit)
                        .collect();
                }
            } else if self.fuzzy_mode == FuzzyMode::Content {
                self.fuzzy_results = self.scoped_dir_suggestions().unwrap_or_default();
                if !self.fuzzy_results.is_empty() {
                    if reset_idx {
                        self.fuzzy_idx = 0;
                    }
                    return;
                }
                self.fuzzy_results = Vec::new();
                // Background search with debouncing logic would be better,
                // but let's at least avoid blocking the main thread.
                // To avoid thread storm, we check if a search is already in progress.
                if self.content_search_receiver.is_none() {
                    let (query_for_search, search_root, scoped_search) =
                        self.resolve_content_search_scope();
                    if query_for_search.is_empty() {
                        if reset_idx {
                            self.fuzzy_idx = 0;
                        }
                        return;
                    }
                    let files = self.all_files.clone();
                    let query_for_thread = self.fuzzy_query.to_lowercase();

                    let (tx, rx) = std::sync::mpsc::channel();
                    self.content_search_receiver = Some(rx);
                    let limit = self.fuzzy_limit;
                    std::thread::spawn(move || {
                        let results = if scoped_search {
                            Self::search_content_in_files(
                                Self::scoped_search_files(search_root),
                                &query_for_search,
                                limit,
                            )
                        } else {
                            Self::search_content_in_files(
                                files.iter().cloned(),
                                &query_for_search,
                                limit,
                            )
                        };
                        let _ = tx.send((query_for_thread, results));
                    });
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
                    if self.fuzzy_lines.len() >= self.fuzzy_limit {
                        break;
                    }
                }
            }
        }

        if self.fuzzy_mode == FuzzyMode::Themes {
            self.ensure_all_themes_loaded();
            let mut seen = std::collections::HashSet::new();
            let mut canonical_order = Vec::new();
            let mut keys: Vec<&String> = self.theme_set.themes.keys().collect();
            keys.sort();
            for key in keys {
                if let Some(theme) = self.theme_set.themes.get(key) {
                    let canonical = theme.name.as_deref().unwrap_or(key.as_str());
                    if seen.insert(canonical.to_string()) {
                        canonical_order.push(canonical.to_string());
                    }
                }
            }
            self.fuzzy_themes = canonical_order
                .into_iter()
                .filter(|t| query.is_empty() || t.to_lowercase().contains(&query))
                .collect();
        }

        if reset_idx {
            self.fuzzy_idx = 0;
        } else if self.fuzzy_idx >= self.fuzzy_themes.len() {
            self.fuzzy_idx = self.fuzzy_themes.len().saturating_sub(1);
        }

        if self.fuzzy_mode == FuzzyMode::Themes && !self.fuzzy_themes.is_empty() {
            if self.fuzzy_idx < self.fuzzy_themes.len() {
                self.current_theme = self.fuzzy_themes[self.fuzzy_idx].clone();
            }
        }
    }
}
