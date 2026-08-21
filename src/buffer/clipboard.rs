use super::EditorBuffer;

impl EditorBuffer {
    pub fn copy(&mut self) {
        if let Some(text) = self.get_selected_text() {
            crate::clipboard::copy(&text);
        }
    }

    pub fn paste(&mut self) {
        if let Some(text) = crate::clipboard::paste() {
            // Delegate to raw insertion so the whole paste is one undo step and
            // whitespace is preserved without auto-indent compounding.
            self.insert_text(&text);
        }
    }

    pub fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }
}
