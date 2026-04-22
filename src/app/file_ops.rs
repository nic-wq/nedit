use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use notify::{RecursiveMode, Watcher};

use crate::app::{
    Focus, FuzzyMode, NotificationType, Workspace, WorkspaceList, DOC_BINDS, DOC_LUA, DOC_MAIN,
};
use crate::buffer::EditorBuffer;

use super::App;

impl App {
    pub fn open_file(&mut self, path: PathBuf) {
        if path.is_dir() {
            self.set_explorer_root(path);
            self.focus = Focus::Explorer;
            return;
        }

        for (i, buf) in self.buffers.iter().enumerate() {
            if let Some(p) = &buf.path {
                if p == &path {
                    self.current_buffer_idx = i;
                    self.focus = Focus::Editor;
                    self.is_welcome = false;
                    if self.live_script_mode
                        && i != self.live_script_buffer_idx.unwrap_or(usize::MAX)
                    {
                        self.target_buffer_idx = Some(i);
                    }
                    return;
                }
            }
        }

        match EditorBuffer::from_path(path.clone()) {
            Ok(buffer) => {
                self.buffers.push(buffer);
                self.current_buffer_idx = self.buffers.len() - 1;
                self.focus = Focus::Editor;
                self.is_welcome = false;
                if self.live_script_mode {
                    self.target_buffer_idx = Some(self.current_buffer_idx);
                }
            }
            Err(err) => {
                let message = match err.downcast_ref::<std::io::Error>().map(|e| e.kind()) {
                    Some(ErrorKind::NotFound) => {
                        format!("File not found: {}", path.display())
                    }
                    Some(ErrorKind::PermissionDenied) => {
                        format!("Permission denied: {}", path.display())
                    }
                    Some(ErrorKind::InvalidData) => {
                        format!("Cannot open binary file: {}", path.display())
                    }
                    _ => format!("Could not open file {}: {}", path.display(), err),
                };
                self.show_notification(message, NotificationType::Error);
            }
        }
    }

    pub fn load_workspaces(&mut self) {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let ws_file = home_dir.join(".config/nedit/workspaces.toml");
        if let Ok(content) = std::fs::read_to_string(&ws_file) {
            if let Ok(ws_list) = toml::from_str::<WorkspaceList>(&content) {
                self.workspaces = ws_list.workspaces;
                if let Some(active) = ws_list.active_workspace {
                    if let Some(workspace) =
                        self.workspaces.iter().find(|w| w.name == active).cloned()
                    {
                        self.current_workspace = Some(active);
                        self.set_explorer_root(workspace.path);
                        self.buffers.clear();
                        self.current_buffer_idx = 0;
                        self.is_welcome = true;
                        for tab_path in workspace.tabs {
                            self.open_file(tab_path);
                        }
                    }
                }
            }
        }
    }

    pub fn save_workspaces(&mut self) {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        let ws_file = home_dir.join(".config/nedit/workspaces.toml");
        if let Some(name) = &self.current_workspace {
            let tabs: Vec<PathBuf> = self.buffers.iter().filter_map(|b| b.path.clone()).collect();
            if let Some(ws) = self.workspaces.iter_mut().find(|w| &w.name == name) {
                ws.tabs = tabs;
            }
        }

        let ws_list = WorkspaceList {
            active_workspace: self.current_workspace.clone(),
            workspaces: self.workspaces.clone(),
        };
        if let Ok(content) = toml::to_string(&ws_list) {
            if let Some(parent) = ws_file.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(&ws_file, content);
        }
    }

    pub fn refresh_workspace_results(&mut self) {
        self.fuzzy_results = self
            .workspaces
            .iter()
            .map(|w| PathBuf::from(&w.name))
            .collect();
        if self.current_workspace.is_some() {
            self.fuzzy_results.push(PathBuf::from("Exit Workspace"));
        }
        self.fuzzy_results.push(PathBuf::from("New Workspace..."));
        self.fuzzy_idx = 0;
    }

    pub fn create_workspace(&mut self, name: String, path: PathBuf) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Workspace name cannot be empty".to_string());
        }
        if self.workspaces.iter().any(|w| w.name == name) {
            return Err(format!("Workspace '{}' already exists", name));
        }
        if !path.is_dir() {
            return Err(format!(
                "Workspace path is not a directory: {}",
                path.display()
            ));
        }

        let tabs = self
            .buffers
            .iter()
            .filter_map(|buffer| buffer.path.clone())
            .collect();
        self.workspaces.push(Workspace {
            name: name.clone(),
            path,
            tabs,
        });
        self.switch_workspace(name.clone());
        self.show_notification(
            format!("Workspace '{}' created", name),
            NotificationType::Info,
        );
        Ok(())
    }

    pub fn exit_workspace(&mut self) {
        if let Some(name) = self.current_workspace.take() {
            self.save_workspaces();
            self.show_notification(
                format!("Exited workspace '{}'", name),
                NotificationType::Info,
            );
        }
    }

    pub fn switch_workspace(&mut self, name: String) {
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
            if self.live_script_mode {
                let is_script = Some(closing_idx) == self.live_script_buffer_idx;
                let is_target = Some(closing_idx) == self.target_buffer_idx;

                if is_script || is_target {
                    self.live_script_mode = false;
                    self.live_script_buffer_idx = None;
                    self.target_buffer_idx = None;
                } else {
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
        if self.live_script_mode
            && self.current_buffer_idx == self.live_script_buffer_idx.unwrap_or(usize::MAX)
        {
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
        if let Some(watcher) = &mut self.watcher {
            let _ = watcher.unwatch(&self.explorer.root);
            let _ = watcher.watch(&path, RecursiveMode::Recursive);
        }
        self.explorer.root = path;
        self.explorer.selected_idx = 0;
        self.explorer.refresh();
        self.collect_all_files();
    }

    pub fn switch_tab_relative(&mut self, delta: isize) {
        if self.buffers.is_empty() {
            return;
        }
        let len = self.buffers.len() as isize;
        self.current_buffer_idx =
            ((self.current_buffer_idx as isize + delta).rem_euclid(len)) as usize;
        self.is_welcome = false;
        if self.live_script_mode
            && self.current_buffer_idx != self.live_script_buffer_idx.unwrap_or(usize::MAX)
        {
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
        if self.buffers.is_empty() {
            return;
        }

        let has_path = self.buffers[self.current_buffer_idx].path.is_some();
        if !has_path {
            self.toggle_fuzzy(FuzzyMode::SaveAs);
        } else {
            let buffer = &mut self.buffers[self.current_buffer_idx];
            let path = buffer.path.clone();
            match buffer.save() {
                Ok(()) => {
                    self.explorer.refresh();
                    if let Some(p) = path {
                        self.show_notification(
                            format!("Saved to {}", p.display()),
                            NotificationType::Info,
                        );
                    }
                }
                Err(err) => self.show_notification(
                    format!("Could not save file: {}", err),
                    NotificationType::Error,
                ),
            }
        }
    }

    pub fn resolve_input_path(&self, raw: &str) -> PathBuf {
        let path = PathBuf::from(raw);
        if path.is_absolute() {
            path
        } else {
            self.explorer.root.join(path)
        }
    }

    pub fn update_buffer_paths(&mut self, old_path: &Path, new_path: &Path) {
        for buffer in &mut self.buffers {
            if let Some(path) = &buffer.path {
                if path == old_path {
                    buffer.path = Some(new_path.to_path_buf());
                } else if path.starts_with(old_path) {
                    if let Ok(relative) = path.strip_prefix(old_path) {
                        buffer.path = Some(new_path.join(relative));
                    }
                }
            }
        }
    }

    pub fn close_buffers_for_path(&mut self, removed_path: &Path) {
        self.buffers.retain(|buffer| {
            buffer
                .path
                .as_ref()
                .map(|path| !path.starts_with(removed_path))
                .unwrap_or(true)
        });
        if self.buffers.is_empty() {
            self.current_buffer_idx = 0;
            self.is_welcome = true;
        } else {
            self.current_buffer_idx = self.current_buffer_idx.min(self.buffers.len() - 1);
        }
        if self.live_script_mode {
            let script_valid = self
                .live_script_buffer_idx
                .map(|idx| idx < self.buffers.len())
                .unwrap_or(false);
            let target_valid = self
                .target_buffer_idx
                .map(|idx| idx < self.buffers.len())
                .unwrap_or(false);
            if !script_valid || !target_valid {
                self.live_script_mode = false;
                self.live_script_buffer_idx = None;
                self.target_buffer_idx = None;
            }
        }
    }
}
