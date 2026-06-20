use super::EditorBuffer;

impl EditorBuffer {
    pub fn copy(&mut self) {
        if let Some(text) = self.get_selected_text() {
            crate::clipboard::copy(&text);
        }
    }

    pub fn paste(&mut self) {
        if let Some(text) = crate::clipboard::paste() {
            // We push history before the operation to allow the user to undo the entire paste
            // as a single atomic action.
            self.push_history();
            let old_row = self.cursor_row;
            self.delete_selection();
            let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
            self.content.insert(char_idx, &text);
            let new_rope = ropey::Rope::from_str(&text);
            let lines = new_rope.len_lines();
            if lines > 1 {
                self.cursor_row += lines - 1;
                self.cursor_col = new_rope.line(lines - 1).len_chars();
            } else {
                self.cursor_col += new_rope.len_chars();
            }
            self.sync_cursor_goal_from_position();
            self.modified = true;
            self.sync_syntax_states(old_row);
            self.sync_rendered_spans(old_row);
        }
    }

    pub fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }
}
