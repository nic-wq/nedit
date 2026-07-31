use super::EditorBuffer;

impl EditorBuffer {
    pub fn get_selected_text(&self) -> Option<String> {
        if let Some(start) = self.selection_start {
            let start_idx = self.to_char_idx(start.0, start.1);
            let end_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
            // We normalize the selection range to ensure start < end, 
            // allowing the user to select text in both directions (forward and backward).
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

    /// Remove the selected range without recording history.
    /// Used when the deletion is part of a larger edit (type-over, paste)
    /// so the whole operation becomes a single undo step.
    pub(crate) fn clear_selection_content(&mut self) -> bool {
        let Some(start) = self.selection_start.take() else {
            return false;
        };
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
            self.sync_cursor_goal_from_position();
        }
        true
    }

    pub fn delete_selection(&mut self) {
        if self.clear_selection_content() {
            self.modified = true;
            self.push_history();
            self.sync_syntax_states(self.cursor_row);
            self.sync_rendered_spans(self.cursor_row);
            self.invalidate_max_visual_width();
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
        self.sync_cursor_goal_from_position();
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
        self.sync_cursor_goal_from_position();
    }

    pub fn get_word_at_cursor(&self) -> Option<String> {
        if self.content.len_chars() == 0 {
            return None;
        }
        let char_idx = self
            .to_char_idx(self.cursor_row, self.cursor_col)
            .min(self.content.len_chars().saturating_sub(1));
        let c = self.content.char(char_idx);

        if !(c.is_alphanumeric() || c == '_') {
            return None;
        }

        let mut start_idx = char_idx;
        let mut end_idx = char_idx;

        while start_idx > 0 {
            let c = self.content.char(start_idx - 1);
            if c.is_alphanumeric() || c == '_' {
                start_idx -= 1;
            } else {
                break;
            }
        }
        while end_idx < self.content.len_chars() {
            let c = self.content.char(end_idx);
            if c.is_alphanumeric() || c == '_' {
                end_idx += 1;
            } else {
                break;
            }
        }

        if start_idx != end_idx {
            Some(self.content.slice(start_idx..end_idx).to_string())
        } else {
            None
        }
    }

    pub fn select_word(&mut self) {
        if self.content.len_chars() == 0 {
            return;
        }
        let char_idx = self
            .to_char_idx(self.cursor_row, self.cursor_col)
            .min(self.content.len_chars().saturating_sub(1));
        let c = self.content.char(char_idx);

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let is_whitespace = |c: char| c.is_whitespace();

        let mut start_idx = char_idx;
        let mut end_idx = char_idx;

        if is_word_char(c) {
            while start_idx > 0 && is_word_char(self.content.char(start_idx - 1)) {
                start_idx -= 1;
            }
            while end_idx < self.content.len_chars() && is_word_char(self.content.char(end_idx)) {
                end_idx += 1;
            }
        } else if is_whitespace(c) {
            while start_idx > 0 && is_whitespace(self.content.char(start_idx - 1)) {
                start_idx -= 1;
            }
            while end_idx < self.content.len_chars() && is_whitespace(self.content.char(end_idx)) {
                end_idx += 1;
            }
        } else {
            end_idx += 1;
        }

        if start_idx != end_idx {
            let (start_row, start_col) = self.char_to_line_col(start_idx);
            let (end_row, end_col) = self.char_to_line_col(end_idx);
            self.selection_start = Some((start_row, start_col));
            self.cursor_row = end_row;
            self.cursor_col = end_col;
            self.sync_cursor_goal_from_position();
        }
    }
}
