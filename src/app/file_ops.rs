use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use notify::{RecursiveMode, Watcher};

use crate::app::{
    Focus, FuzzyMode, NotificationType, Workspace, WorkspaceList, DOC_BINDS, DOC_LUA, DOC_MAIN,
};
use crate::buffer::EditorBuffer;

use super::App;

impl App {
    pub(crate) fn watch_mode_for_path(_path: &Path) -> RecursiveMode {
        RecursiveMode::NonRecursive
    }

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
                    self.ensure_syntax_for_path_loading(Some(path.as_path()));
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
                self.ensure_syntax_for_path_loading(Some(path.as_path()));
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
            self.live_script_mode = false;
            self.live_script_buffer_idx = None;
            self.target_buffer_idx = None;
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
            if self.live_script_mode {
                let is_script = Some(closing_idx) == self.live_script_buffer_idx;
                let is_target = Some(closing_idx) == self.target_buffer_idx;

                if is_target {
                    self.close_live_script_pair(closing_idx);
                } else if is_script {
                    self.buffers.remove(closing_idx);
                    self.live_script_mode = false;
                    self.live_script_buffer_idx = None;
                    self.target_buffer_idx = None;
                } else {
                    self.buffers.remove(closing_idx);
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
            } else {
                self.buffers.remove(closing_idx);
            }

            if self.buffers.is_empty() {
                self.is_welcome = true;
                self.current_buffer_idx = 0;
                self.live_script_mode = false;
                self.live_script_buffer_idx = None;
                self.target_buffer_idx = None;
            } else {
                self.current_buffer_idx = self.current_buffer_idx.min(self.buffers.len() - 1);
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
    }

    fn close_live_script_pair(&mut self, fallback_idx: usize) {
        let mut indexes = Vec::new();
        indexes.push(fallback_idx);
        if let Some(idx) = self.live_script_buffer_idx {
            indexes.push(idx);
        }
        if let Some(idx) = self.target_buffer_idx {
            indexes.push(idx);
        }

        indexes.sort_unstable();
        indexes.dedup();

        for idx in indexes.into_iter().rev() {
            if idx < self.buffers.len() {
                self.buffers.remove(idx);
            }
        }

        self.live_script_mode = false;
        self.live_script_buffer_idx = None;
        self.target_buffer_idx = None;
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
            self.refresh_explorer();
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
            let _ = watcher.watch(&path, Self::watch_mode_for_path(&path));
        }
        self.explorer.root = path;
        self.explorer.selected_idx = 0;
        self.refresh_explorer();
        self.invalidate_file_index();
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
        let path = self.buffers[self.current_buffer_idx].path.clone();
        self.ensure_syntax_for_path_loading(path.as_deref());
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
                    self.refresh_explorer();
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
        let live_script_indexes: Vec<usize> = [self.live_script_buffer_idx, self.target_buffer_idx]
            .into_iter()
            .flatten()
            .collect();
        let close_live_script = self.live_script_mode
            && live_script_indexes.iter().any(|&idx| {
                self.buffers
                    .get(idx)
                    .and_then(|buffer| buffer.path.as_ref())
                    .map(|path| path.starts_with(removed_path))
                    .unwrap_or(false)
            });

        let mut idx = 0;
        self.buffers.retain(|buffer| {
            let should_remove = buffer
                .path
                .as_ref()
                .map(|path| path.starts_with(removed_path))
                .unwrap_or(false)
                || (close_live_script && live_script_indexes.contains(&idx));
            idx += 1;
            !should_remove
        });

        if close_live_script {
            self.live_script_mode = false;
            self.live_script_buffer_idx = None;
            self.target_buffer_idx = None;
        }

        if self.buffers.is_empty() {
            self.current_buffer_idx = 0;
            self.is_welcome = true;
            self.live_script_mode = false;
            self.live_script_buffer_idx = None;
            self.target_buffer_idx = None;
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

    pub fn undo_last_script(&mut self) {
        let Some(undo) = self.last_script_undo.take() else {
            self.show_notification(
                "No script action to undo".to_string(),
                NotificationType::Info,
            );
            return;
        };

        for revert in undo.actions.into_iter().rev() {
            match revert {
                crate::lua::RevertAction::RestoreBufferContent {
                    buffer_idx,
                    content: old_content,
                    cursor,
                } => {
                    if let Some(buf) = self.buffers.get_mut(buffer_idx) {
                        buf.content = ropey::Rope::from_str(&old_content);
                        buf.cursor_row = cursor.0;
                        buf.cursor_col = cursor.1;
                        buf.selection_start = None;
                    }
                }
                crate::lua::RevertAction::RestoreFile {
                    path,
                    content: old_content,
                } => {
                    if let Some(actual_content) = old_content {
                        let _ = std::fs::write(&path, &actual_content);
                        // Update any open buffers with this path
                        for buf in &mut self.buffers {
                            if buf.path.as_ref() == Some(&path) {
                                buf.content = ropey::Rope::from_str(&actual_content);
                                buf.modified = false;
                            }
                        }
                    } else {
                        let _ = std::fs::remove_file(&path);
                        self.close_buffers_for_path(&path);
                    }
                }
            }
        }
        self.show_notification(
            "Last script action undone".to_string(),
            NotificationType::Info,
        );
        self.refresh_explorer();
    }

    pub fn apply_lua_actions(&mut self, actions: Vec<crate::lua::LuaAction>) {
        if actions.is_empty() {
            return;
        }

        let target_idx = if self.live_script_mode {
            self.target_buffer_idx.unwrap_or(self.current_buffer_idx)
        } else {
            self.current_buffer_idx
        };

        let mut reverts = Vec::new();

        for action in actions {
            match action {
                crate::lua::LuaAction::WriteSelection(text) => {
                    if let Some(buf) = self.buffers.get_mut(target_idx) {
                        if buf.selection_start.is_none() {
                            self.show_notification(
                                "Error: write_selection requires selected text.".to_string(),
                                crate::app::NotificationType::Error,
                            );
                            continue;
                        }
                        reverts.push(crate::lua::RevertAction::RestoreBufferContent {
                            buffer_idx: target_idx,
                            content: buf.content.to_string(),
                            cursor: (buf.cursor_row, buf.cursor_col),
                        });
                        buf.delete_selection();
                        for c in text.chars() {
                            buf.insert_char(c);
                        }
                    }
                }
                crate::lua::LuaAction::WriteCurrentFile(text) => {
                    if self.live_script_mode {
                        if let Some(target_buf) = self.buffers.get(target_idx) {
                            if target_buf.path.is_none() {
                                self.show_notification(
                                    "Error: target file has no path".to_string(),
                                    crate::app::NotificationType::Error,
                                );
                                continue;
                            }
                        }
                    }
                    if let Some(buf) = self.buffers.get_mut(target_idx) {
                        reverts.push(crate::lua::RevertAction::RestoreBufferContent {
                            buffer_idx: target_idx,
                            content: buf.content.to_string(),
                            cursor: (buf.cursor_row, buf.cursor_col),
                        });
                        buf.content = ropey::Rope::from_str(&text);
                        buf.cursor_row = 0;
                        buf.cursor_col = 0;
                    }
                }
                crate::lua::LuaAction::WriteFile(path, text) => {
                    let prev_content = std::fs::read_to_string(&path).ok();
                    reverts.push(crate::lua::RevertAction::RestoreFile {
                        path: path.clone(),
                        content: prev_content,
                    });
                    let _ = std::fs::write(&path, text);
                }
                crate::lua::LuaAction::CreateFile(path, text) => {
                    let prev_content = std::fs::read_to_string(&path).ok();
                    reverts.push(crate::lua::RevertAction::RestoreFile {
                        path: path.clone(),
                        content: prev_content,
                    });
                    let _ = std::fs::write(&path, text);
                }
                crate::lua::LuaAction::DeleteFile(path) => {
                    let prev_content = std::fs::read_to_string(&path).ok();
                    if let Some(content) = prev_content {
                        reverts.push(crate::lua::RevertAction::RestoreFile {
                            path: path.clone(),
                            content: Some(content),
                        });
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }

        if !reverts.is_empty() {
            self.last_script_undo = Some(crate::lua::ScriptUndo { actions: reverts });
        }

        self.refresh_explorer();
    }
}
