use super::EditorBuffer;

impl EditorBuffer {
    pub fn update_autocomplete(&mut self) {
        if self.is_large_file || self.is_loading {
            self.autocomplete_options.clear();
            self.autocomplete_idx = 0;
            self.show_autocomplete_list = false;
            return;
        }

        let prefix = self.get_current_word_prefix();
        if prefix.chars().count() < 2 {
            self.autocomplete_options.clear();
            self.autocomplete_idx = 0;
            self.show_autocomplete_list = false;
            return;
        }

        self.autocomplete_options = self.collect_autocomplete_matches(&prefix);
        self.autocomplete_idx = 0;
        self.show_autocomplete_list = self.autocomplete_options.len() > 1;
    }

    pub fn cycle_autocomplete(&mut self, step: isize) {
        if self.autocomplete_options.is_empty() {
            return;
        }
        let total = self.autocomplete_options.len() as isize;
        let next_idx = (self.autocomplete_idx as isize + step).rem_euclid(total);
        self.autocomplete_idx = next_idx as usize;
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

#[cfg(test)]
mod tests {
    use super::EditorBuffer;
    use ropey::Rope;
    use std::path::PathBuf;

    #[test]
    fn large_files_do_not_scan_the_document_for_autocomplete() {
        let mut buffer = EditorBuffer::from_loaded_large_file(
            PathBuf::from("large.txt"),
            Rope::from_str("prefix completion\n"),
        );
        buffer.cursor_col = 3;
        buffer.autocomplete_options = vec!["stale".to_string()];

        buffer.update_autocomplete();

        assert!(buffer.autocomplete_options.is_empty());
        assert_eq!(buffer.autocomplete_idx, 0);
    }

    #[test]
    fn test_autocomplete_multiple_options_and_cycling() {
        let mut buffer = EditorBuffer::new();
        buffer.insert_text("username user_id user_role\nus");
        buffer.cursor_row = 1;
        buffer.cursor_col = 2;

        buffer.update_autocomplete();

        assert_eq!(buffer.autocomplete_options.len(), 3);
        assert!(buffer.show_autocomplete_list);
        assert_eq!(buffer.autocomplete_idx, 0);

        // Cycle forward
        buffer.cycle_autocomplete(1);
        assert_eq!(buffer.autocomplete_idx, 1);

        // Cycle wrap around
        buffer.cycle_autocomplete(2);
        assert_eq!(buffer.autocomplete_idx, 0);

        // Cycle backward
        buffer.cycle_autocomplete(-1);
        assert_eq!(buffer.autocomplete_idx, 2);

        // Accept selected option
        let selected = buffer.autocomplete_options[buffer.autocomplete_idx].clone();
        buffer.accept_autocomplete();
        assert!(!buffer.show_autocomplete_list);
        assert!(buffer.autocomplete_options.is_empty());
        assert_eq!(buffer.line_text(1), selected);
    }

    #[test]
    fn test_autocomplete_single_option_does_not_show_list() {
        let mut buffer = EditorBuffer::new();
        buffer.insert_text("something_unique\nso");
        buffer.cursor_row = 1;
        buffer.cursor_col = 2;

        buffer.update_autocomplete();

        assert_eq!(buffer.autocomplete_options.len(), 1);
        assert!(!buffer.show_autocomplete_list);
    }
}
