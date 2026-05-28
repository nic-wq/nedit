use super::column;
use super::EditorBuffer;

impl EditorBuffer {
    pub fn move_cursor(&mut self, dr: isize, dc: isize, width: usize) {
        let line_count = self.content.len_lines();
        if line_count == 0 {
            return;
        }

        if dr != 0 {
            let new_row = (self.cursor_row as isize + dr).max(0) as usize;
            self.cursor_row = new_row.min(line_count - 1);
            self.apply_vertical_cursor_column();
        } else if dc != 0 {
            let max_col = self.line_max_char_col(self.cursor_row);
            let new_col = (self.cursor_col as isize + dc).max(0) as usize;
            self.cursor_col = new_col.min(max_col);
            self.sync_cursor_goal_from_position();
        }

        self.adjust_horizontal_scroll(width);

        if dr != 0 || dc != 0 {
            self.autocomplete_options.clear();
            self.show_autocomplete_list = false;
        }
    }

    fn apply_vertical_cursor_column(&mut self) {
        let line = self.line_text(self.cursor_row);
        let max_visual = self.line_max_visual_col(self.cursor_row);
        let target_visual = self.cursor_goal_visual_col.min(max_visual);
        self.cursor_col = column::char_index_from_visual_column(&line, target_visual);
    }

    fn adjust_horizontal_scroll(&mut self, width: usize) {
        if width > 5 {
            let edit_width = width - 5;
            if self.cursor_col < self.scroll_col {
                self.scroll_col = self.cursor_col;
            } else if self.cursor_col >= self.scroll_col + edit_width {
                self.scroll_col = self.cursor_col - edit_width + 1;
            }
        }
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor_col = 0;
        self.cursor_goal_visual_col = 0;
        self.autocomplete_options.clear();
    }

    pub fn move_to_line_end(&mut self) {
        self.cursor_col = self.line_max_char_col(self.cursor_row);
        self.sync_cursor_goal_from_position();
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
        self.sync_cursor_goal_from_position();
    }
}

#[cfg(test)]
mod tests {
    use super::super::EditorBuffer;
    use ropey::Rope;

    fn buffer_with(content: &str) -> EditorBuffer {
        let mut buf = EditorBuffer::new();
        buf.content = Rope::from_str(content);
        buf.syntax_states = vec![None; buf.content.len_lines()];
        buf
    }

    #[test]
    fn vertical_move_preserves_goal_column() {
        let mut buf = buffer_with("short\nverylongline\n");
        buf.cursor_row = 1;
        buf.cursor_col = 10;
        buf.sync_cursor_goal_from_position();

        buf.move_cursor(-1, 0, 80);
        assert_eq!(buf.cursor_row, 0);
        assert_eq!(buf.cursor_col, 5);
        assert_eq!(buf.cursor_goal_visual_col, 10);

        buf.move_cursor(1, 0, 80);
        assert_eq!(buf.cursor_row, 1);
        assert_eq!(buf.cursor_col, 10);
        assert_eq!(buf.cursor_goal_visual_col, 10);
    }

    #[test]
    fn horizontal_move_updates_goal_column() {
        let mut buf = buffer_with("hello\nworld\n");
        buf.move_cursor(0, 3, 80);
        assert_eq!(buf.cursor_col, 3);
        assert_eq!(buf.cursor_goal_visual_col, 3);

        buf.move_cursor(1, 0, 80);
        assert_eq!(buf.cursor_col, 3);
        assert_eq!(buf.cursor_goal_visual_col, 3);
    }
}
