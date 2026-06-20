use super::EditorBuffer;

impl EditorBuffer {
    pub fn undo(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            // We clone the Rope here because Ropey's cloning is extremely cheap (O(1) copy-on-write),
            // making state-based undo/redo very efficient.
            self.content = self.history[self.history_idx].clone();
            self.modified = true;
            self.sync_syntax_states(0);
            self.sync_rendered_spans(0);
        }
    }

    pub fn redo(&mut self) {
        if self.history_idx < self.history.len() - 1 {
            self.history_idx += 1;
            self.content = self.history[self.history_idx].clone();
            self.modified = true;
            self.sync_syntax_states(0);
            self.sync_rendered_spans(0);
        }
    }
}
