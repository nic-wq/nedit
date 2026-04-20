use std::fs;
use std::path::PathBuf;
use crate::buffer::EditorBuffer;
use crate::explorer::FileExplorer;
use crate::config::Config;
use crate::i18n::I18n;

const DOC_LUA: &str = include_str!("../docs/lua.md");
const DOC_BINDS: &str = include_str!("../docs/binds.md");
const DOC_MAIN: &str = include_str!("../docs/docs.md");

use syntect::parsing::{SyntaxSet, SyntaxSetBuilder};
use syntect::highlighting::ThemeSet;

#[derive(PartialEq, Eq)]
pub enum Focus {
    Explorer,
    Editor,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum FuzzyMode {
    Files,
    Content,
    Local,
    Themes,
    SaveAs,
    FileOptions,
    Rename,
    DeleteConfirm,
    Workspaces,
    WorkspaceAddName,
    WorkspaceAddPath,
    CommandPalette,
    Move,
    RunScript,
    ScriptConfirm,
    EditScript,
    DeleteScript,
    DocSelect,
    NewFolder,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct Workspace {
    pub name: String,
    pub path: PathBuf,
    pub tabs: Vec<PathBuf>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct WorkspaceList {
    pub workspaces: Vec<Workspace>,
}

pub struct App {
    pub buffers: Vec<EditorBuffer>,
    pub current_buffer_idx: usize,
    pub explorer: FileExplorer,
    pub focus: Focus,
    pub show_explorer: bool,
    pub should_quit: bool,
    pub syntax_set: SyntaxSet,
    pub theme_set: ThemeSet,
    pub is_welcome: bool,
    pub current_theme: String,
    // Fuzzy Finder
    pub is_fuzzy: bool,
    pub fuzzy_mode: FuzzyMode,
    pub fuzzy_query: String,
    pub fuzzy_results: Vec<PathBuf>,
    pub fuzzy_lines: Vec<(usize, String)>,
    pub fuzzy_global_results: Vec<(PathBuf, usize, String)>,
    pub all_files: Vec<PathBuf>,
    pub fuzzy_idx: usize,
    pub original_theme: String,
    pub fuzzy_themes: Vec<String>,
    pub config: Config,
    pub i18n: I18n,
    pub workspaces: Vec<Workspace>,
    pub current_workspace: Option<String>,
    pub temp_ws_name: Option<String>,
    pub move_dir: Option<PathBuf>,
    pub pending_lua_actions: Vec<crate::lua::LuaAction>,
    pub notification: Option<(String, NotificationType)>,
    pub notification_timer: u8,
    pub live_script_mode: bool,
    pub live_script_buffer_idx: Option<usize>,
    pub target_buffer_idx: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Error,
    Info,
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
        let home_dir = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
        let config_dir = home_dir.join(".config/nedit");
        let _ = fs::create_dir_all(&config_dir);
        let _ = fs::create_dir_all(config_dir.join("syntax"));
        let _ = fs::create_dir_all(config_dir.join("themes"));

        // Load default syntaxes
        let mut builder = SyntaxSetBuilder::new();
        builder.add_plain_text_syntax();
        
        // Try to load custom syntaxes from ~/.config/nedit/syntax
        let _ = builder.add_from_folder(config_dir.join("syntax"), true);
        
        let syntax_set = if builder.syntaxes().len() <= 1 {
            SyntaxSet::load_defaults_newlines()
        } else {
            builder.build()
        };

        let mut theme_set = ThemeSet::load_defaults();
        let _ = theme_set.add_from_folder(config_dir.join("themes"));

        // Try to load last used theme
        let theme_file = config_dir.join("theme.txt");
        let mut current_theme = "base16-ocean.dark".to_string();
        if let Ok(saved_theme) = fs::read_to_string(&theme_file) {
            let saved_theme = saved_theme.trim();
            if theme_set.themes.contains_key(saved_theme) {
                current_theme = saved_theme.to_string();
            }
        }

        let mut app = Self {
            buffers: Vec::new(),
            current_buffer_idx: 0,
            explorer: FileExplorer::new(current_dir),
            focus: Focus::Editor,
            show_explorer: false,
            should_quit: false,
            syntax_set,
            theme_set,
            is_welcome: true,
            current_theme: current_theme.clone(),
            is_fuzzy: false,
            fuzzy_mode: FuzzyMode::Files,
            fuzzy_query: String::new(),
            fuzzy_results: Vec::new(),
            fuzzy_lines: Vec::new(),
            fuzzy_global_results: Vec::new(),
            all_files: Vec::new(),
            fuzzy_idx: 0,
            original_theme: current_theme.clone(),
            fuzzy_themes: Vec::new(),
            config: Config::load(),
            i18n: I18n::load(),
            workspaces: Vec::new(),
            current_workspace: None,
            temp_ws_name: None,
            move_dir: None,
            pending_lua_actions: Vec::new(),
            notification: None,
            notification_timer: 0,
            live_script_mode: false,
            live_script_buffer_idx: None,
            target_buffer_idx: None,
        };
        
        app.load_workspaces();

        for arg in args {
            let path = PathBuf::from(arg);
            if path.is_dir() {
                app.set_explorer_root(path);
            } else {
                app.open_file(path);
            }
        }

        // Auto-detect workspace
        let root = app.explorer.root.clone();
        if let Some(ws) = app.workspaces.iter().find(|w| w.path == root).cloned() {
            app.current_workspace = Some(ws.name.clone());
            if app.buffers.is_empty() {
                for tab_path in ws.tabs {
                    app.open_file(tab_path);
                }
            }
        }

        app
    }

    pub fn open_file(&mut self, path: PathBuf) {
        // Check if file is already open
        for (i, buf) in self.buffers.iter().enumerate() {
            if let Some(p) = &buf.path {
                if p == &path {
                    self.current_buffer_idx = i;
                    self.focus = Focus::Editor;
                    self.is_welcome = false;
                    
                    // In live script mode, update target_buffer_idx if opening a regular file
                    if self.live_script_mode && i != self.live_script_buffer_idx.unwrap_or(usize::MAX) {
                        self.target_buffer_idx = Some(i);
                    }
                    return;
                }
            }
        }

        if let Ok(buffer) = EditorBuffer::from_file(path.clone()) {
            self.buffers.push(buffer);
            self.current_buffer_idx = self.buffers.len() - 1;
            self.focus = Focus::Editor;
            self.is_welcome = false;
            
            // In live script mode, update target_buffer_idx for newly opened files
            if self.live_script_mode {
                self.target_buffer_idx = Some(self.current_buffer_idx);
            }
        } else if !path.is_dir() {
            let mut buffer = EditorBuffer::new();
            buffer.path = Some(path);
            self.buffers.push(buffer);
            self.current_buffer_idx = self.buffers.len() - 1;
            self.focus = Focus::Editor;
            self.is_welcome = false;
            
            // In live script mode, update target_buffer_idx for newly opened files
            if self.live_script_mode {
                self.target_buffer_idx = Some(self.current_buffer_idx);
            }
        }
    }

    pub fn load_workspaces(&mut self) {
        let home_dir = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
        let ws_file = home_dir.join(".config/nedit/workspaces.toml");
        if let Ok(content) = std::fs::read_to_string(&ws_file) {
            if let Ok(ws_list) = toml::from_str::<WorkspaceList>(&content) {
                self.workspaces = ws_list.workspaces;
            }
        }
    }

    pub fn save_workspaces(&mut self) {
        let home_dir = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
        let ws_file = home_dir.join(".config/nedit/workspaces.toml");
        
        // Update current workspace tabs before saving
        if let Some(name) = &self.current_workspace {
            let tabs: Vec<PathBuf> = self.buffers.iter()
                .filter_map(|b| b.path.clone())
                .collect();
            if let Some(ws) = self.workspaces.iter_mut().find(|w| &w.name == name) {
                ws.tabs = tabs;
            }
        }

        let ws_list = WorkspaceList { workspaces: self.workspaces.clone() };
        if let Ok(content) = toml::to_string(&ws_list) {
            let _ = std::fs::write(&ws_file, content);
        }
    }

    pub fn switch_workspace(&mut self, name: String) {
        // Save current first
        self.save_workspaces();

        if let Some(ws) = self.workspaces.iter().find(|w| w.name == name).cloned() {
            self.current_workspace = Some(name);
            self.set_explorer_root(ws.path.clone());
            self.buffers.clear();
            self.current_buffer_idx = 0;
            for tab_path in ws.tabs {
                self.open_file(tab_path);
            }
            if self.buffers.is_empty() {
                self.is_welcome = true;
            }
        }
    }

    pub fn close_current_buffer(&mut self) {
        if !self.buffers.is_empty() {
            let closing_idx = self.current_buffer_idx;
            self.buffers.remove(closing_idx);

            // Handle Live Script mode indices
            if self.live_script_mode {
                let is_script = Some(closing_idx) == self.live_script_buffer_idx;
                let is_target = Some(closing_idx) == self.target_buffer_idx;

                if is_script || is_target {
                    // Turn off live mode if one of its components is closed
                    self.live_script_mode = false;
                    self.live_script_buffer_idx = None;
                    self.target_buffer_idx = None;
                } else {
                    // Update indices if they shifted
                    if let Some(idx) = self.live_script_buffer_idx {
                        if closing_idx < idx {
                            self.live_script_buffer_idx = Some(idx - 1);
                        }
                    }
                    if let Some(idx) = self.target_buffer_idx {
                        if closing_idx < idx {
                            self.target_buffer_idx = Some(idx - 1);
                        }
                    }
                }
            }

            if self.buffers.is_empty() {
                self.is_welcome = true;
                self.current_buffer_idx = 0;
            } else {
                self.current_buffer_idx = self.current_buffer_idx.min(self.buffers.len() - 1);
            }
        }
    }

    pub fn switch_tab(&mut self, idx: usize) {
        if idx < self.buffers.len() {
            self.current_buffer_idx = idx;
            self.is_welcome = false;
            
            // In live script mode, update target_buffer_idx if switching away from script buffer
            if self.live_script_mode && idx != self.live_script_buffer_idx.unwrap_or(usize::MAX) {
                self.target_buffer_idx = Some(idx);
            }
        }
    }

    pub fn toggle_explorer(&mut self) {
        self.show_explorer = !self.show_explorer;
        if self.show_explorer {
            self.explorer.refresh();
            self.focus = Focus::Explorer;
        } else if self.focus == Focus::Explorer {
            self.focus = Focus::Editor;
        }
    }

    pub fn new_file(&mut self) {
        // If in live script mode and focused on script buffer, replace it instead
        if self.live_script_mode && self.current_buffer_idx == self.live_script_buffer_idx.unwrap_or(usize::MAX) {
            if let Some(script_idx) = self.live_script_buffer_idx {
                if script_idx < self.buffers.len() {
                    self.buffers[script_idx] = EditorBuffer::new();
                }
            }
        } else {
            self.buffers.push(EditorBuffer::new());
            self.current_buffer_idx = self.buffers.len() - 1;
        }
        self.focus = Focus::Editor;
        self.is_welcome = false;
    }

    pub fn set_explorer_root(&mut self, path: PathBuf) {
        self.explorer.root = path;
        self.explorer.selected_idx = 0;
        self.explorer.refresh();
    }

    pub fn switch_tab_relative(&mut self, delta: isize) {
        if self.buffers.is_empty() { return; }
        let len = self.buffers.len() as isize;
        self.current_buffer_idx = ((self.current_buffer_idx as isize + delta).rem_euclid(len)) as usize;
        self.is_welcome = false;
        
        // In live script mode, update target_buffer_idx if switching away from script buffer
        if self.live_script_mode && self.current_buffer_idx != self.live_script_buffer_idx.unwrap_or(usize::MAX) {
            self.target_buffer_idx = Some(self.current_buffer_idx);
        }
    }

    pub fn open_docs(&mut self) {
        self.toggle_fuzzy(FuzzyMode::DocSelect);
    }

    pub fn open_doc(&mut self, doc_type: &str) {
        let filename = if doc_type == "lua" {
            "docs/lua.md"
        } else if doc_type == "binds" {
            "docs/binds.md"
        } else {
            "docs/docs.md"
        };
        
        // Check if doc is already open
        for (i, buf) in self.buffers.iter().enumerate() {
            if let Some(path) = &buf.path {
                if path.to_string_lossy() == filename {
                    self.current_buffer_idx = i;
                    self.focus = Focus::Editor;
                    self.is_welcome = false;
                    self.is_fuzzy = false;
                    return;
                }
            }
        }
        
        let content = if doc_type == "lua" {
            DOC_LUA.to_string()
        } else if doc_type == "binds" {
            DOC_BINDS.to_string()
        } else {
            DOC_MAIN.to_string()
        };
        
        let mut buffer = EditorBuffer::new();
        buffer.content = ropey::Rope::from_str(&content);
        buffer.is_read_only = true;
        buffer.path = Some(PathBuf::from(filename));
        
        self.buffers.push(buffer);
        self.current_buffer_idx = self.buffers.len() - 1;
        self.focus = Focus::Editor;
        self.is_welcome = false;
        self.is_fuzzy = false;
    }

    pub fn save_current_buffer(&mut self) {
        if self.buffers.is_empty() { return; }
        
        let has_path = self.buffers[self.current_buffer_idx].path.is_some();
        if !has_path {
            self.toggle_fuzzy(FuzzyMode::SaveAs);
        } else {
            let buffer = &mut self.buffers[self.current_buffer_idx];
            let _ = buffer.save();
            self.explorer.refresh();
        }
    }

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
            self.collect_all_files();
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

    fn collect_all_files(&mut self) {
        use walkdir::WalkDir;
        self.all_files = WalkDir::new(&self.explorer.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();
    }

    pub fn update_fuzzy(&mut self) {
        let query = self.fuzzy_query.to_lowercase();
        if query.is_empty() && self.fuzzy_mode == FuzzyMode::Content {
            self.fuzzy_results = Vec::new();
            return;
        }

        if self.fuzzy_mode == FuzzyMode::CommandPalette {
            let commands = vec![
                "Save", "New File", "Open File", "Close Tab", "Toggle Explorer",
                "Global Search", "Local Search", "Switch Theme", "Workspaces",
                "Open Lua Script", "Run Lua Script", "Edit Lua Script", "Delete Lua Script",
                "Open Live Script",
                "Quit", "Undo", "Redo", "Copy", "Paste", "Cut", "Select All", "Open Help"
            ];
            self.fuzzy_results = commands.into_iter()
                .filter(|c| query.is_empty() || c.to_lowercase().contains(&query))
                .map(PathBuf::from)
                .collect();
            self.fuzzy_idx = 0;
            return;
        }

        if self.fuzzy_mode == FuzzyMode::RunScript {
            self.fuzzy_results = Vec::new();
            let home_dir = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
            let scripts_dir = home_dir.join(".config/nedit/scripts");
            let _ = fs::create_dir_all(&scripts_dir);
            if let Ok(entries) = fs::read_dir(&scripts_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().map(|e| e == "lua").unwrap_or(false) {
                        // Fallback name: filename without .lua extension
                        let stem = entry.path()
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let name = if let Ok(content) = fs::read_to_string(entry.path()) {
                            // Use first line if it's a Lua comment: -- NAME
                            if let Some(first_line) = content.lines().next() {
                                let trimmed = first_line.trim();
                                if trimmed.starts_with("-- ") {
                                    trimmed[3..].trim().to_string()
                                } else {
                                    stem.clone()
                                }
                            } else { stem.clone() }
                        } else { stem.clone() };
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
            let home_dir = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
            let scripts_dir = home_dir.join(".config/nedit/scripts");
            let _ = fs::create_dir_all(&scripts_dir);
            if let Ok(entries) = fs::read_dir(&scripts_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().map(|e| e == "lua").unwrap_or(false) {
                        let stem = entry.path()
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
                            } else { stem.clone() }
                        } else { stem.clone() };
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
                self.fuzzy_results = self.all_files.iter()
                    .filter(|p| {
                        let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                        if query.is_empty() { return true; }
                        let mut it = query.chars();
                        let mut curr = it.next();
                        for c in name.chars() {
                            if let Some(target) = curr {
                                if c == target { curr = it.next(); }
                            }
                        }
                        curr.is_none()
                    })
                    .cloned()
                    .take(20)
                    .collect();
            } else if self.fuzzy_mode == FuzzyMode::Content {
                // Global Content Search
                let mut count = 0;
                for path in &self.all_files {
                    if let Ok(content) = fs::read_to_string(path) {
                        for (i, line) in content.lines().enumerate() {
                            if line.to_lowercase().contains(&query) {
                                self.fuzzy_global_results.push((path.clone(), i, line.to_string()));
                                count += 1;
                                if count >= 20 { break; }
                            }
                        }
                    }
                    if count >= 20 { break; }
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
                    if self.fuzzy_lines.len() >= 20 { break; }
                }
            }
        }

        if self.fuzzy_mode == FuzzyMode::Themes {
            let themes: Vec<String> = self.theme_set.themes.keys().cloned().collect();
            self.fuzzy_themes = themes.into_iter()
                .filter(|t| query.is_empty() || t.to_lowercase().contains(&query))
                .collect();
        }

        self.fuzzy_idx = 0;
        
        // Dynamic preview for themes
        if self.fuzzy_mode == FuzzyMode::Themes && !self.fuzzy_themes.is_empty() {
            self.current_theme = self.fuzzy_themes[0].clone();
        }
    }

    pub fn apply_theme(&mut self, theme_name: String) {
        if self.theme_set.themes.contains_key(&theme_name) {
            self.current_theme = theme_name;
        }
    }

    pub fn save_current_theme(&self) {
        let home_dir = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
        let theme_file = home_dir.join(".config/nedit/theme.txt");
        let _ = fs::write(theme_file, &self.current_theme);
    }

    pub fn open_live_script(&mut self) {
        if self.buffers.is_empty() { return; }
        
        self.target_buffer_idx = Some(self.current_buffer_idx);
        
        let mut buffer = EditorBuffer::new();
        buffer.content = ropey::Rope::from_str("-- Name: Live Script\n-- Press F9 to run on the other buffer\n\nlocal sel = nedit.selection()\nif sel ~= \"\" then\n    nedit.write_selection(sel:upper())\nend\n");
        
        self.buffers.push(buffer);
        self.live_script_buffer_idx = Some(self.buffers.len() - 1);
        self.current_buffer_idx = self.buffers.len() - 1;
        self.live_script_mode = true;
        self.focus = Focus::Editor;
        self.is_welcome = false;
        self.is_fuzzy = false;
    }

}
