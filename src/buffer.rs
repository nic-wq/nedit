use ropey::Rope;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

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

    pub fn insert_char(&mut self, ch: char) {
        self.push_history();
        let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
        self.content.insert_char(char_idx, ch);
        
        if ch == '\n' {
            self.cursor_row += 1;
            self.cursor_col = 0;
            self.autocomplete_options.clear();
        } else {
            self.cursor_col += 1;
        }
        self.modified = true;
    }

    pub fn delete_backspace(&mut self) {
        let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
        if char_idx > 0 {
            self.push_history();
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
            self.modified = true;
        }
    }

    pub fn move_cursor(&mut self, dr: isize, dc: isize, width: usize) {
        let line_count = self.content.len_lines();
        let new_row = (self.cursor_row as isize + dr).max(0) as usize;
        self.cursor_row = new_row.min(line_count - 1);

        let line_len = self.content.line(self.cursor_row).len_chars();
        let mut max_col = line_len;
        if self.content.line(self.cursor_row).chars().last() == Some('\n') {
            max_col = max_col.saturating_sub(1);
        }
        
        if dc != 0 {
            let new_col = (self.cursor_col as isize + dc).max(0) as usize;
            self.cursor_col = new_col.min(max_col);
        } else {
            self.cursor_col = self.cursor_col.min(max_col);
        }

        // Vertical scroll
        // (Handled in UI usually, but let's keep it here if needed)

        // Horizontal scroll
        if width > 5 {
            let edit_width = width - 5; // Leave room for line numbers
            if self.cursor_col < self.scroll_col {
                self.scroll_col = self.cursor_col;
            } else if self.cursor_col >= self.scroll_col + edit_width {
                self.scroll_col = self.cursor_col - edit_width + 1;
            }
        }

        self.autocomplete_options.clear();
    }

    pub fn move_to_line_start(&mut self) {
        self.cursor_col = 0;
        self.autocomplete_options.clear();
    }

    pub fn move_to_line_end(&mut self) {
        let line_len = self.content.line(self.cursor_row).len_chars();
        self.cursor_col = if line_len > 0 && self.content.line(self.cursor_row).chars().last() == Some('\n') {
            line_len - 1
        } else {
            line_len
        };
        self.autocomplete_options.clear();
    }

    pub fn move_word(&mut self, dir: isize) {
        let mut char_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
        if dir > 0 {
            // Forward
            while char_idx < self.content.len_chars() && !self.content.char(char_idx).is_alphanumeric() {
                char_idx += 1;
            }
            while char_idx < self.content.len_chars() && self.content.char(char_idx).is_alphanumeric() {
                char_idx += 1;
            }
        } else {
            // Backward
            if char_idx > 0 { char_idx -= 1; }
            while char_idx > 0 && !self.content.char(char_idx).is_alphanumeric() {
                char_idx -= 1;
            }
            while char_idx > 0 && self.content.char(char_idx).is_alphanumeric() {
                char_idx -= 1;
            }
            if char_idx > 0 && !self.content.char(char_idx).is_alphanumeric() {
                char_idx += 1;
            }
        }
        let (row, col) = self.char_to_line_col(char_idx);
        self.cursor_row = row;
        self.cursor_col = col;
    }

    fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let row = self.content.char_to_line(char_idx);
        let col = char_idx - self.content.line_to_char(row);
        (row, col)
    }

    fn push_history(&mut self) {
        if self.history_idx < self.history.len() - 1 {
            self.history.truncate(self.history_idx + 1);
        }
        self.history.push(self.content.clone());
        self.history_idx += 1;
    }

    pub fn undo(&mut self) {
        if self.history_idx > 0 {
            self.history_idx -= 1;
            self.content = self.history[self.history_idx].clone();
            self.modified = true;
            // Reset cursor to end for simplicity, or we could track cursor in history
        }
    }

    pub fn redo(&mut self) {
        if self.history_idx < self.history.len() - 1 {
            self.history_idx += 1;
            self.content = self.history[self.history_idx].clone();
            self.modified = true;
        }
    }

    pub fn copy(&mut self) {
        if let Some(text) = self.get_selected_text() {
            crate::clipboard::copy(&text);
        }
    }

    pub fn paste(&mut self) {
        if let Some(text) = crate::clipboard::paste() {
            self.push_history();
            let char_idx = self.content.line_to_char(self.cursor_row) + self.cursor_col;
            self.content.insert(char_idx, &text);
            let new_rope = Rope::from_str(&text);
            let lines = new_rope.len_lines();
            if lines > 1 {
                self.cursor_row += lines - 1;
                self.cursor_col = new_rope.line(lines - 1).len_chars();
            } else {
                self.cursor_col += new_rope.len_chars();
            }
            self.modified = true;
        }
    }

    pub fn cut(&mut self) {
        self.copy();
        self.delete_selection();
    }

    pub fn get_selected_text(&self) -> Option<String> {
        if let Some(start) = self.selection_start {
            let start_idx = self.to_char_idx(start.0, start.1);
            let end_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
            let (s, e) = if start_idx < end_idx { (start_idx, end_idx) } else { (end_idx, start_idx) };
            Some(self.content.slice(s..e).to_string())
        } else {
            None
        }
    }

    pub fn delete_selection(&mut self) {
        if let Some(start) = self.selection_start {
            self.push_history();
            let start_idx = self.to_char_idx(start.0, start.1);
            let end_idx = self.to_char_idx(self.cursor_row, self.cursor_col);
            let (s, e) = if start_idx < end_idx { (start_idx, end_idx) } else { (end_idx, start_idx) };
            self.content.remove(s..e);
            
            // Move cursor to start of deletion
            if start_idx < end_idx {
                self.cursor_row = start.0;
                self.cursor_col = start.1;
            }
            self.selection_start = None;
            self.modified = true;
        }
    }

    pub fn select_all(&mut self) {
        self.selection_start = Some((0, 0));
        let last_row = self.content.len_lines() - 1;
        let last_col = self.content.line(last_row).len_chars();
        self.cursor_row = last_row;
        self.cursor_col = if last_col > 0 && self.content.line(last_row).chars().last() == Some('\n') {
            last_col - 1
        } else {
            last_col
        };
    }

    pub fn select_line(&mut self) {
        self.selection_start = Some((self.cursor_row, 0));
        let line_len = self.content.line(self.cursor_row).len_chars();
        self.cursor_col = if line_len > 0 && self.content.line(self.cursor_row).chars().last() == Some('\n') {
            line_len - 1
        } else {
            line_len
        };
    }

    fn to_char_idx(&self, row: usize, col: usize) -> usize {
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

    pub fn update_autocomplete(&mut self) {
        let prefix = self.get_current_word_prefix();
        if prefix.is_empty() || prefix.len() < 2 {
            self.autocomplete_options.clear();
            self.autocomplete_idx = 0;
            return;
        }

        let words = self.collect_all_words();
        let mut matches: Vec<(String, usize)> = words.into_iter()
            .filter(|(w, _)| w.starts_with(&prefix) && w.len() > prefix.len())
            .collect();
        
        // Sort by frequency (descending)
        matches.sort_by(|a, b| b.1.cmp(&a.1));
        
        self.autocomplete_options = matches.into_iter().map(|(w, _)| w).collect();
        self.autocomplete_idx = 0;
    }

    pub fn get_current_word_prefix(&self) -> String {
        let mut col = self.cursor_col;
        let line = self.content.line(self.cursor_row);
        let mut prefix = String::new();
        while col > 0 {
            let c = line.char(col - 1);
            if c.is_alphanumeric() || c == '_' {
                prefix.insert(0, c);
                col -= 1;
            } else {
                break;
            }
        }
        prefix
    }

    fn collect_all_words(&self) -> HashMap<String, usize> {
        let mut words = std::collections::HashMap::new();
        for line in self.content.lines() {
            let line_str = line.to_string();
            let mut current_word = String::new();
            for c in line_str.chars() {
                if c.is_alphanumeric() || c == '_' {
                    current_word.push(c);
                } else {
                    if !current_word.is_empty() {
                        *words.entry(current_word.clone()).or_insert(0) += 1;
                        current_word.clear();
                    }
                }
            }
            if !current_word.is_empty() {
                *words.entry(current_word).or_insert(0) += 1;
            }
        }
        words
    }

    pub fn accept_autocomplete(&mut self) {
        if let Some(opt) = self.autocomplete_options.get(self.autocomplete_idx) {
            let prefix = self.get_current_word_prefix();
            let suffix = opt[prefix.len()..].to_string();
            for c in suffix.chars() {
                self.insert_char(c);
            }
            self.autocomplete_options.clear();
            self.show_autocomplete_list = false;
        }
    }
}
