use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use notify::{RecursiveMode, Watcher};

use crate::app::{Focus, FuzzyMode, NotificationType, DOC_BINDS, DOC_LUA, DOC_MAIN};
use crate::buffer::EditorBuffer;

use super::App;

impl App {
    pub(crate) fn watch_mode_for_path(_path: &Path) -> RecursiveMode {
        RecursiveMode::NonRecursive
    }

    pub fn open_file(&mut self, path: PathBuf) {
        // We handle directories by switching the explorer root instead of opening them as buffers
        // to maintain a consistent UX where the editor only deals with text content.
        if path.is_dir() {
            self.set_explorer_root(path);
            self.focus = Focus::Explorer;
            return;
        }

        // Limpar preview se existir antes de abrir um arquivo real
        if let Some(preview_idx) = self.preview_buffer_idx.take() {
            self.clear_preview(preview_idx);
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
                self.record_file_mtime(&path);
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

    pub fn close_current_buffer(&mut self) {
        if !self.buffers.is_empty() {
            let closing_idx = self.current_buffer_idx;
            if self.buffers[closing_idx].modified {
                self.pending_action = Some(crate::app::types::PendingAction::CloseTab);
                self.pending_buffer_idx = Some(closing_idx);
                self.toggle_fuzzy(crate::app::FuzzyMode::UnsavedChanges);
                return;
            }

            self.force_close_buffer(closing_idx);
        }
    }

    pub fn force_close_buffer(&mut self, closing_idx: usize) {
        if closing_idx < self.buffers.len() {
            // Remember path to forget mtime
            let mut forget_paths = Vec::new();
            if let Some(p) = self.buffers[closing_idx].path.clone() {
                forget_paths.push(p);
            }
            if self.live_script_mode {
                let is_script = Some(closing_idx) == self.live_script_buffer_idx;
                let is_target = Some(closing_idx) == self.target_buffer_idx;

                if is_target {
                    // close_live_script_pair will handle its own forget
                    self.close_live_script_pair(closing_idx);
                    for p in forget_paths {
                        self.forget_file_mtime(&p);
                    }
                    // Also forget the other buffers in the pair
                    // (close_live_script_pair already forgets, but double forget is safe)
                } else if is_script {
                    self.buffers.remove(closing_idx);
                    for p in forget_paths {
                        self.forget_file_mtime(&p);
                    }
                    self.live_script_mode = false;
                    self.live_script_buffer_idx = None;
                    self.target_buffer_idx = None;
                } else {
                    self.buffers.remove(closing_idx);
                    for p in forget_paths {
                        self.forget_file_mtime(&p);
                    }
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
                for p in forget_paths {
                    self.forget_file_mtime(&p);
                }
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

        // Forget mtimes before removing
        for &idx in &indexes {
            if let Some(p) = self.buffers.get(idx).and_then(|b| b.path.clone()) {
                self.forget_file_mtime(&p);
            }
        }

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
        if !self.show_explorer {
            // Hidden → show and focus explorer
            self.show_explorer = true;
            self.refresh_explorer();
            self.focus = Focus::Explorer;
        } else if self.focus == Focus::Editor {
            // Visible + editor focused → just focus explorer
            self.focus = Focus::Explorer;
        } else {
            // Visible + explorer focused → hide explorer, return to editor
            if let Some(idx) = self.preview_buffer_idx.take() {
                self.clear_preview(idx);
            }
            self.show_explorer = false;
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
                    // Still the script pane: keep Lua highlighting without a path.
                    self.buffers[script_idx].syntax_override = Some("lua".to_string());
                }
            }
        } else {
            // Discard preview buffer first — same as open_file — so the new
            // untitled buffer becomes the current one and indices stay consistent.
            if let Some(preview_idx) = self.preview_buffer_idx.take() {
                self.clear_preview(preview_idx);
            }
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
        self.explorer.scroll_offset = 0;
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
        buffer.set_content_and_mark_clean(ropey::Rope::from_str(&content));
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
                    if let Some(p) = path.clone() {
                        self.record_file_mtime(&p);
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
            return path;
        }

        let base = if let Some(selected) = self.explorer.get_selected() {
            if selected.is_dir {
                selected.path.clone()
            } else {
                selected.path.parent().unwrap_or(&self.explorer.root).to_path_buf()
            }
        } else {
            self.explorer.root.clone()
        };

        base.join(path)
    }

    pub fn create_path_from_input(&mut self, raw: &str) {
        let trimmed = raw.trim_end_matches('/');
        let is_folder = raw.ends_with('/') || raw.is_empty();

        let resolved = self.resolve_input_path(trimmed);
        if resolved.exists() {
            self.show_notification(
                format!("Path already exists: {}", resolved.display()),
                NotificationType::Error,
            );
            return;
        }

        if let Some(parent) = resolved.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    self.show_notification(
                        format!("Error creating parent directories: {}", e),
                        NotificationType::Error,
                    );
                    return;
                }
            }
        }

        if is_folder {
            match std::fs::create_dir(&resolved) {
                Ok(()) => {
                    self.show_notification(
                        format!("Folder created: {}", resolved.display()),
                        NotificationType::Info,
                    );
                }
                Err(e) => {
                    self.show_notification(
                        format!("Error creating folder: {}", e),
                        NotificationType::Error,
                    );
                }
            }
        } else {
            match std::fs::File::create(&resolved) {
                Ok(_) => {
                    self.open_file(resolved);
                    self.show_notification(
                        format!("File created: {}", trimmed),
                        NotificationType::Info,
                    );
                }
                Err(e) => {
                    self.show_notification(
                        format!("Error creating file: {}", e),
                        NotificationType::Error,
                    );
                }
            }
        }
        self.refresh_explorer();
    }

    pub fn update_buffer_paths(&mut self, old_path: &Path, new_path: &Path) {
        // Update mtimes map for the move/rename.
        if let Some(mtime) = self.file_mtimes.remove(old_path) {
            self.file_mtimes.insert(new_path.to_path_buf(), mtime);
        }
        // Also handle children of a moved directory.
        let mut moved = Vec::new();
        for (p, m) in self.file_mtimes.iter() {
            if p.starts_with(old_path) && p != old_path {
                if let Ok(rel) = p.strip_prefix(old_path) {
                    moved.push((p.clone(), new_path.join(rel), *m));
                }
            }
        }
        for (old, new, m) in moved {
            self.file_mtimes.remove(&old);
            self.file_mtimes.insert(new, m);
        }
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
        // Re-stat new path.
        self.record_file_mtime(new_path);
    }

    pub fn close_buffers_for_path(&mut self, removed_path: &Path) {
        self.forget_file_mtime(removed_path);
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
                        buf.sync_cursor_goal_from_position();
                        buf.selection_start = None;
                        buf.clamp_cursor_to_content();
                        buf.push_history();
                        buf.refresh_modified();
                        buf.sync_syntax_states(0);
                        buf.sync_rendered_spans(0);
                        buf.invalidate_max_visual_width();
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
                                buf.set_content_and_mark_clean(ropey::Rope::from_str(
                                    &actual_content,
                                ));
                                buf.clamp_cursor_to_content();
                                buf.sync_syntax_states(0);
                                buf.sync_rendered_spans(0);
                                buf.invalidate_max_visual_width();
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
                        buf.cursor_goal_visual_col = 0;
                        buf.push_history();
                        buf.refresh_modified();
                        buf.sync_syntax_states(0);
                        buf.sync_rendered_spans(0);
                        buf.invalidate_max_visual_width();
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

    /// Atualiza o preview baseado na seleção atual do explorer.
    /// Chamado quando o usuário navega (Up/Down) no explorer sobre arquivos.
    pub fn update_preview_from_explorer_selection(&mut self) {
        // Só preview se explorer está visível e focado
        if !self.show_explorer || self.focus != Focus::Explorer {
            return;
        }

        // Se preview está desabilitado na config, não faz nada
        if !self.config.preview_enabled {
            if let Some(idx) = self.preview_buffer_idx {
                self.clear_preview(idx);
            }
            return;
        }

        let Some(item) = self.explorer.get_selected() else { return };

        // Só preview em arquivos (não diretórios)
        if item.is_dir {
            if let Some(idx) = self.preview_buffer_idx {
                self.clear_preview(idx);
            }
            return;
        }

        // Verificar se já está fazendo preview DESTE arquivo
        if let Some(idx) = self.preview_buffer_idx {
            if let Some(buf) = self.buffers.get(idx) {
                if buf.path.as_ref() == Some(&item.path) {
                    return; // já é o preview atual
                }
            }
        }

        // Verificar se o arquivo já está aberto como aba REAL
        for (i, buf) in self.buffers.iter().enumerate() {
            if !buf.is_preview && buf.path.as_ref() == Some(&item.path) {
                // Arquivo já está aberto — apenas exibir na aba existente
                if let Some(preview_idx) = self.preview_buffer_idx {
                    self.clear_preview(preview_idx);
                }
                self.current_buffer_idx = i;
                self.is_welcome = false;
                self.needs_redraw = true;
                return;
            }
        }

        // Verificar limite de tamanho para preview
        if let Ok(metadata) = std::fs::metadata(&item.path) {
            if metadata.len() > self.config.preview_max_size as u64 {
                if let Some(idx) = self.preview_buffer_idx {
                    self.clear_preview(idx);
                }
                return;
            }
        }

        // Carregar preview
        match EditorBuffer::from_path(item.path.clone()) {
            Ok(mut buf) => {
                buf.is_preview = true;
                buf.is_read_only = true;

                if let Some(preview_idx) = self.preview_buffer_idx {
                    // Substituir preview existente no lugar
                    self.buffers[preview_idx] = buf;
                    self.current_buffer_idx = preview_idx;
                } else {
                    // Salvar buffer atual para restaurar depois
                    self.saved_buffer_idx = self.current_buffer_idx;
                    // Criar novo buffer preview
                    self.buffers.push(buf);
                    self.preview_buffer_idx = Some(self.buffers.len() - 1);
                    self.current_buffer_idx = self.buffers.len() - 1;
                }

                self.is_welcome = false;
                self.needs_redraw = true;
            }
            Err(_) => {
                // Se não conseguir carregar, limpar preview
                if let Some(idx) = self.preview_buffer_idx {
                    self.clear_preview(idx);
                }
            }
        }
    }

    /// Limpa o preview atual e restaura o buffer anterior.
    pub fn clear_preview(&mut self, preview_idx: usize) {
        if preview_idx >= self.buffers.len() || !self.buffers[preview_idx].is_preview {
            self.preview_buffer_idx = None;
            return;
        }

        // Remover o preview buffer
        self.buffers.remove(preview_idx);
        self.preview_buffer_idx = None;

        // Ajustar saved_buffer_idx se necessário
        if self.saved_buffer_idx > preview_idx && self.saved_buffer_idx > 0 {
            self.saved_buffer_idx -= 1;
        }

        // Restaurar buffer anterior
        if self.buffers.is_empty() {
            self.is_welcome = true;
            self.current_buffer_idx = 0;
        } else {
            self.current_buffer_idx = self.saved_buffer_idx.min(self.buffers.len().saturating_sub(1));
        }

        self.needs_redraw = true;
    }
}
