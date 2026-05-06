use super::EditorBuffer;

impl EditorBuffer {
    pub fn update_autocomplete(&mut self) {
        let prefix = self.get_current_word_prefix();
        if prefix.is_empty() || prefix.len() < 2 {
            self.autocomplete_options.clear();
            self.autocomplete_idx = 0;
            return;
        }

        let words = self.collect_all_words();
        let mut matches: Vec<(String, usize)> = words
            .into_iter()
            .filter(|(w, _)| w.starts_with(&prefix) && w.len() > prefix.len())
            .collect();

        matches.sort_by(|a, b| b.1.cmp(&a.1));

        self.autocomplete_options = matches.into_iter().map(|(w, _)| w).collect();
        self.autocomplete_idx = 0;
        self.show_autocomplete_list = !self.autocomplete_options.is_empty();
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
