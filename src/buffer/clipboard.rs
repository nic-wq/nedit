use super::EditorBuffer;

impl EditorBuffer {
    pub fn copy(&mut self) {
        if let Some(text) = self.get_selected_text() {
            crate::clipboard::copy(&text);
        }
    }

    pub fn paste(&mut self) {
        if let Some(text) = crate::clipboard::paste() {
            self.push_history();
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
            self.modified = true;
        }
    }

    pub fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }
}
