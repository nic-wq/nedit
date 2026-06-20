use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::column;
use ropey::Rope;

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
    pub selection_start: Option<(usize, usize)>,
    pub history: Vec<Rope>,
    pub history_idx: usize,
    pub is_read_only: bool,
    pub autocomplete_options: Vec<String>,
    pub autocomplete_idx: usize,
    pub show_autocomplete_list: bool,
    pub syntax_states: Vec<Option<(syntect::parsing::ParseState, syntect::highlighting::HighlightState)>>,
    pub rendered_spans: Vec<Option<Vec<(ratatui::style::Color, String)>>>,
}

impl EditorBuffer {
    pub fn new() -> Self {
        let content = Rope::from_str("");
        Self {
            content: content.clone(),
            path: None,
            cursor_row: 0,
            cursor_col: 0,
            cursor_goal_visual_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            modified: false,
            selection_start: None,
            history: vec![content.clone()],
            history_idx: 0,
            is_read_only: false,
            autocomplete_options: Vec::new(),
            autocomplete_idx: 0,
            show_autocomplete_list: false,
            syntax_states: vec![None; content.len_lines()],
            rendered_spans: vec![None; content.len_lines()],
        }
    }

    pub fn from_path(path: PathBuf) -> anyhow::Result<Self> {
        let content = if path.exists() {
            Rope::from_reader(fs::File::open(&path)?)?
        } else {
            Rope::from_str("")
        };

        Ok(Self {
            content: content.clone(),
            path: Some(path),
            cursor_row: 0,
            cursor_col: 0,
            cursor_goal_visual_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            modified: false,
            selection_start: None,
            history: vec![content.clone()],
            history_idx: 0,
            is_read_only: false,
            autocomplete_options: Vec::new(),
            autocomplete_idx: 0,
            show_autocomplete_list: false,
            syntax_states: vec![None; content.len_lines()],
            rendered_spans: vec![None; content.len_lines()],
        })
    }

    pub fn line_text(&self, row: usize) -> String {
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
            self.modified = false;
        }
        Ok(())
    }

    pub fn line_number_width(&self) -> usize {
        self.content.len_lines().to_string().len() + 2
    }

    pub fn to_char_idx(&self, row: usize, col: usize) -> usize {
        let line_idx = self.content.line_to_char(row);
        line_idx + col
    }

    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
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
                    *words.entry(current_word.clone()).or_insert(0) += 1;
                }
                current_word.clear();
            }
        }

        if current_word.len() > 1 {
            *words.entry(current_word).or_insert(0) += 1;
        }

        words
    }

    pub fn find_matching_bracket(&self) -> Option<(usize, usize)> {
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
            for i in (char_idx + 1)..self.content.len_chars() {
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
            for i in (0..char_idx).rev() {
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

    pub(crate) fn push_history(&mut self) {
        // If the user performs a new action after an undo, we truncate the "future" 
        // history to maintain a linear and predictable undo/redo timeline.
        if self.history_idx < self.history.len() - 1 {
            self.history.truncate(self.history_idx + 1);
        }
        self.history.push(self.content.clone());
        if self.history.len() > 100 {
            self.history.remove(0);
        } else {
            self.history_idx += 1;
        }
    }

    pub fn sync_syntax_states(&mut self, from_row: usize) {
        let line_count = self.content.len_lines();
        if self.syntax_states.len() != line_count {
            self.syntax_states.resize(line_count, None);
        }

        for i in from_row..self.syntax_states.len() {
            self.syntax_states[i] = None;
        }
    }

    pub fn sync_rendered_spans(&mut self, from_row: usize) {
        let line_count = self.content.len_lines();
        if self.rendered_spans.len() != line_count {
            self.rendered_spans.resize(line_count, None);
        }

        for i in from_row..self.rendered_spans.len() {
            self.rendered_spans[i] = None;
        }
    }

    pub fn invalidate_all_rendered_spans(&mut self) {
        for entry in self.rendered_spans.iter_mut() {
            *entry = None;
        }
    }
}
