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

    /// Toggles focus between the target buffer and the script buffer in live script mode.
    /// Does nothing if live_script_mode is not active or either buffer index is missing.
    pub fn toggle_live_script_pane(&mut self) {
        if !self.live_script_mode {
            return;
        }
        if let (Some(target), Some(script)) = (self.target_buffer_idx, self.live_script_buffer_idx)
        {
            self.current_buffer_idx = if self.current_buffer_idx == target {
                script
            } else {
                target
            };
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
    use super::App;
    use crate::buffer::EditorBuffer;

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
}
