use super::EditorBuffer;

impl EditorBuffer {
    pub fn insert_char(&mut self, ch: char) {
        self.push_history();
        let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
        self.content.insert_char(char_idx, ch);

        if ch == '\n' {
            self.cursor_row += 1;
            self.cursor_col = 0;
            self.autocomplete_options.clear();
        } else {
            self.cursor_col += 1;
        }
        self.modified = true;
    }

    pub fn delete_backspace(&mut self) {
        let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
        if char_idx > 0 {
            self.push_history();
            self.content.remove(char_idx - 1..char_idx);

            if self.cursor_col > 0 {
                self.cursor_col -= 1;
            } else if self.cursor_row > 0 {
                self.cursor_row -= 1;
                self.cursor_col = self.content.line(self.cursor_row).len_chars();
                if self.content.line(self.cursor_row).chars().last() == Some('\n') {
                    self.cursor_col -= 1;
                }
            }
            self.modified = true;
        }
    }
}
