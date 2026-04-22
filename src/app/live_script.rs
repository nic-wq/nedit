use super::{App, Focus};

impl App {
    pub fn open_live_script(&mut self) {
        if self.buffers.is_empty() {
            return;
        }

        self.target_buffer_idx = Some(self.current_buffer_idx);

        let mut buffer = crate::buffer::EditorBuffer::new();
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
    }

    pub fn handle_fs_events(&mut self) {
        let mut changed = false;
        while let Ok(event) = self.fs_event_receiver.try_recv() {
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
            self.explorer.refresh();
        }
    }
}
