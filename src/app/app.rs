use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use notify::{Config as NotifyConfig, RecommendedWatcher, Watcher};
use ratatui::layout::Rect;
use ropey::Rope;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::app::matcher::FuzzyFileResult;
use crate::buffer::EditorBuffer;
use crate::config::Config;
use crate::explorer::FileExplorer;
use crate::i18n::I18n;

use super::{Focus, NotificationType};

pub struct LargeFileLoadResult {
    pub request_id: u64,
    pub path: PathBuf,
    pub result: std::io::Result<Rope>,
}

// The App struct acts as the "Single Source of Truth" for the entire application state.
// By centralizing state here, we simplify data flow and make it easier to coordinate 
// between the UI, input handler, and background tasks.
pub struct App {
    pub buffers: Vec<EditorBuffer>,
    pub current_buffer_idx: usize,
    pub explorer: FileExplorer,
    pub focus: Focus,
    pub show_explorer: bool,
    pub should_quit: bool,
    pub syntax_set: Option<SyntaxSet>,
    pub theme_set: ThemeSet,
    pub themes_loaded: bool,
    pub is_welcome: bool,
    pub current_theme: String,
    pub is_fuzzy: bool,
    pub fuzzy_mode: crate::app::FuzzyMode,
    pub fuzzy_query: String,
    /// Cursor position inside `fuzzy_query` as a char index (0 = before first char).
    pub fuzzy_cursor: usize,
    /// Selection anchor as a char index; `None` means no active selection.
    pub fuzzy_sel_anchor: Option<usize>,
    pub fuzzy_results: Vec<PathBuf>,
    pub fuzzy_file_results: Vec<FuzzyFileResult>,
    pub fuzzy_lines: Vec<(usize, String)>,
    pub fuzzy_global_results: Vec<(PathBuf, usize, String)>,
    pub all_files: Arc<Vec<(String, PathBuf)>>,
    pub all_files_ready: bool,
    pub fuzzy_idx: usize,
    pub original_theme: String,
    pub fuzzy_themes: Vec<String>,
    pub config: Config,
    pub i18n: I18n,
    pub pending_path: Option<PathBuf>,
    pub pending_explorer_selection: Option<PathBuf>,
    pub move_dir: Option<PathBuf>,
    pub notifications: Vec<crate::app::toast::Toast>,
    pub live_script_mode: bool,
    pub live_script_buffer_idx: Option<usize>,
    pub target_buffer_idx: Option<usize>,
    pub watcher: Option<RecommendedWatcher>,
    pub fs_event_receiver: Receiver<notify::Result<notify::Event>>,
    pub syntax_set_receiver: Option<Receiver<SyntaxSet>>,
    pub(crate) large_file_load_sender: Sender<LargeFileLoadResult>,
    pub(crate) large_file_load_receiver: Receiver<LargeFileLoadResult>,
    pub(crate) next_large_file_load_id: u64,
    pub indexed_files_receiver: Option<Receiver<Vec<(String, PathBuf)>>>,
    #[allow(clippy::type_complexity)]
    pub explorer_refresh_receiver:
        Option<Receiver<(Vec<crate::explorer::FileItem>, usize, Vec<(PathBuf, bool)>)>>,
    pub explorer_needs_refresh: bool,
    #[allow(clippy::type_complexity)]
    pub content_search_receiver: Option<Receiver<(String, u64, Vec<(PathBuf, usize, String)>)>>,
    pub content_search_seq: u64,
    #[allow(clippy::type_complexity)]
    pub fuzzy_files_receiver: Option<Receiver<Vec<(String, PathBuf, i32, Vec<usize>)>>>,
    pub fuzzy_files_seq: u64,
    pub fuzzy_files_cancel: Option<Arc<AtomicBool>>,
    pub fuzzy_input_timestamp: Option<Instant>,
    pub fuzzy_input_reset_idx: bool,
    pub explorer_area: Rect,
    pub editor_area: Rect,
    /// Full terminal area of the last rendered frame, for overlay hit-testing.
    pub screen_area: Rect,
    pub fuzzy_limit: usize,
    pub last_script_undo: Option<crate::lua::ScriptUndo>,
    pub last_click_time: std::time::Instant,
    pub last_click_pos: (u16, u16),
    pub icon_registry: crate::ui::icons::IconRegistry,
    pub pending_action: Option<crate::app::types::PendingAction>,
    pub pending_buffer_idx: Option<usize>,
    pub needs_redraw: bool,
    pub preview_buffer_idx: Option<usize>,
    pub saved_buffer_idx: usize,
    pub file_mtimes: HashMap<PathBuf, SystemTime>,
    pub last_external_check: Instant,
}

impl App {
    fn push_toast(&mut self, toast: crate::app::toast::Toast) {
        // Cap the stack so a burst of messages never covers the screen.
        if self.notifications.len() >= crate::app::toast::MAX_TOASTS {
            self.notifications.remove(0);
        }
        self.notifications.push(toast);
        self.needs_redraw = true;
    }

    pub fn show_notification(&mut self, message: String, ntype: NotificationType) {
        self.push_toast(crate::app::toast::Toast::new(message, ntype));
    }

    pub fn show_notification_with_duration(
        &mut self,
        message: String,
        ntype: NotificationType,
        duration: std::time::Duration,
    ) {
        self.push_toast(crate::app::toast::Toast::with_duration(message, ntype, duration));
    }

    pub fn clear_notification(&mut self) {
        self.notifications.clear();
    }

    pub fn tick_notification(&mut self) {
        self.notifications.retain(|toast| !toast.is_expired());
    }

    /// Dismiss the toast whose close button sits at (`col`, `row`).
    /// Returns true when a toast was dismissed (the click is consumed).
    pub fn dismiss_toast_at(&mut self, col: u16, row: u16) -> bool {
        let area = self.screen_area;
        if area.width == 0 || area.height == 0 {
            return false;
        }
        let hit = crate::app::toast::toast_close_hit(
            &self.notifications,
            area,
            self.config.notification_position,
            col,
            row,
        );
        if let Some(idx) = hit {
            if idx < self.notifications.len() {
                self.notifications.remove(idx);
                self.needs_redraw = true;
                return true;
            }
        }
        false
    }

    pub fn request_redraw(&mut self) {
        self.needs_redraw = true;
    }

    pub fn new(args: &[String]) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let config_dir = Self::config_dir();
        let _ = fs::create_dir_all(&config_dir);
        let _ = fs::create_dir_all(config_dir.join("syntax"));
        let _ = fs::create_dir_all(config_dir.join("themes"));

        let config = Config::load();
        let theme_file = config_dir.join("theme.txt");
        let mut current_theme = config.theme.clone();

        if let Ok(saved_theme) = fs::read_to_string(&theme_file) {
            let saved_theme = saved_theme.trim();
            if !saved_theme.is_empty() {
                current_theme = saved_theme.to_string();
            }
        }

        let mut theme_set = ThemeSet::new();
        if let Some(theme) = Self::load_embedded_theme() {
            theme_set.themes.insert("NEdit Dark Complete".to_string(), theme.clone());
            theme_set.themes.insert("nedit-dark-complete".to_string(), theme.clone());
            theme_set.themes.insert("NEdit Dark".to_string(), theme);
        }
        Self::load_custom_themes_into(&mut theme_set, &config_dir);
        if let Some(theme) = Self::load_theme_by_name(&current_theme, &theme_set) {
            theme_set.themes.insert(current_theme.clone(), theme);
        } else {
            current_theme = default_theme_name();
            if let Some(theme) = Self::load_theme_by_name(&current_theme, &theme_set) {
                theme_set.themes.insert(current_theme.clone(), theme);
            }
        }

        let mut initial_root = current_dir.clone();
        for arg in args {
            let path = Self::parse_path_arg(arg);
            if path.is_dir() {
                initial_root = path;
                break;
            } else if let Some(root) = Self::find_project_root_or_parent(&path, &current_dir) {
                initial_root = root;
                break;
            }
        }
        let _ = std::env::set_current_dir(&initial_root);

        let (tx, rx) = channel();
        let watcher = RecommendedWatcher::new(tx, NotifyConfig::default()).ok();
        let (large_file_load_sender, large_file_load_receiver) = channel();

        let mut app = Self {
            buffers: Vec::new(),
            current_buffer_idx: 0,
            explorer: FileExplorer::new(initial_root.clone()),
            focus: Focus::Editor,
            show_explorer: false,
            should_quit: false,
            syntax_set: None,
            theme_set,
            themes_loaded: false,
            is_welcome: true,
            current_theme: current_theme.clone(),
            is_fuzzy: false,
            fuzzy_mode: crate::app::FuzzyMode::Files,
            fuzzy_query: String::new(),
            fuzzy_cursor: 0,
            fuzzy_sel_anchor: None,
            fuzzy_results: Vec::new(),
            fuzzy_file_results: Vec::new(),
            fuzzy_lines: Vec::new(),
            fuzzy_global_results: Vec::new(),
            all_files: Arc::new(Vec::new()),
            all_files_ready: false,
            fuzzy_idx: 0,
            original_theme: current_theme.clone(),
            fuzzy_themes: Vec::new(),
            config,
            i18n: I18n::load(),
            pending_path: None,
            pending_explorer_selection: None,
            move_dir: None,
            notifications: Vec::new(),
            live_script_mode: false,
            live_script_buffer_idx: None,
            target_buffer_idx: None,
            watcher,
            fs_event_receiver: rx,
            syntax_set_receiver: None,
            large_file_load_sender,
            large_file_load_receiver,
            next_large_file_load_id: 0,
            indexed_files_receiver: None,
            explorer_refresh_receiver: None,
            explorer_needs_refresh: false,
            content_search_receiver: None,
            content_search_seq: 0,
            fuzzy_files_receiver: None,
            fuzzy_files_seq: 0,
            fuzzy_files_cancel: None,
            fuzzy_input_timestamp: None,
            fuzzy_input_reset_idx: false,
            explorer_area: Rect::default(),
            editor_area: Rect::default(),
            screen_area: Rect::default(),
            fuzzy_limit: 20,
            last_script_undo: None,
            last_click_time: std::time::Instant::now(),
            last_click_pos: (0, 0),
            icon_registry: crate::ui::icons::IconRegistry::load(),
            pending_action: None,
            pending_buffer_idx: None,
            needs_redraw: true,
            preview_buffer_idx: None,
            saved_buffer_idx: 0,
            file_mtimes: HashMap::new(),
            last_external_check: Instant::now(),
        };

        let is_huge_system_dir = initial_root == Path::new("/")
            || dirs::home_dir().map(|h| h == initial_root).unwrap_or(false);

        if !is_huge_system_dir {
            if let Some(watcher) = &mut app.watcher {
                let _ = watcher.watch(&initial_root, Self::watch_mode_for_path(&initial_root));
            }
        }

        app.refresh_explorer();

        for arg in args {
            let path = Self::parse_path_arg(arg);
            if !path.is_dir() {
                let open_path = if path.is_absolute() {
                    path
                } else {
                    current_dir.join(path)
                };
                app.open_file(open_path);
            }
        }

        app
    }

    pub fn parse_path_arg(arg: &str) -> PathBuf {
        let clean = if let Some(stripped) = arg.strip_prefix("file://") {
            stripped
        } else {
            arg
        };
        PathBuf::from(clean)
    }

    pub fn find_project_root_or_parent(file_path: &Path, current_dir: &Path) -> Option<PathBuf> {
        let abs_path = if file_path.is_absolute() {
            file_path.to_path_buf()
        } else {
            current_dir.join(file_path)
        };

        let parent = abs_path.parent()?;
        if !parent.exists() {
            return None;
        }

        let parent_canonical = parent.canonicalize().unwrap_or_else(|_| parent.to_path_buf());
        let home = dirs::home_dir().and_then(|h| h.canonicalize().ok());

        let mut current = parent_canonical.as_path();
        loop {
            if let Some(ref h) = home {
                if current == h {
                    break;
                }
            }

            if current.join(".git").exists() {
                return Some(current.to_path_buf());
            }

            match current.parent() {
                Some(p) if p != current => current = p,
                _ => break,
            }
        }

        Some(parent_canonical)
    }

    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nedit")
    }

    fn load_theme_by_name(theme_name: &str, loaded_custom_themes: &ThemeSet) -> Option<Theme> {
        if let Some(theme) = loaded_custom_themes.themes.get(theme_name) {
            return Some(theme.clone());
        }

        if let Some(stripped) = theme_name.strip_suffix(".tmTheme") {
            if let Some(theme) = loaded_custom_themes.themes.get(stripped) {
                return Some(theme.clone());
            }
        }

        None
    }

    pub fn ensure_current_theme_loaded(&mut self) {
        let key = self.resolve_theme_key(&self.current_theme);
        if self.theme_set.themes.contains_key(&key) {
            self.current_theme = key;
            return;
        }

        if let Some(theme) = Self::load_theme_by_name(&self.current_theme, &self.theme_set) {
            self.theme_set
                .themes
                .insert(self.current_theme.clone(), theme);
            return;
        }

        self.current_theme = default_theme_name();
        if let Some(theme) = Self::load_theme_by_name(&self.current_theme, &self.theme_set) {
            self.theme_set
                .themes
                .insert(self.current_theme.clone(), theme);
        }
    }

    pub fn ensure_all_themes_loaded(&mut self) {
        if self.themes_loaded {
            return;
        }

        let config_dir = Self::config_dir();
        let mut theme_set = ThemeSet::new();
        if let Some(theme) = Self::load_embedded_theme() {
            theme_set.themes.insert("NEdit Dark Complete".to_string(), theme.clone());
            theme_set.themes.insert("nedit-dark-complete".to_string(), theme.clone());
            theme_set.themes.insert("NEdit Dark".to_string(), theme);
        }
        Self::load_custom_themes_into(&mut theme_set, &config_dir);
        self.theme_set = theme_set;
        self.themes_loaded = true;
        self.ensure_current_theme_loaded();
    }

    fn load_embedded_theme() -> Option<Theme> {
        let bytes = include_bytes!("../../themes/nedit-dark-complete.tmTheme");
        let temp_path = std::env::temp_dir().join("nedit-dark-complete-theme.tmTheme");
        std::fs::write(&temp_path, bytes).ok()?;
        let theme = ThemeSet::get_theme(&temp_path).ok()?;
        let _ = std::fs::remove_file(&temp_path);
        Some(theme)
    }

    fn load_custom_themes_into(theme_set: &mut ThemeSet, config_dir: &Path) {
        let themes_dir = config_dir.join("themes");
        let Ok(theme_paths) = ThemeSet::discover_theme_paths(&themes_dir) else {
            return;
        };

        for path in theme_paths {
            let Ok(theme) = ThemeSet::get_theme(&path) else {
                continue;
            };

            for alias in Self::theme_aliases_for_path(&path, &themes_dir, &theme) {
                theme_set.themes.insert(alias, theme.clone());
            }
        }
    }

    fn theme_aliases_for_path(path: &Path, themes_dir: &Path, theme: &Theme) -> Vec<String> {
        let mut aliases = Vec::new();
        let mut push_alias = |alias: String| {
            let alias = alias.trim();
            if !alias.is_empty() && !aliases.iter().any(|existing| existing == alias) {
                aliases.push(alias.to_string());
            }
        };

        if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
            push_alias(stem.to_string());
        }

        if let Ok(relative_path) = path.strip_prefix(themes_dir) {
            let relative_stem = relative_path.with_extension("");
            if let Some(relative_stem) = relative_stem.to_str() {
                push_alias(relative_stem.to_string());
            }
        }

        if let Some(theme_name) = theme.name.as_deref() {
            push_alias(theme_name.to_string());
        }

        aliases
    }

    pub fn ensure_syntax_set_loaded(&mut self) {
        if self.syntax_set.is_some() {
            return;
        }

        let config_dir = Self::config_dir();
        self.syntax_set = Some(Self::load_cached_or_default_syntax_set(&config_dir).0);
    }

    pub fn ensure_syntax_set_loading(&mut self) {
        self.start_syntax_set_loading(None);
    }

    pub fn ensure_syntax_for_path_loading(&mut self, path: Option<&Path>) {
        let extension = path
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
            .map(str::to_string);

        if let (Some(syntax_set), Some(extension)) = (&self.syntax_set, extension.as_deref()) {
            if syntax_set.find_syntax_by_extension(extension).is_some() {
                return;
            }
        }

        self.start_syntax_set_loading(extension);
    }

    fn start_syntax_set_loading(&mut self, requested_extension: Option<String>) {
        if self.syntax_set_receiver.is_some() {
            return;
        }

        let config_dir = Self::config_dir();
        let (tx, rx) = std::sync::mpsc::channel();
        self.syntax_set_receiver = Some(rx);

        std::thread::spawn(move || {
            let defaults = Self::load_default_syntax_set();
            let needs_custom = requested_extension
                .as_deref()
                .map(|extension| defaults.find_syntax_by_extension(extension).is_none())
                .unwrap_or(false);
            let _ = tx.send(defaults.clone());

            if !needs_custom {
                return;
            }

            if let Some(cached) = Self::load_cached_custom_syntax_set(&config_dir) {
                let _ = tx.send(cached);
                return;
            }

            if !Self::has_custom_syntax_files(&config_dir.join("syntax")) {
                return;
            }

            let custom = Self::build_custom_syntax_set(defaults, &config_dir);
            let _ = tx.send(custom);
        });
    }

    pub fn load_syntax_set_for_diagnostics() -> (SyntaxSet, &'static str) {
        let config_dir = Self::config_dir();
        Self::load_cached_or_default_syntax_set(&config_dir)
    }

    fn load_default_syntax_set() -> SyntaxSet {
        SyntaxSet::load_defaults_nonewlines()
    }

    fn load_cached_or_default_syntax_set(config_dir: &Path) -> (SyntaxSet, &'static str) {
        if let Some(cached) = Self::load_cached_custom_syntax_set(config_dir) {
            return (cached, "custom-cache");
        }

        let defaults = Self::load_default_syntax_set();
        if Self::has_custom_syntax_files(&config_dir.join("syntax")) {
            (defaults, "defaults-custom-cache-missing")
        } else {
            (defaults, "defaults")
        }
    }

    fn load_cached_custom_syntax_set(config_dir: &Path) -> Option<SyntaxSet> {
        let cache_dir = config_dir.join("cache");
        let cache_file = cache_dir.join("syntax_nonewlines.packdump");
        let stamp_file = cache_dir.join("syntax_nonewlines.stamp");
        let current_stamp = Self::custom_syntax_stamp(&config_dir.join("syntax"))?;
        let cached_stamp = fs::read_to_string(stamp_file).ok()?;

        if cached_stamp != current_stamp {
            return None;
        }

        syntect::dumps::from_uncompressed_dump_file(cache_file).ok()
    }

    fn build_custom_syntax_set(defaults: SyntaxSet, config_dir: &Path) -> SyntaxSet {
        let syntax_dir = config_dir.join("syntax");
        let mut builder = defaults.into_builder();
        let _ = builder.add_from_folder(&syntax_dir, false);
        let syntax_set = builder.build();
        Self::write_custom_syntax_cache(config_dir, &syntax_set);
        syntax_set
    }

    fn write_custom_syntax_cache(config_dir: &Path, syntax_set: &SyntaxSet) {
        let Some(stamp) = Self::custom_syntax_stamp(&config_dir.join("syntax")) else {
            return;
        };

        let cache_dir = config_dir.join("cache");
        if fs::create_dir_all(&cache_dir).is_err() {
            return;
        }

        let cache_file = cache_dir.join("syntax_nonewlines.packdump");
        let stamp_file = cache_dir.join("syntax_nonewlines.stamp");
        if syntect::dumps::dump_to_uncompressed_file(syntax_set, &cache_file).is_ok() {
            let _ = fs::write(stamp_file, stamp);
        }
    }

    fn has_custom_syntax_files(syntax_dir: &Path) -> bool {
        Self::custom_syntax_stamp(syntax_dir).is_some()
    }

    fn custom_syntax_stamp(syntax_dir: &Path) -> Option<String> {
        let mut entries = Vec::new();
        let read_dir = fs::read_dir(syntax_dir).ok()?;

        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("sublime-syntax") {
                continue;
            }

            let metadata = entry.metadata().ok()?;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| format!("{}.{}", duration.as_secs(), duration.subsec_nanos()))
                .unwrap_or_else(|| "unknown".to_string());
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string();
            entries.push(format!("{}:{}:{}", name, metadata.len(), modified));
        }

        if entries.is_empty() {
            return None;
        }

        entries.sort();
        Some(format!("v1-nonewlines\n{}", entries.join("\n")))
    }

    pub fn refresh_explorer(&mut self) {
        if self.explorer_refresh_receiver.is_some() {
            self.explorer_needs_refresh = true;
            return;
        }

        self.pending_explorer_selection = self
            .explorer
            .items
            .get(self.explorer.selected_idx)
            .map(|i| i.path.clone());

        let (tx, rx) = std::sync::mpsc::channel();
        self.explorer_refresh_receiver = Some(rx);

        let mut explorer_clone = crate::explorer::FileExplorer {
            root: self.explorer.root.clone(),
            items: Vec::new(),
            selected_idx: 0,
            expanded_paths: self.explorer.expanded_paths.clone(),
            scroll_offset: 0,
            max_item_width: 20,
            search_input: crate::line_input::LineInput::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_scroll: 0,
            search_hscroll: std::cell::Cell::new(0),
            search_corpus: Vec::new(),
        };

        std::thread::spawn(move || {
            explorer_clone.refresh_sync();
            let _ = tx.send((
                explorer_clone.items,
                explorer_clone.max_item_width,
                explorer_clone.search_corpus,
            ));
        });
    }

    pub fn record_file_mtime(&mut self, path: &Path) {
        if let Ok(meta) = fs::metadata(path) {
            if let Ok(mtime) = meta.modified() {
                self.file_mtimes.insert(path.to_path_buf(), mtime);
                return;
            }
        }
        // If file doesn't exist or mtime unreadable, forget it.
        self.file_mtimes.remove(path);
    }

    pub fn forget_file_mtime(&mut self, path: &Path) {
        self.file_mtimes.remove(path);
        // Also remove any child paths (for directory moves/deletes)
        self.file_mtimes.retain(|p, _| !p.starts_with(path));
    }

    /// Poll open buffers for external modifications.
    /// If a file changed on disk, show a small popup (same style as UnsavedChanges)
    /// asking to reload. Respects debounce and does not interrupt an existing fuzzy.
    pub fn check_external_modifications(&mut self) {
        // Debounce: check at most twice per second.
        if self.last_external_check.elapsed() < std::time::Duration::from_millis(500) {
            return;
        }
        self.last_external_check = Instant::now();

        // Don't pile up popups.
        if self.is_fuzzy {
            return;
        }

        for idx in 0..self.buffers.len() {
            let Some(path) = self.buffers[idx].path.clone() else {
                continue;
            };
            // Skip preview buffers — they are transient.
            if self.buffers[idx].is_preview {
                continue;
            }
            let Ok(meta) = fs::metadata(&path) else {
                // File deleted externally — forget mtime and continue.
                // We don't auto-close; user can still save to recreate.
                continue;
            };
            let Ok(mtime) = meta.modified() else {
                continue;
            };
            let known = self.file_mtimes.get(&path).copied();
            if let Some(known_mtime) = known {
                if mtime != known_mtime {
                    // File changed externally.
                    self.pending_buffer_idx = Some(idx);
                    self.pending_path = Some(path.clone());
                    self.is_fuzzy = true;
                    self.fuzzy_mode = crate::app::FuzzyMode::ExternalChange;
                    self.clear_fuzzy_query();
                    self.fuzzy_idx = 0;
                    self.needs_redraw = true;
                    // Update stored mtime to avoid immediate re-trigger
                    // until user decides. If they keep local, we update to new
                    // mtime so popup doesn't reappear constantly.
                    // If they reload, we'll update after reloading.
                    break;
                }
            } else {
                // First time we see this path — record it.
                self.file_mtimes.insert(path, mtime);
            }
        }
    }

    pub fn reload_buffer_from_disk(&mut self, idx: usize) {
        if idx >= self.buffers.len() {
            return;
        }
        let path = match self.buffers[idx].path.clone() {
            Some(p) => p,
            None => return,
        };
        if let Ok(content) = fs::read_to_string(&path) {
            let buf = &mut self.buffers[idx];
            let saved_row = buf.cursor_row;
            let saved_col = buf.cursor_col;
            buf.set_content_and_mark_clean(ropey::Rope::from_str(&content));
            buf.cursor_row = saved_row.min(buf.content.len_lines().saturating_sub(1));
            buf.cursor_col = saved_col.min(buf.line_max_char_col(buf.cursor_row));
            buf.cursor_goal_visual_col = 0;
            buf.selection_start = None;
            buf.sync_syntax_states(0);
            buf.sync_rendered_spans(0);
            buf.invalidate_max_visual_width();
            self.record_file_mtime(&path);
            self.show_notification(
                format!("Reloaded {}", path.display()),
                crate::app::NotificationType::Info,
            );
        } else {
            self.show_notification(
                format!("Could not reload {}", path.display()),
                crate::app::NotificationType::Error,
            );
        }
    }
}

fn default_theme_name() -> String {
    "NEdit Dark Complete".to_string()
}

#[cfg(test)]
mod tests {
    use super::App;
    use std::fs;
    use syntect::highlighting::ThemeSet;
    use tempfile::tempdir;

    #[test]
    fn loads_custom_themes_with_multiple_aliases() {
        let temp_dir = tempdir().unwrap();
        let config_dir = temp_dir.path();
        let themes_dir = config_dir.join("themes/nested");
        fs::create_dir_all(&themes_dir).unwrap();

        fs::write(
            themes_dir.join("Oceanic.tmTheme"),
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>name</key>
  <string>Base16 Ocean Dark</string>
  <key>settings</key>
  <array>
    <dict>
      <key>settings</key>
      <dict>
        <key>background</key>
        <string>#1B2B34</string>
        <key>foreground</key>
        <string>#C0C5CE</string>
      </dict>
    </dict>
  </array>
</dict>
</plist>
"#,
        )
        .unwrap();

        let mut theme_set = ThemeSet::new();
        App::load_custom_themes_into(&mut theme_set, config_dir);

        assert!(theme_set.themes.contains_key("Oceanic"));
        assert!(theme_set.themes.contains_key("nested/Oceanic"));
        assert!(theme_set.themes.contains_key("Base16 Ocean Dark"));
        assert!(App::load_theme_by_name("Oceanic.tmTheme", &theme_set).is_some());
        assert!(App::load_theme_by_name("nested/Oceanic.tmTheme", &theme_set).is_some());
    }

    #[test]
    fn adopts_parent_dir_as_explorer_root_when_opening_file() {
        let temp = tempdir().unwrap();
        let sub_dir = temp.path().join("teste");
        fs::create_dir_all(&sub_dir).unwrap();
        let file_path = sub_dir.join("arquivo.txt");
        fs::write(&file_path, "hello").unwrap();

        let app = App::new(&[file_path.to_str().unwrap().to_string()]);
        assert_eq!(
            app.explorer.root.canonicalize().unwrap(),
            sub_dir.canonicalize().unwrap()
        );
        assert_eq!(app.buffers.len(), 1);
        assert_eq!(app.buffers[0].path.as_ref(), Some(&file_path));
    }

    #[test]
    fn adopts_git_root_as_explorer_root_when_opening_file_inside_repo() {
        let temp = tempdir().unwrap();
        let repo_root = temp.path().join("my_repo");
        let git_dir = repo_root.join(".git");
        let sub_dir = repo_root.join("src");
        fs::create_dir_all(&git_dir).unwrap();
        fs::create_dir_all(&sub_dir).unwrap();
        let file_path = sub_dir.join("main.rs");
        fs::write(&file_path, "fn main() {}").unwrap();

        let app = App::new(&[file_path.to_str().unwrap().to_string()]);
        assert_eq!(
            app.explorer.root.canonicalize().unwrap(),
            repo_root.canonicalize().unwrap()
        );
        assert_eq!(app.buffers.len(), 1);
    }

    #[test]
    fn show_notification_with_duration_records_custom_duration() {
        let mut app = App::new(&[]);
        let dur = std::time::Duration::from_millis(1500);
        app.show_notification_with_duration(
            "test custom".to_string(),
            crate::app::NotificationType::Error,
            dur,
        );
        assert_eq!(app.notifications.len(), 1);
        assert_eq!(app.notifications[0].message, "test custom");
        assert_eq!(app.notifications[0].kind, crate::app::NotificationType::Error);
        assert_eq!(app.notifications[0].duration, dur);
    }
}
