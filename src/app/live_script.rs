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
}
