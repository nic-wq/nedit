use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use ropey::Rope;

#[derive(Clone)]
pub struct EditorBuffer {
    pub content: Rope,
    pub path: Option<PathBuf>,
    pub cursor_row: usize,
    pub cursor_col: usize,
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
}

impl EditorBuffer {
    pub fn new() -> Self {
        let content = Rope::from_str("");
        Self {
            content: content.clone(),
            path: None,
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            scroll_col: 0,
            modified: false,
            selection_start: None,
            history: vec![content],
            history_idx: 0,
            is_read_only: false,
            autocomplete_options: Vec::new(),
            autocomplete_idx: 0,
            show_autocomplete_list: false,
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
            scroll_row: 0,
            scroll_col: 0,
            modified: false,
            selection_start: None,
            history: vec![content],
            history_idx: 0,
            is_read_only: false,
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
        let content_str = self.content.to_string();
        for word in content_str
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|w| w.len() > 1)
        {
            *words.entry(word.to_string()).or_insert(0) += 1;
        }
        words
    }

    pub(crate) fn push_history(&mut self) {
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
}
