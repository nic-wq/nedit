use super::EditorBuffer;

impl EditorBuffer {
    pub fn move_cursor(&mut self, dr: isize, dc: isize, width: usize) {
        let line_count = self.content.len_lines();
        let new_row = (self.cursor_row as isize + dr).max(0) as usize;
        self.cursor_row = new_row.min(line_count - 1);

        let line_len = self.content.line(self.cursor_row).len_chars();
        let mut max_col = line_len;
        if self.content.line(self.cursor_row).chars().last() == Some('\n') {
            max_col = max_col.saturating_sub(1);
        }

        if dc != 0 {
            let new_col = (self.cursor_col as isize + dc).max(0) as usize;
            self.cursor_col = new_col.min(max_col);
        } else {
            self.cursor_col = self.cursor_col.min(max_col);
        }

        if width > 5 {
            let edit_width = width - 5;
            if self.cursor_col < self.scroll_col {
                self.scroll_col = self.cursor_col;
            } else if self.cursor_col >= self.scroll_col + edit_width {
                self.scroll_col = self.cursor_col - edit_width + 1;
            }
        }

        self.autocomplete_options.clear();
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor_col = 0;
        self.autocomplete_options.clear();
    }

    pub fn move_to_line_end(&mut self) {
        let line_len = self.content.line(self.cursor_row).len_chars();
        self.cursor_col =
            if line_len > 0 && self.content.line(self.cursor_row).chars().last() == Some('\n') {
                line_len - 1
            } else {
                line_len
            };
        self.autocomplete_options.clear();
    }

    pub fn move_word(&mut self, dir: isize) {
        let mut char_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
        if dir > 0 {
            while char_idx < self.content.len_chars()
                && !self.content.char(char_idx).is_alphanumeric()
            {
                char_idx += 1;
            }
            while char_idx < self.content.len_chars()
                && self.content.char(char_idx).is_alphanumeric()
            {
                char_idx += 1;
            }
        } else {
            if char_idx > 0 {
                char_idx -= 1;
            }
            while char_idx > 0 && !self.content.char(char_idx).is_alphanumeric() {
                char_idx -= 1;
            }
            while char_idx > 0 && self.content.char(char_idx).is_alphanumeric() {
                char_idx -= 1;
            }
            if char_idx > 0 && !self.content.char(char_idx).is_alphanumeric() {
                char_idx += 1;
            }
        }
        let (row, col) = self.char_to_line_col(char_idx);
        self.cursor_row = row;
        self.cursor_col = col;
    }
}
