use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use ropey::Rope;

pub struct EditorBuffer {
    pub content: Rope,
    pub path: Option<PathBuf>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub scroll_col: usize,
    pub selection_start: Option<(usize, usize)>,
    pub history: Vec<Rope>,
    pub history_idx: usize,
    pub modified: bool,
    pub is_read_only: bool,
    #[allow(dead_code)]
    pub diagnostics: Vec<(usize, String)>,
    pub autocomplete_options: Vec<String>,
    pub autocomplete_idx: usize,
    pub show_autocomplete_list: bool,
}

impl EditorBuffer {
    pub fn new() -> Self {
        let rope = Rope::new();
        Self {
            content: rope.clone(),
            path: None,
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            selection_start: None,
            history: vec![rope],
            history_idx: 0,
            modified: false,
            is_read_only: false,
            diagnostics: Vec::new(),
            autocomplete_options: Vec::new(),
            autocomplete_idx: 0,
            show_autocomplete_list: false,
        }
    }

    pub fn from_file(path: PathBuf) -> anyhow::Result<Self> {
        let text = fs::read_to_string(&path)?;
        let rope = Rope::from_str(&text);
        Ok(Self {
            content: rope.clone(),
            path: Some(path),
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            selection_start: None,
            history: vec![rope],
            history_idx: 0,
            modified: false,
            is_read_only: false,
            diagnostics: Vec::new(),
            autocomplete_options: Vec::new(),
            autocomplete_idx: 0,
            show_autocomplete_list: false,
        })
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.path {
            fs::write(path, self.content.to_string())?;
            self.modified = false;
        }
        Ok(())
    }

    pub(crate) fn push_history(&mut self) {
        if self.history_idx < self.history.len() - 1 {
            self.history.truncate(self.history_idx + 1);
        }
        self.history.push(self.content.clone());
        self.history_idx += 1;
    }

    pub(crate) fn to_char_idx(&self, row: usize, col: usize) -> usize {
        let line_idx = row.min(self.content.len_lines().saturating_sub(1));
        let line = self.content.line(line_idx);
        let line_len = line.len_chars();
        let col_idx = col.min(if line_len > 0 && line.chars().last() == Some('\n') {
            line_len - 1
        } else {
            line_len
        });
        self.content.line_to_char(line_idx) + col_idx
    }

    pub(crate) fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let row = self.content.char_to_line(char_idx);
        let col = char_idx - self.content.line_to_char(row);
        (row, col)
    }

    pub(crate) fn collect_all_words(&self) -> HashMap<String, usize> {
        let mut words = HashMap::new();
        for line in self.content.lines() {
            let line_str = line.to_string();
            let mut current_word = String::new();
            for c in line_str.chars() {
                if c.is_alphanumeric() || c == '_' {
                    current_word.push(c);
                } else if !current_word.is_empty() {
                    *words.entry(current_word.clone()).or_insert(0) += 1;
                    current_word.clear();
                }
            }
            if !current_word.is_empty() {
                *words.entry(current_word).or_insert(0) += 1;
            }
        }
        words
    }
}
