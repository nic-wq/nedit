use super::EditorBuffer;

impl EditorBuffer {
    pub fn insert_char(&mut self, ch: char) {
        // A newline shifts the current line and every following line down by
        // one row. Keep the original row so those cached render/syntax states
        // are invalidated, including the newly empty line at the cursor.
        let edited_row = self.cursor_row;
        let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;

        let indent = if ch == '\n' {
            let line = self.content.line(self.cursor_row);
            let mut indentation = String::new();
            for c in line.chars() {
                if c == ' ' || c == '\t' {
                    indentation.push(c);
                } else {
                    break;
                }
            }
            indentation
        } else {
            String::new()
        };

        self.content.insert_char(char_idx, ch);

        if ch == '\n' {
            self.cursor_row += 1;
            self.cursor_col = 0;

            if !indent.is_empty() {
                let new_char_idx = self.content.line_to_char(self.cursor_row);
                self.content.insert(new_char_idx, &indent);
                self.cursor_col = indent.chars().count();
            }

            self.autocomplete_options.clear();
        } else {
            self.cursor_col += 1;
        }
        self.sync_cursor_goal_from_position();
        self.modified = true;
        self.push_history();
        self.sync_syntax_states(edited_row);
        self.sync_rendered_spans(edited_row);
        self.invalidate_max_visual_width();
    }

    pub fn delete_backspace(&mut self) {
        let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
        if char_idx > 0 {
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
            self.sync_cursor_goal_from_position();
            self.modified = true;
            self.push_history();
            self.sync_syntax_states(self.cursor_row);
            self.sync_rendered_spans(self.cursor_row);
            self.invalidate_max_visual_width();
        }
    }

    pub fn delete_word(&mut self) {
        if self.selection_start.is_some() {
            self.delete_selection();
            return;
        }

        let end_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
        if end_idx == 0 {
            return;
        }

        self.move_word(-1);
        let start_idx = self.to_char_idx(self.cursor_row, self.cursor_col);

        self.content.remove(start_idx..end_idx);
        self.modified = true;
        self.push_history();
        self.sync_syntax_states(self.cursor_row);
        self.sync_rendered_spans(self.cursor_row);
        self.invalidate_max_visual_width();
    }
}

#[cfg(test)]
mod tests {
    use super::super::EditorBuffer;
    use ropey::Rope;

    #[test]
    fn inserting_newline_at_line_start_invalidates_shifted_render_cache() {
        let mut buf = EditorBuffer::new();
        buf.content = Rope::from_str("olá");
        buf.syntax_states = vec![None; buf.content.len_lines()];
        buf.rendered_spans = vec![Some(Vec::new()); buf.content.len_lines()];
        buf.cursor_row = 0;
        buf.cursor_col = 0;

        buf.insert_char('\n');

        assert_eq!(buf.content.to_string(), "\nolá");
        assert_eq!(buf.rendered_spans.len(), 2);
        assert!(buf.rendered_spans.iter().all(Option::is_none));
    }
}
