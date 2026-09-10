use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use walkdir::WalkDir;

use super::matcher::{FuzzyFileResult, FuzzyMatcher};
use super::{App, FuzzyMode, NotificationType};

// Binary file extensions to skip during indexing and content search
const BINARY_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".ico", ".svg",
    ".zip", ".tar", ".gz", ".bz2", ".xz", ".7z", ".rar",
    ".o", ".so", ".a", ".dylib", ".dll", ".exe", ".bin",
    ".pdf", ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx",
    ".mp3", ".mp4", ".wav", ".avi", ".mov", ".mkv", ".flac",
    ".woff", ".woff2", ".ttf", ".otf", ".eot",
];

// Skip binary files and files > 1MB for content search
fn is_searchable_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        let ext_with_dot = format!(".{}", ext_lower);
        if BINARY_EXTENSIONS.contains(&ext_with_dot.as_str()) {
            return false;
        }
    }
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > 1_048_576 {
            return false;
        }
    }
    true
}

// Case-insensitive substring search without allocation (fast path for ASCII)
fn contains_ascii_insensitive(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if !haystack.is_ascii() || !needle.is_ascii() {
        return haystack.to_lowercase().contains(&needle.to_lowercase());
    }
    let needle = needle.as_bytes();
    let haystack = haystack.as_bytes();
    let mut hi = 0;
    while hi + needle.len() <= haystack.len() {
        if needle
            .iter()
            .zip(&haystack[hi..])
            .all(|(&n, &h)| n == h.to_ascii_lowercase())
        {
            return true;
        }
        hi += 1;
    }
    false
}

/// Probe the filesystem directly for files matching a path-like query.
///
/// This provides **instant** results without waiting for the full file index.
/// If the query looks like a path (contains `/` or could be a partial path),
/// we do a `read_dir()` on the parent directory and filter by basename prefix.
///
/// Returns `(relative_path, full_path)` pairs.
fn probe_filesystem(root: &Path, query: &str) -> Vec<(String, PathBuf)> {
    if query.is_empty() || query.starts_with('@') {
        return Vec::new();
    }

    // Normalise — strip trailing slashes for predictable parent logic.
    let normalised = query.trim_end_matches('/');
    if normalised.is_empty() {
        return Vec::new();
    }

    let abs = root.join(normalised);

    // Case 1: the query points to an existing directory → list its files.
    if let Ok(entries) = fs::read_dir(&abs) {
        return entries
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter(|e| {
                let fname = e.file_name();
                let name = fname.to_string_lossy();
                !name.starts_with('.')
            })
            .take(30)
            .map(|e| {
                let full = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                let rel = format!("{}/{}", normalised, name);
                (rel, full)
            })
            .collect();
    }

    // Case 2: the query is a partial path — find parent, filter by prefix.
    let parent = match abs.parent() {
        Some(p) => p,
        None => return Vec::new(),
    };
    let basename = match abs.file_name().and_then(|n| n.to_str()) {
        Some(b) => b,
        None => return Vec::new(),
    };
    let rel_parent = match parent.strip_prefix(root) {
        Ok(p) => p.to_string_lossy().replace('\\', "/"),
        Err(_) => return Vec::new(), // parent is outside the project root
    };

    let lower_needle = basename.to_lowercase();
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter(|e| {
            let fname = e.file_name();
            let name = fname.to_string_lossy();
            !name.starts_with('.') && name.to_lowercase().starts_with(&lower_needle)
        })
        .take(30)
        .map(|e| {
            let full = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            let rel = if rel_parent.is_empty() {
                name
            } else {
                format!("{}/{}", rel_parent, name)
            };
            (rel, full)
        })
        .collect()
}

/// All documentation options as (path, search alias) pairs.
/// The alias keeps short queries like "lua" or "binds" matching even though
/// the file names are longer.
fn doc_options() -> Vec<(&'static str, &'static str)> {
    vec![
        ("docs.md", "docs general help"),
        ("docs/lua.md", "lua scripts api"),
        ("docs/binds.md", "binds keybindings shortcuts keys"),
    ]
}

/// Filter documentation options by query (case-insensitive substring over
/// path and alias). Empty query returns everything.
fn filter_doc_options(query: &str) -> Vec<PathBuf> {
    let query = query.trim().to_lowercase();
    doc_options()
        .into_iter()
        .filter(|(path, alias)| {
            query.is_empty() || path.to_lowercase().contains(&query) || alias.contains(&query)
        })
        .map(|(path, _)| PathBuf::from(path))
        .collect()
}

impl App {
    fn cancel_pending_file_search(&mut self) {
        if let Some(cancel) = &self.fuzzy_files_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        self.fuzzy_files_cancel = None;
        self.fuzzy_files_receiver = None;
    }

    fn sync_visible_file_results(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.fuzzy_file_results
            .sort_by(|a, b| {
                b.score.cmp(&a.score)
                    .then_with(|| a.relative_path.matches('/').count().cmp(&b.relative_path.matches('/').count()))
            });
        self.fuzzy_file_results
            .retain(|result| seen.insert(result.full_path.clone()));
        self.fuzzy_file_results.truncate(self.fuzzy_limit);
        self.fuzzy_results = self
            .fuzzy_file_results
            .iter()
            .map(|r| r.full_path.clone())
            .collect();
        self.fuzzy_idx = self
            .fuzzy_idx
            .min(self.fuzzy_results.len().saturating_sub(1));
    }

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
                    .map(|name| !crate::explorer::should_skip_dir_name(name))
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
                !crate::explorer::should_skip_dir_name(name.as_ref())
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
            if results.len() >= limit {
                break;
            }
            // Skip binary files and files > 1MB
            if !is_searchable_file(&path) {
                continue;
            }
            // Read line-by-line with BufReader to avoid loading entire files
            // and enable early exit at the first match per file
            if let Ok(file) = std::fs::File::open(&path) {
                let reader = std::io::BufReader::new(file);
                for (i, line_result) in reader.lines().enumerate() {
                    if results.len() >= limit {
                        return results;
                    }
                    if let Ok(line) = line_result {
                        if contains_ascii_insensitive(&line, query) {
                            results.push((path.clone(), i, line));
                            break; // One match per file is enough
                        }
                    }
                }
            }
        }
        results
    }

    // Debounce: schedule fuzzy update, executes after 80ms of typing pause
    pub fn schedule_fuzzy_update(&mut self, reset_idx: bool) {
        self.fuzzy_input_timestamp = Some(std::time::Instant::now());
        self.fuzzy_input_reset_idx = reset_idx;
    }

    pub fn toggle_fuzzy(&mut self, mode: FuzzyMode) {
        if self.is_fuzzy && self.fuzzy_mode == mode {
            self.is_fuzzy = false;
        } else {
            if mode == FuzzyMode::SaveAs
                && self.live_script_mode
                && Some(self.current_buffer_idx) == self.live_script_buffer_idx
            {
                self.show_notification(
                    "Live scripts cannot be saved".to_string(),
                    NotificationType::Info,
                );
                return;
            }

            self.is_fuzzy = true;
            self.fuzzy_mode = mode;
            self.clear_fuzzy_query();
            self.fuzzy_limit = 20;

            if mode == FuzzyMode::SaveAs && !self.buffers.is_empty() {
                let buf = &self.buffers[self.current_buffer_idx];
                if buf.content.len_lines() > 0 {
                    let first_line = buf.content.line(0).to_string();
                    let trimmed = first_line.trim();
                    if let Some(name) = trimmed.strip_prefix("-- Name: ") {
                        self.set_fuzzy_query(self.slugify(name.trim()));
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
        self.fuzzy_file_results = Vec::new();
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
            if let Ok((items, max_width, corpus)) = rx.try_recv() {
                self.explorer.items = items;
                self.explorer.max_item_width = max_width;
                self.explorer.search_corpus = corpus;
                self.explorer_refresh_receiver = None;

                // Restore selection
                if let Some(path) = self.pending_explorer_selection.take() {
                    if let Some(idx) = self.explorer.items.iter().position(|i| i.path == path) {
                        self.explorer.selected_idx = idx;
                    }
                }

                // A fresh corpus can change the filtered view: re-filter,
                // keeping the selected path when it is still present.
                if self.explorer.is_searching() {
                    self.explorer.update_search_results(false);
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
            while let Ok((query, seq, results)) = rx.try_recv() {
                message_received = true;
                // Ignore stale results from superseded searches
                if seq >= self.content_search_seq
                    && query == self.fuzzy_query.to_lowercase()
                {
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

        // Collect incremental batches from background file finder (Ctrl+O).
        // The thread sends multiple batches during iteration and the sender drops
        // when it exits — we accumulate, sort, and truncate on each batch.
        if let Some(rx) = &self.fuzzy_files_receiver {
            use std::sync::mpsc::TryRecvError;
            let mut got_batch = false;
            loop {
                match rx.try_recv() {
                    Ok(batch) => {
                        got_batch = true;
                        for (rel, full, score, positions) in batch {
                            self.fuzzy_file_results.push(FuzzyFileResult {
                                relative_path: rel,
                                full_path: full,
                                score,
                                match_positions: positions,
                            });
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        // Thread finished: stop polling this channel.
                        self.fuzzy_files_receiver = None;
                        break;
                    }
                }
            }
            if got_batch {
                self.sync_visible_file_results();
                self.needs_redraw = true;
            }
        }

        // Debounce: run update_fuzzy only after typing pauses for 80ms
        if let Some(ts) = self.fuzzy_input_timestamp {
            if ts.elapsed() >= std::time::Duration::from_millis(80) {
                let reset = self.fuzzy_input_reset_idx;
                self.fuzzy_input_timestamp = None;
                self.fuzzy_input_reset_idx = false;
                self.update_fuzzy(reset);
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
            let files: Vec<(String, PathBuf)> = WalkDir::new(&root)
                .into_iter()
                .filter_entry(|e| {
                    let path = e.path();
                    if path == explorer_root {
                        return true;
                    }
                    let name = e.file_name().to_string_lossy();
                    !crate::explorer::should_skip_dir_name(name.as_ref())
                })
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| {
                    let full = e.path().to_path_buf();
                    let relative = full
                        .strip_prefix(&root)
                        .unwrap_or(&full)
                        .to_string_lossy()
                        .to_string();
                    (relative, full)
                })
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

        if self.fuzzy_mode == FuzzyMode::FileOptions {
            // Build full option list (depends on whether a directory is selected)
            // and filter by fuzzy query so the menu is searchable.
            let is_dir = self
                .explorer
                .get_selected()
                .map(|i| i.is_dir)
                .unwrap_or(false);
            let mut all = vec![
                "New File",
                "New Folder",
                "Rename",
                "Move",
                "Delete",
            ];
            if is_dir {
                all.push("Set as Root");
            }
            self.fuzzy_results = all
                .into_iter()
                .filter(|c| query.is_empty() || c.to_lowercase().contains(&query))
                .map(PathBuf::from)
                .collect();
            if reset_idx {
                self.fuzzy_idx = 0;
            } else {
                self.fuzzy_idx = self.fuzzy_idx.min(self.fuzzy_results.len().saturating_sub(1));
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
            self.fuzzy_results = filter_doc_options(&query);
            if reset_idx {
                self.fuzzy_idx = 0;
            } else {
                self.fuzzy_idx = self
                    .fuzzy_idx
                    .min(self.fuzzy_results.len().saturating_sub(1));
            }
            return;
        }

        if self.fuzzy_mode == FuzzyMode::Create
            || self.fuzzy_mode == FuzzyMode::ExternalChange
            || self.fuzzy_mode == FuzzyMode::UnsavedChanges
        {
            self.fuzzy_results = Vec::new();
            return;
        }

        if self.fuzzy_mode == FuzzyMode::Files || self.fuzzy_mode == FuzzyMode::Content {
            self.fuzzy_global_results = Vec::new();
            if self.fuzzy_mode == FuzzyMode::Files {
                if let Some(suggestions) = self.scoped_dir_suggestions() {
                    self.cancel_pending_file_search();
                    self.fuzzy_results = suggestions;
                    self.fuzzy_file_results = Vec::new();
                    if reset_idx {
                        self.fuzzy_idx = 0;
                    }
                    return;
                }

                if query.is_empty() {
                    self.cancel_pending_file_search();
                    self.fuzzy_results = Vec::new();
                    self.fuzzy_file_results = Vec::new();
                    if reset_idx {
                        self.fuzzy_idx = 0;
                    }
                    return;
                }

                // Cancel the previous worker before replacing the visible state.
                self.cancel_pending_file_search();
                self.fuzzy_file_results.clear();
                self.fuzzy_results.clear();

                // ------------------------------------------------------------------
                // 1. Prefix probing: instant results via direct filesystem access.
                // ------------------------------------------------------------------
                let probe_hits = probe_filesystem(&self.explorer.root, &query);
                if !probe_hits.is_empty() {
                    let mut m = FuzzyMatcher::new(&query);
                    let scored: Vec<FuzzyFileResult> = probe_hits
                        .iter()
                        .filter_map(|(rel, full)| {
                            let fm = m.match_target(rel);
                            if fm.matched {
                                Some(FuzzyFileResult {
                                    relative_path: rel.clone(),
                                    full_path: full.clone(),
                                    score: fm.score,
                                    match_positions: fm.match_positions,
                                })
                            } else {
                                None
                            }
                        })
                        .collect();
                    self.fuzzy_file_results = scored;
                    self.sync_visible_file_results();
                    if reset_idx {
                        self.fuzzy_idx = 0;
                    }
                    self.needs_redraw = true;
                } else {
                    self.needs_redraw = true;
                }

                // ------------------------------------------------------------------
                // 2. Cancel any previous search so stale results don't arrive later.
                // ------------------------------------------------------------------
                self.fuzzy_files_seq += 1;
                let cancel = Arc::new(AtomicBool::new(false));
                self.fuzzy_files_cancel = Some(cancel.clone());

                // ------------------------------------------------------------------
                // 3. Start a new background search that sends incremental batches.
                // ------------------------------------------------------------------
                let (tx, rx) = std::sync::mpsc::channel();
                self.fuzzy_files_receiver = Some(rx);
                let limit = self.fuzzy_limit;
                let query_for_thread = query.clone();

                if let Some((query_scope, search_root)) = self.resolve_file_search_scope() {
                    // Scoped search: walk + score in one thread, send batches.
                    std::thread::spawn(move || {
                        let mut batch: Vec<(String, PathBuf, i32, Vec<usize>)> = Vec::new();
                        let mut count = 0;
                        for full in Self::scoped_search_files(search_root.clone()) {
                            if cancel.load(Ordering::Relaxed) {
                                return;
                            }
                            let relative = full
                                .strip_prefix(&search_root)
                                .unwrap_or(&full)
                                .to_string_lossy()
                                .to_string();
                            let mut m = FuzzyMatcher::new(&query_scope);
                            let fm = m.match_target(&relative);
                            if fm.matched {
                                batch.push((relative, full, fm.score, fm.match_positions));
                                count += 1;
                            }
                            // Send batch every 15 results or at limit.
                            if batch.len() >= 15 || count >= limit {
                                let _ = tx.send(batch);
                                batch = Vec::new();
                            }
                            if count >= limit {
                                // Flush remaining
                                if !batch.is_empty() {
                                    let _ = tx.send(batch);
                                }
                                return;
                            }
                        }
                        // Flush final partial batch.
                        if !batch.is_empty() {
                            let _ = tx.send(batch);
                        }
                    });
                } else {
                    // Full search against the already-collected file index.
                    let files = self.all_files.clone();
                    std::thread::spawn(move || {
                        let mut m = FuzzyMatcher::new(&query_for_thread);
                        let mut batch: Vec<(String, PathBuf, i32, Vec<usize>)> = Vec::new();
                        for (rel, full) in files.iter() {
                            if cancel.load(Ordering::Relaxed) {
                                return;
                            }
                            let fm = m.match_target(rel);
                            if fm.matched {
                                batch.push((rel.clone(), full.clone(), fm.score, fm.match_positions));
                                if batch.len() >= 15 {
                                    let _ = tx.send(batch);
                                    batch = Vec::new();
                                }
                            }
                        }
                        if !batch.is_empty() {
                            let _ = tx.send(batch);
                        }
                    });
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
                    self.content_search_seq += 1;
                    let seq = self.content_search_seq;

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
                                files.iter().map(|(_, full)| full.clone()),
                                &query_for_search,
                                limit,
                            )
                        };
                        let _ = tx.send((query_for_thread, seq, results));
                    });
                }
            }
        }

        if self.fuzzy_mode == FuzzyMode::Local {
            if let Some(buffer) = self.buffers.get(self.current_buffer_idx) {
                self.fuzzy_lines = Vec::new();
                for i in 0..buffer.content.len_lines() {
                    let line_slice = buffer.content.line(i);
                    let matches = if query.is_empty() {
                        true
                    } else {
                        let mut chunks = line_slice.chunks();
                        if let Some(chunk) = chunks.next() {
                            if chunks.next().is_none() {
                                contains_ascii_insensitive(chunk, &query)
                            } else {
                                let line = line_slice.to_string();
                                contains_ascii_insensitive(&line, &query)
                            }
                        } else {
                            false
                        }
                    };
                    if matches {
                        self.fuzzy_lines.push((i, line_slice.to_string()));
                        if self.fuzzy_lines.len() >= self.fuzzy_limit {
                            break;
                        }
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

        if self.fuzzy_mode == FuzzyMode::Themes && !self.fuzzy_themes.is_empty()
            && self.fuzzy_idx < self.fuzzy_themes.len() {
                self.current_theme = self.fuzzy_themes[self.fuzzy_idx].clone();
            }
    }
}

#[cfg(test)]
mod tests {
    use super::filter_doc_options;
    use super::{App, FuzzyMode};
    use std::path::PathBuf;

    #[test]
    fn doc_select_lists_everything_on_empty_query() {
        assert_eq!(filter_doc_options("").len(), 3);
    }

    #[test]
    fn doc_select_filters_by_query() {
        assert_eq!(
            filter_doc_options("lua"),
            vec![PathBuf::from("docs/lua.md")]
        );
        assert_eq!(
            filter_doc_options("binds"),
            vec![PathBuf::from("docs/binds.md")]
        );
        // Alias terms match too.
        assert_eq!(
            filter_doc_options("keybindings"),
            vec![PathBuf::from("docs/binds.md")]
        );
        assert_eq!(
            filter_doc_options("LUA"),
            vec![PathBuf::from("docs/lua.md")]
        );
        assert!(filter_doc_options("no-such-doc").is_empty());
    }

    #[test]
    fn doc_select_modal_filters_as_you_type() {
        let mut app = App::new(&[]);
        app.toggle_fuzzy(FuzzyMode::DocSelect);
        assert_eq!(app.fuzzy_results.len(), 3);
        app.set_fuzzy_query("lua".to_string());
        app.update_fuzzy(true);
        assert_eq!(app.fuzzy_results, vec![PathBuf::from("docs/lua.md")]);
        app.set_fuzzy_query("xyz-no-match".to_string());
        app.update_fuzzy(true);
        assert!(app.fuzzy_results.is_empty());
    }
}
