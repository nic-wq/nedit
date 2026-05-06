use super::EditorBuffer;

impl EditorBuffer {
    pub fn insert_char(&mut self, ch: char) {
        self.push_history();
        let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
        self.content.insert_char(char_idx, ch);

        if ch == '\n' {
            let prev_row = self.cursor_row;
            let prev_line = self.content.line(prev_row);
            let mut indentation = String::new();
            for c in prev_line.chars() {
                if c == ' ' || c == '\t' {
                    indentation.push(c);
                } else {
                    break;
                }
            }

            self.cursor_row += 1;
            self.cursor_col = 0;

            if !indentation.is_empty() {
                let new_char_idx = self.content.line_to_char(self.cursor_row);
                self.content.insert(new_char_idx, &indentation);
                self.cursor_col = indentation.chars().count();
            }

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
