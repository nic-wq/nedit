use super::{App, Focus};

impl App {
    pub fn open_live_script(&mut self) {
        if self.buffers.is_empty() {
            return;
        }

        self.target_buffer_idx = Some(self.current_buffer_idx);

        let mut buffer = crate::buffer::EditorBuffer::new();
        // We provide a basic template with common operations to lower the barrier for entry
        // and show the user how to interact with the nedit API immediately.
        buffer.content = ropey::Rope::from_str(
            "-- Name: Live Script\n-- Press F9 to run on the other buffer\n\nlocal sel = nedit.selection()\nif sel ~= \"\" then\n    nedit.write_selection(sel:upper())\nend\n",
        );

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
