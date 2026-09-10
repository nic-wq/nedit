// Single-line text input model shared by every search/text field that is
// not the main editor (explorer search bar, ...).
//
// The cursor and the selection anchor are char indices, so unicode input can
// never split a char. Selection follows standard editor semantics: plain
// arrows collapse towards the movement direction, Shift extends, typing and
// deletion replace the selection.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineInput {
    pub text: String,
    /// Cursor as a char index (0 = before the first char).
    pub cursor: usize,
    /// Selection anchor as a char index; `None` means no selection.
    pub anchor: Option<usize>,
}

impl LineInput {
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the whole text, moving the cursor to the end.
    pub fn set_text(&mut self, text: String) {
        self.text = text;
        self.cursor = self.len_chars();
        self.anchor = None;
    }

    pub fn clear(&mut self) {
        self.set_text(String::new());
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub fn len_chars(&self) -> usize {
        self.text.chars().count()
    }

    /// Byte offset of a char index (clamped, always a char boundary).
    pub fn byte_idx(&self, char_idx: usize) -> usize {
        let char_idx = char_idx.min(self.len_chars());
        self.text
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or_else(|| self.text.len())
    }

    /// Clamp cursor and anchor into bounds; collapses degenerate selections.
    /// Call after external writes that bypass `set_text`.
    pub fn clamp(&mut self) {
        let len = self.len_chars();
        self.cursor = self.cursor.min(len);
        if let Some(anchor) = self.anchor {
            let anchor = anchor.min(len);
            self.anchor = if anchor == self.cursor {
                None
            } else {
                Some(anchor)
            };
        }
    }

    /// Ordered (start, end) char range of the active selection, if any.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.anchor?.min(self.len_chars());
        let cursor = self.cursor.min(self.len_chars());
        if anchor == cursor {
            None
        } else if anchor < cursor {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
        }
    }

    /// Same range as byte offsets, for slicing and rendering.
    pub fn selection_bytes(&self) -> Option<(usize, usize)> {
        self.selection_range()
            .map(|(s, e)| (self.byte_idx(s), self.byte_idx(e)))
    }

    /// Delete the selected range, placing the cursor at its start.
    pub fn delete_selection(&mut self) -> bool {
        if let Some((s, e)) = self.selection_range() {
            let (bs, be) = (self.byte_idx(s), self.byte_idx(e));
            self.text.drain(bs..be);
            // Removing [s, e) shifts everything after `e` left, so the
            // deletion point keeps char index `s`.
            self.cursor = s;
            self.anchor = None;
            true
        } else {
            false
        }
    }

    /// Insert text at the cursor, replacing any selection.
    pub fn insert_str(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.delete_selection();
        let byte_idx = self.byte_idx(self.cursor);
        self.text.insert_str(byte_idx, text);
        self.cursor += text.chars().count();
        self.anchor = None;
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let end = self.byte_idx(self.cursor);
        let start = self.byte_idx(self.cursor - 1);
        self.text.drain(start..end);
        self.cursor -= 1;
        self.anchor = None;
    }

    pub fn delete_fwd(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor >= self.len_chars() {
            return;
        }
        let start = self.byte_idx(self.cursor);
        let end = self.byte_idx(self.cursor + 1);
        self.text.drain(start..end);
        self.anchor = None;
    }

    /// Core movement: with `extend`, grow/shrink from the anchor; without
    /// it, collapse any selection (callers collapse towards their direction
    /// first, then delegate here for the plain move).
    fn move_to(&mut self, pos: usize, extend: bool) {
        self.clamp();
        let pos = pos.min(self.len_chars());
        if extend {
            if self.anchor.is_none() {
                self.anchor = Some(self.cursor);
            }
            self.cursor = pos;
            if self.anchor == Some(self.cursor) {
                self.anchor = None;
            }
        } else {
            self.cursor = pos;
            self.anchor = None;
        }
    }

    pub fn move_left(&mut self, extend: bool) {
        if !extend {
            if let Some((s, _)) = self.selection_range() {
                self.cursor = s;
                self.anchor = None;
                return;
            }
        }
        let pos = self.cursor.saturating_sub(1);
        self.move_to(pos, extend);
    }

    pub fn move_right(&mut self, extend: bool) {
        if !extend {
            if let Some((_, e)) = self.selection_range() {
                self.cursor = e;
                self.anchor = None;
                return;
            }
        }
        let pos = (self.cursor + 1).min(self.len_chars());
        self.move_to(pos, extend);
    }

    pub fn home(&mut self, extend: bool) {
        if !extend {
            if let Some((s, _)) = self.selection_range() {
                self.cursor = s;
                self.anchor = None;
                return;
            }
        }
        self.move_to(0, extend);
    }

    pub fn end(&mut self, extend: bool) {
        if !extend {
            if let Some((_, e)) = self.selection_range() {
                self.cursor = e;
                self.anchor = None;
                return;
            }
        }
        self.move_to(self.len_chars(), extend);
    }

    pub fn select_all(&mut self) {
        if self.len_chars() == 0 {
            self.anchor = None;
            return;
        }
        self.anchor = Some(0);
        self.cursor = self.len_chars();
    }

    fn is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    /// Previous word start (skip separators leftwards, then the word itself).
    fn word_start(&self, from: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = from.min(chars.len());
        while i > 0 && !Self::is_word_char(chars[i - 1]) {
            i -= 1;
        }
        while i > 0 && Self::is_word_char(chars[i - 1]) {
            i -= 1;
        }
        i
    }

    /// Next word start (skip the current word, then separators).
    fn word_end(&self, from: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut i = from.min(chars.len());
        while i < chars.len() && Self::is_word_char(chars[i]) {
            i += 1;
        }
        while i < chars.len() && !Self::is_word_char(chars[i]) {
            i += 1;
        }
        i
    }

    pub fn move_word(&mut self, dir: isize, extend: bool) {
        if !extend && self.selection_range().is_some() {
            if dir < 0 {
                self.move_left(false);
            } else {
                self.move_right(false);
            }
            return;
        }
        let pos = if dir < 0 {
            self.word_start(self.cursor)
        } else {
            self.word_end(self.cursor)
        };
        self.move_to(pos, extend);
    }

    pub fn delete_word_back(&mut self) {
        if self.delete_selection() {
            return;
        }
        let start = self.word_start(self.cursor);
        if start == self.cursor {
            return;
        }
        let (s, e) = (self.byte_idx(start), self.byte_idx(self.cursor));
        self.text.drain(s..e);
        self.cursor = start;
        self.anchor = None;
    }

    pub fn delete_word_fwd(&mut self) {
        if self.delete_selection() {
            return;
        }
        let end = self.word_end(self.cursor);
        if end == self.cursor {
            return;
        }
        let (s, e) = (self.byte_idx(self.cursor), self.byte_idx(end));
        self.text.drain(s..e);
        self.anchor = None;
    }

    /// Ctrl+U: delete everything before the cursor.
    pub fn kill_to_start(&mut self) {
        self.delete_selection();
        let end = self.byte_idx(self.cursor);
        self.text.drain(..end);
        self.cursor = 0;
        self.anchor = None;
    }

    /// Ctrl+K: delete everything from the cursor onwards.
    pub fn kill_to_end(&mut self) {
        if self.delete_selection() {
            return;
        }
        let start = self.byte_idx(self.cursor);
        self.text.truncate(start);
        self.anchor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::LineInput;

    fn input(text: &str) -> LineInput {
        let mut input = LineInput::new();
        input.set_text(text.to_string());
        input
    }

    #[test]
    fn insert_and_delete_around_cursor() {
        let mut input = input("ac");
        input.move_left(false);
        input.insert_str("b");
        assert_eq!(input.text, "abc");
        assert_eq!(input.cursor, 2);
        input.backspace();
        assert_eq!(input.text, "ac");
        input.delete_fwd();
        assert_eq!(input.text, "a");
    }

    #[test]
    fn typing_replaces_selection() {
        let mut input = input("hello");
        input.select_all();
        input.insert_str("hi");
        assert_eq!(input.text, "hi");
        assert_eq!(input.selection_range(), None);
    }

    #[test]
    fn arrows_collapse_selection_towards_direction() {
        let mut input = input("abc");
        input.move_left(false);
        input.move_right(true);
        assert_eq!(input.selection_range(), Some((2, 3)));
        input.move_left(false);
        assert_eq!(input.cursor, 2);
        assert_eq!(input.selection_range(), None);
        input.home(true);
        assert_eq!(input.selection_range(), Some((0, 2)));
        input.end(false);
        assert_eq!(input.cursor, 2);
    }

    #[test]
    fn word_jumps_and_kills() {
        let mut input = input("src/main file");
        input.move_word(-1, false);
        assert_eq!(input.cursor, 9);
        input.move_word(-1, false);
        assert_eq!(input.cursor, 4);
        input.move_word(1, false);
        assert_eq!(input.cursor, 9);
        input.delete_word_back();
        assert_eq!(input.text, "src/file");
        assert_eq!(input.cursor, 4);
        input.kill_to_end();
        assert_eq!(input.text, "src/");
        input.kill_to_start();
        assert_eq!(input.text, "");
    }

    #[test]
    fn unicode_never_splits_chars() {
        let mut input = input("aé😀b");
        assert_eq!(input.len_chars(), 4);
        input.move_left(false);
        input.move_left(false);
        input.backspace();
        assert_eq!(input.text, "a😀b");
        for i in 0..=input.len_chars() {
            assert!(input.text.is_char_boundary(input.byte_idx(i)));
        }
    }

    #[test]
    fn deleting_mid_selection_keeps_deletion_point() {
        let mut input = input("abcde");
        input.home(false);
        input.move_right(false);
        input.move_right(false);
        input.move_right(true);
        input.move_right(true);
        assert_eq!(input.selection_range(), Some((2, 4)));
        assert!(input.delete_selection());
        assert_eq!(input.text, "abe");
        assert_eq!(input.cursor, 2);
    }
}
