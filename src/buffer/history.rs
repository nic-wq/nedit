use super::EditorBuffer;

impl EditorBuffer {
    pub fn undo(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            // Ropey clone is O(1) (copy-on-write), so full-state undo stays cheap.
            self.content = self.history[self.history_idx].clone();
            self.modified = true;
            // History only stores text, not caret position — clamp or render/edit will panic
            // when the restored content is shorter than the previous cursor location.
            self.clamp_cursor_to_content();
            self.sync_syntax_states(0);
            self.sync_rendered_spans(0);
            self.invalidate_max_visual_width();
        }
    }

    pub fn redo(&mut self) {
        if self.history_idx + 1 < self.history.len() {
            self.history_idx += 1;
            self.content = self.history[self.history_idx].clone();
            self.modified = true;
            self.clamp_cursor_to_content();
            self.sync_syntax_states(0);
            self.sync_rendered_spans(0);
            self.invalidate_max_visual_width();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::EditorBuffer;

    #[test]
    fn undo_restores_previous_text() {
        let mut buf = EditorBuffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        assert_eq!(buf.content.to_string(), "ab");

        buf.undo();
        assert_eq!(buf.content.to_string(), "a");
        buf.undo();
        assert_eq!(buf.content.to_string(), "");
        buf.undo(); // no-op at origin
        assert_eq!(buf.content.to_string(), "");
    }

    #[test]
    fn redo_restores_undone_text() {
        let mut buf = EditorBuffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        buf.undo();
        buf.redo();
        assert_eq!(buf.content.to_string(), "a");
        buf.redo();
        assert_eq!(buf.content.to_string(), "ab");
    }

    #[test]
    fn undo_clamps_cursor_when_content_shrinks() {
        let mut buf = EditorBuffer::new();
        buf.insert_char('a');
        buf.insert_char('\n');
        buf.insert_char('b');
        buf.insert_char('\n');
        buf.insert_char('c');
        assert_eq!(buf.cursor_row, 2);

        // Rewind past the newlines; cursor must not stay on a missing line.
        buf.undo(); // remove 'c'
        buf.undo(); // remove second '\n'
        buf.undo(); // remove 'b'
        buf.undo(); // remove first '\n' -> content "a"
        assert_eq!(buf.content.to_string(), "a");
        assert!(
            buf.cursor_row < buf.content.len_lines(),
            "cursor_row {} out of bounds for {} lines",
            buf.cursor_row,
            buf.content.len_lines()
        );
        // Must not panic: line_to_char / line access used by render and edits.
        let _ = buf.to_char_idx(buf.cursor_row, buf.cursor_col);
        let _ = buf.line_text(buf.cursor_row);
        let _ = buf.cursor_visual_col();
    }

    #[test]
    fn undo_then_edit_discards_redo_branch() {
        let mut buf = EditorBuffer::new();
        buf.insert_char('a');
        buf.insert_char('b');
        buf.undo();
        assert_eq!(buf.content.to_string(), "a");
        buf.insert_char('c');
        assert_eq!(buf.content.to_string(), "ac");
        buf.redo(); // nothing to redo
        assert_eq!(buf.content.to_string(), "ac");
    }
}
