use super::{App, Focus};

impl App {
    pub fn open_live_script(&mut self) {
        if self.buffers.is_empty() {
            return;
        }

        self.target_buffer_idx = Some(self.current_buffer_idx);

        let mut buffer = crate::buffer::EditorBuffer::new();
        // The script pane has no file on disk, so point the highlighter at
        // Lua explicitly instead of falling back to plain text.
        buffer.syntax_override = Some("lua".to_string());
        // We provide a basic template with common operations to lower the barrier for entry
        // and show the user how to interact with the nedit API immediately.
        buffer.set_content_and_mark_clean(ropey::Rope::from_str(
            "-- Name: Live Script\n-- Press F9 to run on the other buffer\n\nlocal sel = nedit.selection()\nif sel ~= \"\" then\n    nedit.write_selection(sel:upper())\nend\n",
        ));

        self.buffers.push(buffer);
        self.live_script_buffer_idx = Some(self.buffers.len() - 1);
        self.current_buffer_idx = self.buffers.len() - 1;
        self.live_script_mode = true;
        self.focus = Focus::Editor;
        self.is_welcome = false;
        self.is_fuzzy = false;
        self.ensure_syntax_set_loading();
    }

    pub fn open_saved_script(&mut self, script_name: &str) {
        if self.buffers.is_empty() {
            let empty_buf = crate::buffer::EditorBuffer::new();
            self.buffers.push(empty_buf);
            self.current_buffer_idx = 0;
        }

        let clean_name = script_name.trim_end_matches(".lua");
        let script_path = Self::scripts_dir().join(format!("{}.lua", clean_name));
        let content = match std::fs::read_to_string(&script_path) {
            Ok(c) => c,
            Err(e) => {
                self.show_notification(
                    format!("Could not read script: {}", e),
                    crate::app::NotificationType::Error,
                );
                return;
            }
        };

        if !self.live_script_mode || self.live_script_buffer_idx.is_none() {
            self.open_live_script();
        }

        if let Some(script_idx) = self.live_script_buffer_idx {
            if script_idx < self.buffers.len() {
                self.buffers[script_idx].set_content_and_mark_clean(ropey::Rope::from_str(&content));
                self.buffers[script_idx].cursor_row = 0;
                self.buffers[script_idx].cursor_col = 0;
                self.live_script_origin = Some(clean_name.to_string());
                self.current_buffer_idx = script_idx;
                self.focus = Focus::Editor;
                self.is_fuzzy = false;
                self.show_notification(
                    format!("{}: {}", self.i18n.t("loaded_live_script"), clean_name),
                    crate::app::NotificationType::Info,
                );
                self.needs_redraw = true;
            }
        }
    }

    pub fn save_live_script(&mut self) {
        if !self.live_script_mode || self.live_script_buffer_idx.is_none() {
            return;
        }
        let script_idx = self.live_script_buffer_idx.unwrap();
        if script_idx >= self.buffers.len() {
            return;
        }

        match self.live_script_origin.clone() {
            None => {
                self.is_fuzzy = true;
                self.fuzzy_mode = crate::app::FuzzyMode::SaveLiveScript;
                self.clear_fuzzy_query();
                // Pre-fill from "-- Name: <name>" if present in script
                let buf = &self.buffers[script_idx];
                if buf.content.len_lines() > 0 {
                    let first_line = buf.content.line(0).to_string();
                    let trimmed = first_line.trim();
                    if let Some(name) = trimmed.strip_prefix("-- Name: ") {
                        let candidate = name.trim();
                        if candidate != "Live Script" && !candidate.is_empty() {
                            self.set_fuzzy_query(self.slugify(candidate));
                        }
                    }
                }
                self.modal_button_idx = None;
                self.needs_redraw = true;
            }
            Some(_existing_name) => {
                if self.buffers[script_idx].modified {
                    self.is_fuzzy = true;
                    self.fuzzy_mode = crate::app::FuzzyMode::SaveScriptChoice;
                    self.modal_button_idx = Some(0);
                    self.needs_redraw = true;
                } else {
                    self.show_notification(
                        self.i18n.t("script_already_up_to_date").to_string(),
                        crate::app::NotificationType::Info,
                    );
                }
            }
        }
    }

    pub fn execute_save_live_script(&mut self, name: &str) {
        let clean = name.trim().trim_end_matches(".lua").trim();
        if clean.is_empty() {
            self.show_notification(
                "Script name cannot be empty".to_string(),
                crate::app::NotificationType::Error,
            );
            return;
        }
        let Some(script_idx) = self.live_script_buffer_idx else {
            return;
        };
        if script_idx >= self.buffers.len() {
            return;
        }

        let dir = Self::scripts_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.show_notification(
                format!("Could not create scripts directory: {}", e),
                crate::app::NotificationType::Error,
            );
            return;
        }

        let file_path = dir.join(format!("{}.lua", clean));
        let content = self.buffers[script_idx].content.to_string();
        if let Err(e) = std::fs::write(&file_path, content) {
            self.show_notification(
                format!("Could not save script: {}", e),
                crate::app::NotificationType::Error,
            );
            return;
        }

        self.buffers[script_idx].modified = false;
        self.live_script_origin = Some(clean.to_string());
        self.is_fuzzy = false;
        self.modal_button_idx = None;
        self.show_notification(
            format!("{}: {}", self.i18n.t("script_saved"), clean),
            crate::app::NotificationType::Info,
        );
        self.needs_redraw = true;
    }

    pub fn execute_overwrite_live_script(&mut self) {
        if let Some(origin) = self.live_script_origin.clone() {
            self.execute_save_live_script(&origin);
        }
    }

    pub fn list_saved_scripts(&self) -> Vec<std::path::PathBuf> {
        let dir = Self::scripts_dir();
        let mut results = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "lua").unwrap_or(false) {
                    results.push(path);
                }
            }
        }
        results.sort();
        results
    }

    pub fn delete_saved_script(&mut self, path: &std::path::Path) {
        if let Err(e) = std::fs::remove_file(path) {
            self.show_notification(
                format!("Could not delete script: {}", e),
                crate::app::NotificationType::Error,
            );
        } else {
            let script_stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if self.live_script_origin.as_deref() == Some(&script_stem) {
                self.live_script_origin = None;
            }
            self.show_notification(
                format!("Deleted script: {}", script_stem),
                crate::app::NotificationType::Info,
            );
            self.update_fuzzy(true);
            self.needs_redraw = true;
        }
    }

    pub fn rename_saved_script(&mut self, old_path: &std::path::Path, new_name: &str) {
        let clean = new_name.trim().trim_end_matches(".lua").trim();
        if clean.is_empty() {
            self.show_notification(
                "New name cannot be empty".to_string(),
                crate::app::NotificationType::Error,
            );
            return;
        }
        let dir = Self::scripts_dir();
        let new_path = dir.join(format!("{}.lua", clean));
        if let Err(e) = std::fs::rename(old_path, &new_path) {
            self.show_notification(
                format!("Could not rename script: {}", e),
                crate::app::NotificationType::Error,
            );
        } else {
            let old_stem = old_path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if self.live_script_origin.as_deref() == Some(&old_stem) {
                self.live_script_origin = Some(clean.to_string());
            }
            self.show_notification(
                format!("Renamed to {}", clean),
                crate::app::NotificationType::Info,
            );
            self.update_fuzzy(true);
            self.needs_redraw = true;
        }
    }

    pub fn handle_fs_events(&mut self) {
        let mut changed = false;
        let mut processed = 0usize;
        while processed < 128 {
            let Ok(event) = self.fs_event_receiver.try_recv() else {
                break;
            };
            processed += 1;
            if let Ok(event) = event {
                match event.kind {
                    notify::EventKind::Create(_)
                    | notify::EventKind::Remove(_)
                    | notify::EventKind::Modify(notify::event::ModifyKind::Name(_)) => {
                        changed = true;
                    }
                    _ => {}
                }
            }
        }
        if changed {
            self.invalidate_file_index();
            self.refresh_explorer();
            self.needs_redraw = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Focus};
    use crate::buffer::EditorBuffer;
    use crate::explorer::FileItem;

    #[test]
    fn live_script_pane_highlights_lua_without_path() {
        let mut app = App::new(&[]);
        app.buffers.push(EditorBuffer::new());
        app.open_live_script();

        assert!(app.live_script_mode);
        let idx = app.live_script_buffer_idx.unwrap();
        let buf = &app.buffers[idx];
        assert!(buf.path.is_none());
        assert_eq!(buf.syntax_override.as_deref(), Some("lua"));

        // Resetting the pane via New File keeps Lua highlighting.
        app.current_buffer_idx = idx;
        app.new_file();
        assert_eq!(app.buffers[idx].syntax_override.as_deref(), Some("lua"));
    }

    #[test]
    fn live_script_can_be_saved_to_library() {
        let mut app = App::new(&[]);
        app.buffers.push(EditorBuffer::new());
        app.open_live_script();

        assert_eq!(app.current_buffer_idx, app.live_script_buffer_idx.unwrap());

        app.save_current_buffer();

        assert!(app.is_fuzzy);
        assert_eq!(app.fuzzy_mode, crate::app::types::FuzzyMode::SaveLiveScript);
    }

    #[test]
    fn opening_file_with_preview_keeps_script_on_the_right() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "AAA").unwrap();
        std::fs::write(dir.path().join("b.txt"), "BBB").unwrap();

        let mut app = App::new(&[]);
        app.set_explorer_root(dir.path().to_path_buf());
        app.open_file(dir.path().join("a.txt"));
        app.open_live_script();
        // [a(0), script(1)], browsing the explorer with preview on.
        app.show_explorer = true;
        app.focus = Focus::Explorer;
        app.explorer.items = ["a.txt", "b.txt"]
            .iter()
            .map(|name| FileItem {
                path: dir.path().join(name),
                is_dir: false,
                name: name.to_string(),
                depth: 0,
                expanded: false,
            })
            .collect();
        app.explorer.selected_idx = 1;

        // Browsing already shows the preview on the left pane.
        app.update_preview_from_explorer_selection();
        assert_eq!(app.target_buffer_idx, Some(1));
        assert_eq!(app.live_script_buffer_idx, Some(2));

        // Confirming with Enter opens the file on the left, script stays last.
        app.open_file(dir.path().join("b.txt"));
        assert_eq!(app.buffers.len(), 3);
        assert_eq!(app.current_buffer_idx, 1);
        assert_eq!(app.target_buffer_idx, Some(1));
        assert_eq!(app.live_script_buffer_idx, Some(2));
        assert_eq!(app.preview_buffer_idx, None);
        assert_eq!(
            app.buffers[1].path.as_deref(),
            Some(dir.path().join("b.txt").as_path())
        );
    }

    #[test]
    fn clear_preview_repairs_live_indices() {
        let mut app = App::new(&[]);
        app.buffers.push(EditorBuffer::new());
        app.open_live_script();
        // Simulate a preview inserted before the script: [t(0), prev(1), s(2)].
        let preview_idx = app.push_buffer(EditorBuffer::new());
        app.buffers[preview_idx].is_preview = true;
        app.preview_buffer_idx = Some(preview_idx);
        app.saved_buffer_idx = 0;
        app.current_buffer_idx = preview_idx;

        app.clear_preview(preview_idx);
        // Script slid back to 1 instead of dangling at 2.
        assert_eq!(app.live_script_buffer_idx, Some(1));
        assert_eq!(app.target_buffer_idx, Some(0));
        assert_eq!(app.preview_buffer_idx, None);
        assert_eq!(app.current_buffer_idx, 0);
    }

    #[test]
    fn docs_and_untitled_opens_become_live_target() {
        let mut app = App::new(&[]);
        app.buffers.push(EditorBuffer::new());
        app.open_live_script();
        app.current_buffer_idx = 0;

        app.new_file();
        assert_eq!(app.target_buffer_idx, Some(1));
        assert_eq!(app.live_script_buffer_idx, Some(2));

        app.open_doc("general");
        assert_eq!(app.target_buffer_idx, Some(app.current_buffer_idx));
        assert_eq!(app.live_script_buffer_idx, Some(app.buffers.len() - 1));
    }

    #[test]
    fn new_buffers_open_before_the_pinned_script_tab() {
        let mut app = App::new(&[]);
        app.buffers.push(EditorBuffer::new());
        app.open_live_script();
        // [target(0), script(1)], current on the script pane.
        assert_eq!(app.live_script_buffer_idx, Some(1));

        let idx = app.push_buffer(EditorBuffer::new());
        assert_eq!(idx, 1);
        assert_eq!(app.live_script_buffer_idx, Some(2));
        assert_eq!(app.buffers.len(), 3);
        // Stored indices at/after the insertion point shifted.
        assert_eq!(app.current_buffer_idx, 2);
        assert_eq!(app.target_buffer_idx, Some(0));

        // Closing the middle tab keeps the script last.
        app.force_close_buffer(1);
        assert_eq!(app.live_script_buffer_idx, Some(1));
        assert_eq!(app.buffers.len(), 2);
        assert_eq!(app.current_buffer_idx, 1);
    }

    #[test]
    fn test_save_open_rename_delete_live_script() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path());

        let mut app = App::new(&[]);
        app.buffers.push(EditorBuffer::new());
        app.open_live_script();

        let script_idx = app.live_script_buffer_idx.unwrap();
        app.buffers[script_idx].content = ropey::Rope::from_str("-- Name: Counter Test\nreturn 42\n");
        app.buffers[script_idx].modified = true;

        let script_name = "test_script_unit_sample";
        app.execute_save_live_script(script_name);

        assert_eq!(app.live_script_origin.as_deref(), Some(script_name));
        assert!(!app.buffers[script_idx].modified);

        let script_path = App::scripts_dir().join(format!("{}.lua", script_name));
        assert!(script_path.exists());

        // Modify content and trigger save_live_script -> prompts SaveScriptChoice
        app.buffers[script_idx].modified = true;
        app.save_live_script();
        assert!(app.is_fuzzy);
        assert_eq!(app.fuzzy_mode, crate::app::types::FuzzyMode::SaveScriptChoice);

        // Rename script
        let renamed_name = "test_script_unit_renamed";
        app.rename_saved_script(&script_path, renamed_name);
        let new_path = App::scripts_dir().join(format!("{}.lua", renamed_name));
        assert!(new_path.exists());
        assert!(!script_path.exists());
        assert_eq!(app.live_script_origin.as_deref(), Some(renamed_name));

        // Delete script
        app.delete_saved_script(&new_path);
        assert!(!new_path.exists());
        assert_eq!(app.live_script_origin, None);

        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
