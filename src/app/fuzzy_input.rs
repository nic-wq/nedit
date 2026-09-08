use super::{App, FuzzyMode};

// Single-line text editing model for every fuzzy modal input.
//
// The query itself stays in `App::fuzzy_query` (read by filtering, saving,
// path creation, ...). The cursor is a char index so unicode input can never
// split a char, and the optional anchor enables Shift-selection exactly like
// the main editor (arrows/Home/End/Ctrl+A, Backspace/Delete aware of it).

impl App {
    /// Modes that show an editable text field.
    /// `UnsavedChanges`, `ExternalChange` and `DeleteConfirm` only react to
    /// shortcut keys, so text editing must not consume keystrokes there.
    pub fn fuzzy_has_editable_input(&self) -> bool {
        !matches!(
            self.fuzzy_mode,
            FuzzyMode::UnsavedChanges | FuzzyMode::ExternalChange | FuzzyMode::DeleteConfirm
        )
    }

    /// Modes whose visible results must refresh while typing.
    /// `Rename`, `SaveAs` and `Create` are plain text fields with no list.
    pub fn fuzzy_is_live_mode(&self) -> bool {
        matches!(
            self.fuzzy_mode,
            FuzzyMode::Files
                | FuzzyMode::Content
                | FuzzyMode::Local
                | FuzzyMode::Themes
                | FuzzyMode::FileOptions
                | FuzzyMode::CommandPalette
                | FuzzyMode::Move
                | FuzzyMode::DocSelect
        )
    }

    /// Replace the whole query (mode switches, prefilled names, ...).
    /// The cursor goes to the end and any selection is dropped.
    pub fn set_fuzzy_query(&mut self, query: String) {
        self.fuzzy_query = query;
        self.fuzzy_cursor = self.fuzzy_len_chars();
        self.fuzzy_sel_anchor = None;
    }

    pub fn clear_fuzzy_query(&mut self) {
        self.set_fuzzy_query(String::new());
    }

    pub fn fuzzy_len_chars(&self) -> usize {
        self.fuzzy_query.chars().count()
    }

    /// Byte offset of a char index (clamped). Used for slicing and rendering.
    pub fn fuzzy_byte_idx(&self, char_idx: usize) -> usize {
        let char_idx = char_idx.min(self.fuzzy_len_chars());
        self.fuzzy_query
            .char_indices()
            .nth(char_idx)
            .map(|(i, _)| i)
            .unwrap_or_else(|| self.fuzzy_query.len())
    }

    /// Clamp cursor and anchor into bounds (defensive: direct `fuzzy_query`
    /// writes from perf harnesses bypass `set_fuzzy_query`).
    pub fn fuzzy_clamp(&mut self) {
        let len = self.fuzzy_len_chars();
        self.fuzzy_cursor = self.fuzzy_cursor.min(len);
        if let Some(anchor) = self.fuzzy_sel_anchor {
            if anchor >= len && anchor != self.fuzzy_cursor {
                self.fuzzy_sel_anchor = Some(anchor.min(len));
            }
            if self.fuzzy_sel_anchor == Some(self.fuzzy_cursor) {
                self.fuzzy_sel_anchor = None;
            }
        }
    }

    /// Ordered (start, end) char range of the active selection, if any.
    pub fn fuzzy_selection_range(&self) -> Option<(usize, usize)> {
        let anchor = self.fuzzy_sel_anchor?;
        let cursor = self.fuzzy_cursor.min(self.fuzzy_len_chars());
        let anchor = anchor.min(self.fuzzy_len_chars());
        if anchor == cursor {
            None
        } else if anchor < cursor {
            Some((anchor, cursor))
        } else {
            Some((cursor, anchor))
        }
    }

    /// Same range as byte offsets, for slicing and rendering.
    pub fn fuzzy_selection_bytes(&self) -> Option<(usize, usize)> {
        self.fuzzy_selection_range()
            .map(|(s, e)| (self.fuzzy_byte_idx(s), self.fuzzy_byte_idx(e)))
    }

    /// Delete the selected range, placing the cursor at its start.
    pub fn fuzzy_delete_selection(&mut self) -> bool {
        if let Some((s, e)) = self.fuzzy_selection_range() {
            let (bs, be) = (self.fuzzy_byte_idx(s), self.fuzzy_byte_idx(e));
            self.fuzzy_query.drain(bs..be);
            // `s` is still valid: removing [s, e) shifts everything after `e`
            // left, so the deletion point keeps char index `s`.
            self.fuzzy_cursor = s;
            self.fuzzy_sel_anchor = None;
            true
        } else {
            false
        }
    }

    /// Insert text at the cursor, replacing any selection.
    pub fn fuzzy_insert_str(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.fuzzy_delete_selection();
        let byte_idx = self.fuzzy_byte_idx(self.fuzzy_cursor);
        self.fuzzy_query.insert_str(byte_idx, text);
        self.fuzzy_cursor += text.chars().count();
        self.fuzzy_sel_anchor = None;
    }

    pub fn fuzzy_backspace(&mut self) {
        if self.fuzzy_delete_selection() {
            return;
        }
        if self.fuzzy_cursor == 0 {
            return;
        }
        let end = self.fuzzy_byte_idx(self.fuzzy_cursor);
        let start = self.fuzzy_byte_idx(self.fuzzy_cursor - 1);
        self.fuzzy_query.drain(start..end);
        self.fuzzy_cursor -= 1;
        self.fuzzy_sel_anchor = None;
    }

    pub fn fuzzy_delete_fwd(&mut self) {
        if self.fuzzy_delete_selection() {
            return;
        }
        if self.fuzzy_cursor >= self.fuzzy_len_chars() {
            return;
        }
        let start = self.fuzzy_byte_idx(self.fuzzy_cursor);
        let end = self.fuzzy_byte_idx(self.fuzzy_cursor + 1);
        self.fuzzy_query.drain(start..end);
        self.fuzzy_sel_anchor = None;
    }

    /// Core movement: with `extend`, grow/shrink the selection from the
    /// anchor; without it, collapse any selection towards `pos` first
    /// (standard editor behavior: plain arrows just collapse).
    fn fuzzy_move_to(&mut self, pos: usize, extend: bool) {
        self.fuzzy_clamp();
        let pos = pos.min(self.fuzzy_len_chars());
        if extend {
            if self.fuzzy_sel_anchor.is_none() {
                self.fuzzy_sel_anchor = Some(self.fuzzy_cursor);
            }
            self.fuzzy_cursor = pos;
            if self.fuzzy_sel_anchor == Some(self.fuzzy_cursor) {
                self.fuzzy_sel_anchor = None;
            }
        } else if self.fuzzy_selection_range().is_some() {
            // Collapse towards the movement direction is handled by callers
            // passing the right edge; default here collapses to `pos`.
            self.fuzzy_cursor = pos;
            self.fuzzy_sel_anchor = None;
        } else {
            self.fuzzy_cursor = pos;
            self.fuzzy_sel_anchor = None;
        }
    }

    pub fn fuzzy_move_left(&mut self, extend: bool) {
        if !extend {
            if let Some((s, _)) = self.fuzzy_selection_range() {
                self.fuzzy_cursor = s;
                self.fuzzy_sel_anchor = None;
                return;
            }
        }
        let pos = self.fuzzy_cursor.saturating_sub(1);
        self.fuzzy_move_to(pos, extend);
    }

    pub fn fuzzy_move_right(&mut self, extend: bool) {
        if !extend {
            if let Some((_, e)) = self.fuzzy_selection_range() {
                self.fuzzy_cursor = e;
                self.fuzzy_sel_anchor = None;
                return;
            }
        }
        let pos = (self.fuzzy_cursor + 1).min(self.fuzzy_len_chars());
        self.fuzzy_move_to(pos, extend);
    }

    pub fn fuzzy_home(&mut self, extend: bool) {
        if !extend {
            if let Some((s, _)) = self.fuzzy_selection_range() {
                self.fuzzy_cursor = s;
                self.fuzzy_sel_anchor = None;
                return;
            }
        }
        self.fuzzy_move_to(0, extend);
    }

    pub fn fuzzy_end(&mut self, extend: bool) {
        if !extend {
            if let Some((_, e)) = self.fuzzy_selection_range() {
                self.fuzzy_cursor = e;
                self.fuzzy_sel_anchor = None;
                return;
            }
        }
        self.fuzzy_move_to(self.fuzzy_len_chars(), extend);
    }

    pub fn fuzzy_select_all(&mut self) {
        if self.fuzzy_len_chars() == 0 {
            self.fuzzy_sel_anchor = None;
            return;
        }
        self.fuzzy_sel_anchor = Some(0);
        self.fuzzy_cursor = self.fuzzy_len_chars();
    }

    fn fuzzy_is_word_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    /// Previous word start from `from` (VSCode/readline style: skip
    /// separators leftwards, then the word itself).
    fn fuzzy_word_start(&self, from: usize) -> usize {
        let chars: Vec<char> = self.fuzzy_query.chars().collect();
        let mut i = from.min(chars.len());
        while i > 0 && !Self::fuzzy_is_word_char(chars[i - 1]) {
            i -= 1;
        }
        while i > 0 && Self::fuzzy_is_word_char(chars[i - 1]) {
            i -= 1;
        }
        i
    }

    /// Next word start from `from` (skip the current word, then separators).
    fn fuzzy_word_end(&self, from: usize) -> usize {
        let chars: Vec<char> = self.fuzzy_query.chars().collect();
        let mut i = from.min(chars.len());
        while i < chars.len() && Self::fuzzy_is_word_char(chars[i]) {
            i += 1;
        }
        while i < chars.len() && !Self::fuzzy_is_word_char(chars[i]) {
            i += 1;
        }
        i
    }

    pub fn fuzzy_move_word(&mut self, dir: isize, extend: bool) {
        if !extend && self.fuzzy_selection_range().is_some() {
            // Collapse towards the movement direction first.
            if dir < 0 {
                self.fuzzy_move_left(false);
            } else {
                self.fuzzy_move_right(false);
            }
            return;
        }
        let pos = if dir < 0 {
            self.fuzzy_word_start(self.fuzzy_cursor)
        } else {
            self.fuzzy_word_end(self.fuzzy_cursor)
        };
        self.fuzzy_move_to(pos, extend);
    }

    pub fn fuzzy_delete_word_back(&mut self) {
        if self.fuzzy_delete_selection() {
            return;
        }
        let start = self.fuzzy_word_start(self.fuzzy_cursor);
        if start == self.fuzzy_cursor {
            return;
        }
        let (s, e) = (
            self.fuzzy_byte_idx(start),
            self.fuzzy_byte_idx(self.fuzzy_cursor),
        );
        self.fuzzy_query.drain(s..e);
        self.fuzzy_cursor = start;
        self.fuzzy_sel_anchor = None;
    }

    pub fn fuzzy_delete_word_fwd(&mut self) {
        if self.fuzzy_delete_selection() {
            return;
        }
        let end = self.fuzzy_word_end(self.fuzzy_cursor);
        if end == self.fuzzy_cursor {
            return;
        }
        let (s, e) = (
            self.fuzzy_byte_idx(self.fuzzy_cursor),
            self.fuzzy_byte_idx(end),
        );
        self.fuzzy_query.drain(s..e);
        self.fuzzy_sel_anchor = None;
    }

    /// Ctrl+U: delete everything before the cursor.
    pub fn fuzzy_kill_to_start(&mut self) {
        self.fuzzy_delete_selection();
        let end = self.fuzzy_byte_idx(self.fuzzy_cursor);
        self.fuzzy_query.drain(..end);
        self.fuzzy_cursor = 0;
        self.fuzzy_sel_anchor = None;
    }

    /// Ctrl+K: delete everything from the cursor onwards.
    pub fn fuzzy_kill_to_end(&mut self) {
        if self.fuzzy_delete_selection() {
            return;
        }
        let start = self.fuzzy_byte_idx(self.fuzzy_cursor);
        self.fuzzy_query.truncate(start);
        self.fuzzy_sel_anchor = None;
    }
}

#[cfg(test)]
mod tests {
    use super::App;

    fn app_with(query: &str) -> App {
        let mut app = App::new(&[]);
        app.set_fuzzy_query(query.to_string());
        app
    }

    #[test]
    fn set_query_places_cursor_at_end() {
        let app = app_with("hello");
        assert_eq!(app.fuzzy_cursor, 5);
        assert_eq!(app.fuzzy_sel_anchor, None);
    }

    #[test]
    fn insert_and_backspace_around_cursor() {
        let mut app = app_with("ac");
        app.fuzzy_move_left(false);
        app.fuzzy_insert_str("b");
        assert_eq!(app.fuzzy_query, "abc");
        assert_eq!(app.fuzzy_cursor, 2);
        app.fuzzy_backspace();
        assert_eq!(app.fuzzy_query, "ac");
        assert_eq!(app.fuzzy_cursor, 1);
        app.fuzzy_delete_fwd();
        assert_eq!(app.fuzzy_query, "a");
        assert_eq!(app.fuzzy_cursor, 1);
    }

    #[test]
    fn typing_replaces_selection() {
        let mut app = app_with("hello");
        app.fuzzy_select_all();
        app.fuzzy_insert_str("hi");
        assert_eq!(app.fuzzy_query, "hi");
        assert_eq!(app.fuzzy_selection_range(), None);
    }

    #[test]
    fn backspace_and_delete_use_selection() {
        let mut app = app_with("hello");
        app.fuzzy_select_all();
        app.fuzzy_backspace();
        assert_eq!(app.fuzzy_query, "");
        assert_eq!(app.fuzzy_cursor, 0);

        let mut app = app_with("hello");
        app.fuzzy_select_all();
        app.fuzzy_delete_fwd();
        assert_eq!(app.fuzzy_query, "");
    }

    #[test]
    fn arrows_move_and_collapse_selection() {
        let mut app = app_with("abc");
        app.fuzzy_move_left(false);
        app.fuzzy_move_left(false);
        assert_eq!(app.fuzzy_cursor, 1);
        // Shift+Right selects one char.
        app.fuzzy_move_right(true);
        assert_eq!(app.fuzzy_selection_range(), Some((1, 2)));
        // Plain Left collapses to the selection start.
        app.fuzzy_move_left(false);
        assert_eq!(app.fuzzy_cursor, 1);
        assert_eq!(app.fuzzy_selection_range(), None);
        // Shift+Home selects to the start.
        app.fuzzy_move_right(false);
        app.fuzzy_home(true);
        assert_eq!(app.fuzzy_selection_range(), Some((0, 2)));
        // Plain End collapses to the selection end.
        app.fuzzy_end(false);
        assert_eq!(app.fuzzy_cursor, 2);
    }

    #[test]
    fn deleting_mid_selection_keeps_cursor_at_deletion_point() {
        let mut app = app_with("abcde");
        // Select "cd" (chars 2..4) via Shift+Right.
        app.fuzzy_home(false);
        app.fuzzy_move_right(false);
        app.fuzzy_move_right(false);
        app.fuzzy_move_right(true);
        app.fuzzy_move_right(true);
        assert_eq!(app.fuzzy_selection_range(), Some((2, 4)));
        assert!(app.fuzzy_delete_selection());
        assert_eq!(app.fuzzy_query, "abe");
        assert_eq!(app.fuzzy_cursor, 2);
        // Reversed selection deletes the same way.
        app.fuzzy_home(false);
        app.fuzzy_move_right(true);
        app.fuzzy_move_right(true);
        assert_eq!(app.fuzzy_selection_range(), Some((0, 2)));
        app.fuzzy_insert_str("XY");
        assert_eq!(app.fuzzy_query, "XYe");
        assert_eq!(app.fuzzy_cursor, 2);
    }

    #[test]
    fn select_all_then_delete_kills_line() {
        let mut app = app_with("some query");
        app.fuzzy_select_all();
        assert_eq!(app.fuzzy_selection_range(), Some((0, 10)));
        assert!(app.fuzzy_delete_selection());
        assert_eq!(app.fuzzy_query, "");
        assert!(!app.fuzzy_delete_selection());
    }

    #[test]
    fn word_jumps_and_deletion() {
        let mut app = app_with("src/main file");
        app.fuzzy_move_word(-1, false);
        assert_eq!(app.fuzzy_cursor, 9);
        app.fuzzy_move_word(-1, false);
        assert_eq!(app.fuzzy_cursor, 4);
        app.fuzzy_move_word(1, false);
        assert_eq!(app.fuzzy_cursor, 9);
        // Ctrl+W style deletion.
        app.fuzzy_delete_word_back();
        assert_eq!(app.fuzzy_query, "src/file");
        assert_eq!(app.fuzzy_cursor, 4);
    }

    #[test]
    fn kill_to_start_and_end() {
        let mut app = app_with("hello world");
        app.fuzzy_home(false);
        for _ in 0..5 {
            app.fuzzy_move_right(false);
        }
        app.fuzzy_kill_to_start();
        assert_eq!(app.fuzzy_query, " world");
        assert_eq!(app.fuzzy_cursor, 0);
        app.fuzzy_kill_to_end();
        assert_eq!(app.fuzzy_query, "");
    }

    #[test]
    fn unicode_never_splits_chars() {
        let mut app = app_with("aé😀b");
        assert_eq!(app.fuzzy_len_chars(), 4);
        app.fuzzy_move_left(false);
        app.fuzzy_move_left(false);
        app.fuzzy_backspace();
        assert_eq!(app.fuzzy_query, "a😀b");
        app.fuzzy_insert_str("é");
        assert_eq!(app.fuzzy_query, "aé😀b");
        // Byte conversion stays on char boundaries.
        for i in 0..=4 {
            let b = app.fuzzy_byte_idx(i);
            assert!(app.fuzzy_query.is_char_boundary(b));
        }
    }

    #[test]
    fn stale_cursor_is_clamped() {
        let mut app = App::new(&[]);
        // Direct writes bypassing set_fuzzy_query (perf harness style).
        app.fuzzy_query = "hi".to_string();
        app.fuzzy_cursor = 99;
        app.fuzzy_sel_anchor = Some(50);
        app.fuzzy_clamp();
        assert_eq!(app.fuzzy_cursor, 2);
        // Degenerate selection collapses.
        app.fuzzy_sel_anchor = Some(2);
        app.fuzzy_clamp();
        assert_eq!(app.fuzzy_sel_anchor, None);
    }
}
