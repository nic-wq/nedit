use super::EditorBuffer;

impl EditorBuffer {
    pub fn update_autocomplete(&mut self) {
        let prefix = self.get_current_word_prefix();
        if prefix.chars().count() < 2 {
            self.autocomplete_options.clear();
            self.autocomplete_idx = 0;
            return;
        }

        let words = self.collect_all_words();
        let prefix_len = prefix.chars().count();
        let mut matches: Vec<(String, usize)> = words
            .into_iter()
            .filter(|(w, _)| w.starts_with(&prefix) && w.chars().count() > prefix_len)
            .collect();

        // We sort by frequency (the second element of the tuple) to prioritize 
        // the most commonly used words in the current buffer.
        matches.sort_by_key(|b| std::cmp::Reverse(b.1));

        self.autocomplete_options = matches.into_iter().map(|(w, _)| w).collect();
        self.autocomplete_idx = 0;
    }

    pub fn get_current_word_prefix(&self) -> String {
        let mut col = self.cursor_col.min(self.content.line(self.cursor_row).len_chars());
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
        if let Some(opt) = self.autocomplete_options.get(self.autocomplete_idx).cloned() {
            let prefix = self.get_current_word_prefix();
            let prefix_len = prefix.chars().count();
            let suffix: String = opt.chars().skip(prefix_len).collect();
            if !suffix.is_empty() {
                // Single undo step for the whole completion.
                self.insert_text(&suffix);
            }
            self.autocomplete_options.clear();
            self.show_autocomplete_list = false;
        }
    }
}
