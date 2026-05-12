use super::inner::EditorBuffer;

impl EditorBuffer {
    pub fn get_selected_text(&self) -> Option<String> {
        if let Some(start) = self.selection_start {
            let start_idx = self.to_char_idx(start.0, start.1);
            let end_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
            let (s, e) = if start_idx < end_idx {
                (start_idx, end_idx)
            } else {
                (end_idx, start_idx)
            };
            Some(self.content.slice(s..e).to_string())
        } else {
            None
        }
    }

    pub fn delete_selection(&mut self) {
        if let Some(start) = self.selection_start {
            self.push_history();
            let start_idx = self.to_char_idx(start.0, start.1);
            let end_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
            let (s, e) = if start_idx < end_idx {
                (start_idx, end_idx)
            } else {
                (end_idx, start_idx)
            };
            self.content.remove(s..e);

            if start_idx < end_idx {
                self.cursor_row = start.0;
                self.cursor_col = start.1;
            }
            self.selection_start = None;
            self.modified = true;
        }
    }

    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        let last_row = self.content.len_lines() - 1;
        let last_col = self.content.line(last_row).len_chars();
        self.cursor_row = last_row;
        self.cursor_col =
            if last_col > 0 && self.content.line(last_row).chars().last() == Some('\n') {
                last_col - 1
            } else {
                last_col
            };
    }

    pub fn select_line(&mut self) {
        self.selection_start = Some((self.cursor_row, 0));
        let line_len = self.content.line(self.cursor_row).len_chars();
        self.cursor_col =
            if line_len > 0 && self.content.line(self.cursor_row).chars().last() == Some('\n') {
                line_len - 1
            } else {
                line_len
            };
    }
}
