use super::EditorBuffer;

impl EditorBuffer {
    pub fn copy(&mut self) {
        if let Some(text) = self.get_selected_text() {
            crate::clipboard::copy(&text);
        } else {
            // When no text is selected, copy the current line (including newline if present),
            // matching standard editor behavior (VS Code, Sublime, etc.)
            let row = self.cursor_row;
            if row < self.content.len_lines() {
                let line_str = self.content.line(row).to_string();
                if !line_str.is_empty() {
                    crate::clipboard::copy(&line_str);
                }
            }
        }
    }

    pub fn paste(&mut self) {
        if let Some(text) = crate::clipboard::paste() {
            // Delegate to raw insertion so the whole paste is one undo step and
            // whitespace is preserved without auto-indent compounding.
            self.insert_text(&text);
        }
    }

    pub fn paste_primary(&mut self) {
        if let Some(text) = crate::clipboard::paste_primary() {
            self.insert_text(&text);
        }
    }

    pub fn cut(&mut self) {
        if self.selection_start.is_some() {
            self.copy();
            self.delete_selection();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_without_selection_copies_current_line() {
        let mut buf = EditorBuffer::new();
        buf.insert_text("first line\nsecond line\nthird line\n");
        buf.cursor_row = 1;
        buf.cursor_col = 3;
        buf.selection_start = None;

        buf.copy();
        let pasted = crate::clipboard::paste();
        assert_eq!(pasted.as_deref(), Some("second line\n"));
    }

    #[test]
    fn test_copy_with_selection_copies_selected_text() {
        let mut buf = EditorBuffer::new();
        buf.insert_text("hello world\n");
        buf.selection_start = Some((0, 0));
        buf.cursor_row = 0;
        buf.cursor_col = 5;

        buf.copy();
        let pasted = crate::clipboard::paste();
        assert_eq!(pasted.as_deref(), Some("hello"));
    }
}
