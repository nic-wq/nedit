use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use notify::{Config as NotifyConfig, RecommendedWatcher, Watcher};
use ratatui::layout::Rect;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::buffer::EditorBuffer;
use crate::config::Config;
use crate::explorer::FileExplorer;
use crate::i18n::I18n;

use super::{Focus, NotificationType};

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
    pub fuzzy_results: Vec<PathBuf>,
    pub fuzzy_lines: Vec<(usize, String)>,
    pub fuzzy_global_results: Vec<(PathBuf, usize, String)>,
    pub all_files: Arc<Vec<PathBuf>>,
    pub all_files_ready: bool,
    pub fuzzy_idx: usize,
    pub original_theme: String,
    pub fuzzy_themes: Vec<String>,
    pub config: Config,
    pub i18n: I18n,
    pub pending_path: Option<PathBuf>,
    pub pending_explorer_selection: Option<PathBuf>,
    pub move_dir: Option<PathBuf>,
    pub notification: Option<(String, NotificationType)>,
    pub notification_timer: u8,
    pub live_script_mode: bool,
    pub live_script_buffer_idx: Option<usize>,
    pub target_buffer_idx: Option<usize>,
    pub watcher: Option<RecommendedWatcher>,
    pub fs_event_receiver: Receiver<notify::Result<notify::Event>>,
    pub syntax_set_receiver: Option<Receiver<SyntaxSet>>,
    pub indexed_files_receiver: Option<Receiver<Vec<PathBuf>>>,
    pub explorer_refresh_receiver: Option<Receiver<(Vec<crate::explorer::FileItem>, usize)>>,
    pub explorer_needs_refresh: bool,
    pub content_search_receiver: Option<Receiver<(String, Vec<(PathBuf, usize, String)>)>>,
    pub explorer_area: Rect,
    pub editor_area: Rect,
    pub fuzzy_limit: usize,
    pub last_script_undo: Option<crate::lua::ScriptUndo>,
    pub script_response_tx: Option<std::sync::mpsc::Sender<crate::lua::ScriptResponse>>,
    pub script_request_rx: Option<std::sync::mpsc::Receiver<crate::lua::ScriptRequest>>,
    pub script_action_rx: Option<std::sync::mpsc::Receiver<Vec<crate::lua::LuaAction>>>,
    pub last_click_time: std::time::Instant,
    pub last_click_pos: (u16, u16),
    pub icon_registry: crate::ui::icons::IconRegistry,
    pub pending_action: Option<crate::app::types::PendingAction>,
    pub pending_buffer_idx: Option<usize>,
}

impl App {
    pub fn show_notification(&mut self, message: String, ntype: NotificationType) {
        self.notification = Some((message, ntype));
        self.notification_timer = 5;
    }

    pub fn clear_notification(&mut self) {
        self.notification = None;
        self.notification_timer = 0;
    }

    pub fn tick_notification(&mut self) {
        if self.notification_timer > 0 {
            self.notification_timer -= 1;
            if self.notification_timer == 0 {
                self.notification = None;
            }
        }
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

        let (tx, rx) = channel();
        let watcher = RecommendedWatcher::new(tx, NotifyConfig::default()).ok();

        let mut app = Self {
            buffers: Vec::new(),
            current_buffer_idx: 0,
            explorer: FileExplorer::new(current_dir.clone()),
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
            fuzzy_results: Vec::new(),
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
            notification: None,
            notification_timer: 0,
            live_script_mode: false,
            live_script_buffer_idx: None,
            target_buffer_idx: None,
            watcher,
            fs_event_receiver: rx,
            syntax_set_receiver: None,
            indexed_files_receiver: None,
            explorer_refresh_receiver: None,
            explorer_needs_refresh: false,
            content_search_receiver: None,
            explorer_area: Rect::default(),
            editor_area: Rect::default(),
            fuzzy_limit: 20,
            last_script_undo: None,
            script_response_tx: None,
            script_request_rx: None,
            script_action_rx: None,
            last_click_time: std::time::Instant::now(),
            last_click_pos: (0, 0),
            icon_registry: crate::ui::icons::IconRegistry::load(),
            pending_action: None,
            pending_buffer_idx: None,
        };

        if let Some(watcher) = &mut app.watcher {
            let _ = watcher.watch(&current_dir, Self::watch_mode_for_path(&current_dir));
        }

        app.refresh_explorer();

        for arg in args {
            let path = PathBuf::from(arg);
            if path.is_dir() {
                app.set_explorer_root(path);
            } else {
                app.open_file(path);
            }
        }

        app
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
            theme_set.themes.insert("NEdit Dark".to_string(), theme);
        }
        Self::load_custom_themes_into(&mut theme_set, &config_dir);
        self.theme_set = theme_set;
        self.themes_loaded = true;
        self.ensure_current_theme_loaded();
    }

    fn load_embedded_theme() -> Option<Theme> {
        let bytes = include_bytes!("../../themes/nedit-dark.tmTheme");
        let temp_path = std::env::temp_dir().join("nedit-dark-theme.tmTheme");
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

    pub(crate) fn should_skip_dir_name(name: &str) -> bool {
        matches!(
            name,
            ".git"
                | ".hg"
                | ".svn"
                | "target"
                | "node_modules"
                | "dist"
                | "build"
                | ".cache"
                | ".next"
                | ".nuxt"
                | "vendor"
                | "proc"
                | "sys"
                | "dev"
                | "run"
        )
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
        };

        std::thread::spawn(move || {
            explorer_clone.refresh_sync();
            let _ = tx.send((explorer_clone.items, explorer_clone.max_item_width));
        });
    }
}

fn default_theme_name() -> String {
    "NEdit Dark".to_string()
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
}
