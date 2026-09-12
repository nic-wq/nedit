use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::column;
use ropey::Rope;

/// Keep large-file behavior aligned with the existing syntax-highlighting limit.
pub const LARGE_FILE_THRESHOLD: u64 = 5 * 1024 * 1024;

#[derive(Clone)]
pub struct EditorBuffer {
    // We use Ropey because it's optimized for very large files, allowing for
    // O(log n) insertions and deletions while maintaining efficient memory usage.
    pub content: Rope,
    pub path: Option<PathBuf>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    /// Visual column remembered across vertical moves (goal column).
    pub cursor_goal_visual_col: usize,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub modified: bool,
    /// Snapshot of the content the last time the buffer was clean
    /// (open, reload or save). `Rope::clone` is O(1) (copy-on-write via
    /// `Arc`), so keeping this snapshot is cheap and lets us recompute
    /// `modified` by comparison instead of setting `true` on every edit.
    pub(crate) saved_content: Rope,
    /// Explicit syntax override as a file extension (e.g. `"lua"`), used when
    /// the buffer has no path to derive one from — like the live script pane.
    /// `None` means "detect from `path`, fall back to plain text".
    pub syntax_override: Option<String>,
    pub selection_start: Option<(usize, usize)>,
    pub history: Vec<Rope>,
    pub history_idx: usize,
    pub is_read_only: bool,
    pub is_preview: bool,
    /// Large buffers avoid per-line caches and document-wide helper work.
    pub is_large_file: bool,
    /// A buffer is read-only until its background load replaces the placeholder.
    pub is_loading: bool,
    /// Identifies a background load independently of the buffer's current index.
    pub load_request_id: Option<u64>,
    pub autocomplete_options: Vec<String>,
    pub autocomplete_idx: usize,
    pub show_autocomplete_list: bool,
    pub syntax_states: Vec<Option<(syntect::parsing::ParseState, syntect::highlighting::HighlightState)>>,
    pub rendered_spans: Vec<Option<Vec<(ratatui::style::Color, String)>>>,
    pub max_visual_width: Option<usize>,
}

impl Default for EditorBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorBuffer {
    pub fn new() -> Self {
        let content = Rope::from_str("");
        Self::from_content(None, content, false)
    }

    fn from_content(path: Option<PathBuf>, content: Rope, is_large_file: bool) -> Self {
        let line_count = if is_large_file { 0 } else { content.len_lines() };
        Self {
            content: content.clone(),
            saved_content: content.clone(),
            path,
            cursor_row: 0,
            cursor_col: 0,
            cursor_goal_visual_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            modified: false,
            syntax_override: None,
            selection_start: None,
            history: vec![content.clone()],
            history_idx: 0,
            is_read_only: false,
            is_preview: false,
            is_large_file,
            is_loading: false,
            load_request_id: None,
            autocomplete_options: Vec::new(),
            autocomplete_idx: 0,
            show_autocomplete_list: false,
            syntax_states: vec![None; line_count],
            rendered_spans: vec![None; line_count],
            max_visual_width: None,
        }
    }

    pub fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        let content = if path.exists() {
            Rope::from_reader(fs::File::open(&path)?)?
        } else {
            Rope::from_str("")
        };

        Ok(Self::from_content(Some(path), content, false))
    }

    /// Creates the non-editable tab shown while a large file is read off-thread.
    pub(crate) fn loading(path: PathBuf, request_id: u64) -> Self {
        let mut buffer = Self::from_content(Some(path), Rope::from_str(""), true);
        buffer.is_loading = true;
        buffer.is_read_only = true;
        buffer.load_request_id = Some(request_id);
        buffer
    }

    /// Builds a finished large-file buffer without allocating caches per line.
    pub(crate) fn from_loaded_large_file(path: PathBuf, content: Rope) -> Self {
        Self::from_content(Some(path), content, true)
    }

    pub fn line_text(&self, row: usize) -> String {
        let row = row.min(self.content.len_lines().saturating_sub(1));
        let mut line = self.content.line(row).to_string();
        if line.ends_with('\n') {
            line.pop();
        }
        if line.ends_with('\r') {
            line.pop();
        }
        line
    }

    pub fn line_max_char_col(&self, row: usize) -> usize {
        let row = row.min(self.content.len_lines().saturating_sub(1));
        let line_len = self.content.line(row).len_chars();
        let mut max_col = line_len;
        if self.content.line(row).chars().last() == Some('\n') {
            max_col = max_col.saturating_sub(1);
        }
        max_col
    }

    pub fn line_max_visual_col(&self, row: usize) -> usize {
        let line = self.line_text(row);
        column::visual_column_at_char_index(&line, self.line_max_char_col(row))
    }

    pub fn cursor_visual_col(&self) -> usize {
        let line = self.line_text(self.cursor_row);
        column::visual_column_at_char_index(&line, self.cursor_col)
    }

    pub fn sync_cursor_goal_from_position(&mut self) {
        self.cursor_goal_visual_col = self.cursor_visual_col();
    }

    pub fn place_cursor(&mut self, row: usize, char_col: usize) {
        let line_count = self.content.len_lines();
        self.cursor_row = row.min(line_count.saturating_sub(1));
        let max_col = self.line_max_char_col(self.cursor_row);
        self.cursor_col = char_col.min(max_col);
        self.sync_cursor_goal_from_position();
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.path {
            fs::write(path, self.content.to_string())?;
            self.mark_clean();
        }
        Ok(())
    }

    /// Marks the current content as clean (no `*`).
    /// `Rope::clone` is O(1), so this is cheap to call after save/reload/load.
    pub fn mark_clean(&mut self) {
        self.saved_content = self.content.clone();
        self.modified = false;
    }

    /// Recomputes `modified` by comparing against the last clean state.
    ///
    /// Fast by construction:
    /// - `Rope::eq` checks `len_bytes()` first (O(1)), so the vast
    ///   majority of keystrokes (which change the size) never scan the text.
    /// - Only when the sizes match do we walk the chunks with early-exit
    ///   on the first differing byte, without any allocation (`to_string`).
    #[inline]
    pub fn refresh_modified(&mut self) {
        self.modified = self.content != self.saved_content;
    }

    /// Replaces the content and marks it as clean (load/reload/docs).
    /// Also resets the history so undo cannot jump to a stale state.
    pub fn set_content_and_mark_clean(&mut self, content: Rope) {
        self.content = content.clone();
        self.saved_content = content;
        self.modified = false;
        self.history = vec![self.content.clone()];
        self.history_idx = 0;
    }

    /// Replaces the content and recomputes `modified` (e.g. script actions/external undo).
    pub fn set_content_and_refresh(&mut self, content: Rope) {
        self.content = content;
        self.refresh_modified();
        self.push_history();
        self.sync_syntax_states(0);
        self.sync_rendered_spans(0);
        self.invalidate_max_visual_width();
    }

    pub fn line_number_width(&self) -> usize {
        self.content.len_lines().to_string().len() + 2
    }

    pub fn to_char_idx(&self, row: usize, col: usize) -> usize {
        let row = row.min(self.content.len_lines().saturating_sub(1));
        let line_idx = self.content.line_to_char(row);
        let max_col = self.line_max_char_col(row);
        let col = col.min(max_col).min(self.content.len_chars().saturating_sub(line_idx));
        line_idx + col
    }

    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let char_idx = char_idx.min(self.content.len_chars());
        let line = self.content.char_to_line(char_idx);
        let line_start = self.content.line_to_char(line);
        (line, char_idx - line_start)
    }

    pub fn collect_all_words(&self) -> HashMap<String, usize> {
        let mut words = HashMap::new();
        let mut current_word = String::new();

        for c in self.content.chars() {
            if c.is_alphanumeric() || c == '_' {
                current_word.push(c);
            } else {
                if current_word.len() > 1 {
                    if let Some(count) = words.get_mut(&current_word) {
                        *count += 1;
                    } else {
                        words.insert(current_word.clone(), 1);
                    }
                }
                current_word.clear();
            }
        }

        if current_word.len() > 1 {
            if let Some(count) = words.get_mut(&current_word) {
                *count += 1;
            } else {
                words.insert(current_word, 1);
            }
        }

        words
    }

    pub fn find_matching_bracket(&self) -> Option<(usize, usize)> {
        self.find_matching_bracket_with_limit(None)
    }

    /// Finds a bracket pair while optionally bounding the number of characters
    /// inspected in either direction. Large files use this to keep redraws local.
    pub fn find_matching_bracket_with_limit(&self, max_chars: Option<usize>) -> Option<(usize, usize)> {
        let char_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
        if char_idx >= self.content.len_chars() {
            return None;
        }

        let current_char = self.content.char(char_idx);
        let open_chars = ['(', '[', '{'];
        let close_chars = [')', ']', '}'];

        if let Some(pos) = open_chars.iter().position(|&c| c == current_char) {
            let target_close = close_chars[pos];
            let mut depth = 0;
            let end = max_chars
                .map(|limit| char_idx.saturating_add(1 + limit).min(self.content.len_chars()))
                .unwrap_or(self.content.len_chars());
            for i in (char_idx + 1)..end {
                let c = self.content.char(i);
                if c == current_char {
                    depth += 1;
                } else if c == target_close {
                    if depth == 0 {
                        return Some(self.char_to_line_col(i));
                    }
                    depth -= 1;
                }
            }
        } else if let Some(pos) = close_chars.iter().position(|&c| c == current_char) {
            let target_open = open_chars[pos];
            let mut depth = 0;
            let start = max_chars.map(|limit| char_idx.saturating_sub(limit)).unwrap_or(0);
            for i in (start..char_idx).rev() {
                let c = self.content.char(i);
                if c == current_char {
                    depth += 1;
                } else if c == target_open {
                    if depth == 0 {
                        return Some(self.char_to_line_col(i));
                    }
                    depth -= 1;
                }
            }
        }

        None
    }

    /// Commit the current buffer content as a new history entry.
    ///
    /// Call this **after** a successful mutation. `history[history_idx]` is always
    /// a snapshot of a stable content state (including the initial load).
    pub(crate) fn push_history(&mut self) {
        // Drop redo states when branching from the middle of the timeline.
        if self.history_idx + 1 < self.history.len() {
            self.history.truncate(self.history_idx + 1);
        }
        self.history.push(self.content.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.history_idx = self.history.len() - 1;
    }

    /// Keep cursor, scroll and selection inside the current content bounds.
    /// Required after undo/redo, which restore text without restoring caret state.
    pub(crate) fn clamp_cursor_to_content(&mut self) {
        let line_count = self.content.len_lines().max(1);
        self.cursor_row = self.cursor_row.min(line_count - 1);
        let max_col = self.line_max_char_col(self.cursor_row);
        self.cursor_col = self.cursor_col.min(max_col);
        self.sync_cursor_goal_from_position();

        if self.scroll_row >= line_count {
            self.scroll_row = line_count - 1;
        }
        if self.scroll_col > self.cursor_col {
            self.scroll_col = self.cursor_col;
        }

        // Selection anchors are not stored in history; drop them after content rewinds.
        self.selection_start = None;
        self.autocomplete_options.clear();
        self.show_autocomplete_list = false;
    }

    pub fn sync_syntax_states(&mut self, from_row: usize) {
        if self.is_large_file {
            self.syntax_states.clear();
            return;
        }
        let line_count = self.content.len_lines();
        if self.syntax_states.len() != line_count {
            self.syntax_states.resize(line_count, None);
        }

        for i in from_row..self.syntax_states.len() {
            self.syntax_states[i] = None;
        }
    }

    pub fn sync_rendered_spans(&mut self, from_row: usize) {
        if self.is_large_file {
            self.rendered_spans.clear();
            return;
        }
        let line_count = self.content.len_lines();
        if self.rendered_spans.len() != line_count {
            self.rendered_spans.resize(line_count, None);
        }

        for i in from_row..self.rendered_spans.len() {
            self.rendered_spans[i] = None;
        }
    }

    pub fn invalidate_all_rendered_spans(&mut self) {
        if self.is_large_file {
            self.rendered_spans.clear();
            return;
        }
        for entry in self.rendered_spans.iter_mut() {
            *entry = None;
        }
    }

    pub fn update_max_visual_width(&mut self) {
        if self.is_large_file {
            return;
        }
        use super::column::TAB_WIDTH;
        let max = (0..self.content.len_lines())
            .map(|row| {
                self.content
                    .line(row)
                    .chars()
                    .filter(|&c| c != '\n' && c != '\r')
                    .map(|c| if c == '\t' { TAB_WIDTH } else { 1 })
                    .sum()
            })
            .max()
            .unwrap_or(0);
        self.max_visual_width = Some(max);
    }

    pub fn invalidate_max_visual_width(&mut self) {
        self.max_visual_width = None;
    }
}

#[cfg(test)]
mod tests {
    use super::EditorBuffer;
    use ropey::Rope;
    use std::path::PathBuf;

    fn clean_buffer(content: &str) -> EditorBuffer {
        let mut buf = EditorBuffer::new();
        buf.set_content_and_mark_clean(Rope::from_str(content));
        buf
    }

    #[test]
    fn typing_then_deleting_back_to_original_clears_modified() {
        let mut buf = clean_buffer("hello");
        assert!(!buf.modified);
        buf.move_to_line_end();
        buf.insert_char('X');
        assert!(buf.modified);
        assert_eq!(buf.content.to_string(), "helloX");
        buf.delete_backspace();
        assert_eq!(buf.content.to_string(), "hello");
        assert!(
            !buf.modified,
            "content is back to the original but `modified` stayed true"
        );
    }

    #[test]
    fn insert_text_then_delete_selection_back_to_original_clears_modified() {
        let mut buf = clean_buffer("hello");
        buf.move_to_line_end();
        buf.insert_text(" world");
        assert!(buf.modified);
        buf.selection_start = Some((0, 5));
        // The cursor is already at the end after insert_text; select the inserted suffix.
        buf.delete_selection();
        assert_eq!(buf.content.to_string(), "hello");
        assert!(!buf.modified);
    }

    #[test]
    fn same_length_replacement_keeps_modified() {
        let mut buf = clean_buffer("hello");
        buf.move_to_line_end();
        buf.delete_backspace(); // "hell"
        assert!(buf.modified);
        buf.insert_char('o'); // back to the same size through a different path
        assert_eq!(buf.content.to_string(), "hello");
        assert!(!buf.modified);
        // A real same-size replacement must stay dirty.
        buf.delete_backspace(); // "hell"
        buf.insert_char('O'); // "hellO" — same len, different content
        assert_eq!(buf.content.to_string(), "hellO");
        assert!(buf.modified);
    }

    #[test]
    fn undo_to_original_clears_modified_and_redo_dirties_again() {
        let mut buf = clean_buffer("abc");
        buf.move_to_line_end();
        buf.insert_char('z');
        assert!(buf.modified);
        buf.undo();
        assert_eq!(buf.content.to_string(), "abc");
        assert!(!buf.modified);
        buf.redo();
        assert_eq!(buf.content.to_string(), "abcz");
        assert!(buf.modified);
    }

    #[test]
    fn delete_word_then_undo_clears_modified() {
        let mut buf = clean_buffer("hello world");
        buf.move_to_line_end();
        buf.delete_word();
        assert_eq!(buf.content.to_string(), "hello ");
        assert!(buf.modified);
        buf.undo();
        assert_eq!(buf.content.to_string(), "hello world");
        assert!(!buf.modified);
    }

    #[test]
    fn save_marks_clean_and_later_revert_clears_again() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.txt");
        std::fs::write(&path, "hi").unwrap();
        let mut buf = EditorBuffer::from_path(path).unwrap();
        assert!(!buf.modified);
        buf.move_to_line_end();
        buf.insert_char('!');
        assert!(buf.modified);
        buf.save().unwrap();
        assert!(!buf.modified);
        // Edit and revert after saving: must become clean again.
        buf.insert_char('?');
        assert!(buf.modified);
        buf.delete_backspace();
        assert!(!buf.modified);
    }

    #[test]
    fn empty_untitled_buffer_type_and_delete_stays_clean() {
        let mut buf = EditorBuffer::new();
        assert!(!buf.modified);
        buf.insert_char('a');
        assert!(buf.modified);
        buf.delete_backspace();
        assert_eq!(buf.content.to_string(), "");
        assert!(!buf.modified);
    }

    #[test]
    fn loaded_large_file_skips_per_line_caches_and_global_width() {
        let path = PathBuf::from("large.txt");
        let content = Rope::from_str("one\ntwo\nthree\n");
        let mut buffer = EditorBuffer::from_loaded_large_file(path, content);

        assert!(buffer.is_large_file);
        assert!(!buffer.is_loading);
        assert!(buffer.syntax_states.is_empty());
        assert!(buffer.rendered_spans.is_empty());

        buffer.update_max_visual_width();
        assert_eq!(buffer.max_visual_width, None);
    }

    #[test]
    fn bounded_bracket_search_does_not_scan_past_limit() {
        let buffer = clean_buffer("(xxxxxxxx)");
        assert_eq!(buffer.find_matching_bracket_with_limit(Some(3)), None);
        assert_eq!(buffer.find_matching_bracket_with_limit(Some(9)), Some((0, 9)));
    }
}
