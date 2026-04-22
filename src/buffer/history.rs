use super::EditorBuffer;

impl EditorBuffer {
    pub fn undo(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            self.content = self.history[self.history_idx].clone();
            self.modified = true;
        }
    }

    pub fn redo(&mut self) {
        if self.history_idx < self.history.len() - 1 {
            self.history_idx += 1;
            self.content = self.history[self.history_idx].clone();
            self.modified = true;
        }
    }
}
