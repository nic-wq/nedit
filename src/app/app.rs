use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};

use notify::{Config as NotifyConfig, RecommendedWatcher, Watcher};
use ratatui::layout::Rect;
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::SyntaxSet;

use crate::buffer::EditorBuffer;
use crate::config::Config;
use crate::explorer::FileExplorer;
use crate::i18n::I18n;

use super::{Focus, NotificationType};

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
    pub all_files: Vec<PathBuf>,
    pub all_files_ready: bool,
    pub fuzzy_idx: usize,
    pub original_theme: String,
    pub fuzzy_themes: Vec<String>,
    pub config: Config,
    pub i18n: I18n,
    pub workspaces: Vec<crate::app::Workspace>,
    pub current_workspace: Option<String>,
    pub temp_ws_name: Option<String>,
    pub pending_path: Option<PathBuf>,
    pub move_dir: Option<PathBuf>,
    pub pending_lua_actions: Vec<crate::lua::LuaAction>,
    pub notification: Option<(String, NotificationType)>,
    pub notification_timer: u8,
    pub live_script_mode: bool,
    pub live_script_buffer_idx: Option<usize>,
    pub target_buffer_idx: Option<usize>,
    pub watcher: Option<RecommendedWatcher>,
    pub fs_event_receiver: Receiver<notify::Result<notify::Event>>,
    pub explorer_area: Rect,
    pub editor_area: Rect,
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
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let config_dir = home_dir.join(".config/nedit");
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
        if let Some(theme) = Self::load_theme_by_name(&current_theme, &config_dir) {
            theme_set.themes.insert(current_theme.clone(), theme);
        } else {
            current_theme = default_theme_name();
            if let Some(theme) = Self::load_theme_by_name(&current_theme, &config_dir) {
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
            all_files: Vec::new(),
            all_files_ready: false,
            fuzzy_idx: 0,
            original_theme: current_theme.clone(),
            fuzzy_themes: Vec::new(),
            config,
            i18n: I18n::load(),
            workspaces: Vec::new(),
            current_workspace: None,
            temp_ws_name: None,
            pending_path: None,
            move_dir: None,
            pending_lua_actions: Vec::new(),
            notification: None,
            notification_timer: 0,
            live_script_mode: false,
            live_script_buffer_idx: None,
            target_buffer_idx: None,
            watcher,
            fs_event_receiver: rx,
            explorer_area: Rect::default(),
            editor_area: Rect::default(),
        };

        if let Some(watcher) = &mut app.watcher {
            let _ = watcher.watch(&current_dir, Self::watch_mode_for_path(&current_dir));
        }

        app.load_workspaces();

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

    fn config_dir() -> PathBuf {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".config/nedit")
    }

    fn load_theme_by_name(theme_name: &str, config_dir: &std::path::Path) -> Option<Theme> {
        let custom_theme = config_dir
            .join("themes")
            .join(format!("{}.tmTheme", theme_name));
        if let Ok(theme) = ThemeSet::get_theme(custom_theme) {
            return Some(theme);
        }

        ThemeSet::load_defaults().themes.remove(theme_name)
    }

    pub fn ensure_current_theme_loaded(&mut self) {
        if self.theme_set.themes.contains_key(&self.current_theme) {
            return;
        }

        let config_dir = Self::config_dir();
        if let Some(theme) = Self::load_theme_by_name(&self.current_theme, &config_dir) {
            self.theme_set
                .themes
                .insert(self.current_theme.clone(), theme);
            return;
        }

        self.current_theme = default_theme_name();
        if let Some(theme) = Self::load_theme_by_name(&self.current_theme, &config_dir) {
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
        let mut theme_set = ThemeSet::load_defaults();
        let _ = theme_set.add_from_folder(config_dir.join("themes"));
        self.theme_set = theme_set;
        self.themes_loaded = true;
        self.ensure_current_theme_loaded();
    }

    pub fn ensure_syntax_set_loaded(&mut self) {
        if self.syntax_set.is_some() {
            return;
        }

        let config_dir = Self::config_dir();
        let mut builder = SyntaxSet::load_defaults_newlines().into_builder();
        let _ = builder.add_from_folder(config_dir.join("syntax"), true);
        self.syntax_set = Some(builder.build());
    }
}

fn default_theme_name() -> String {
    "base16-ocean.dark".to_string()
}
