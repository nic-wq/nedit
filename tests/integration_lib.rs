#[cfg(test)]
mod tests {
    use nedit::buffer::EditorBuffer;
    use nedit::config::Config;

    #[test]
    fn test_config_default_load() {
        // Happy path: ensuring the default config loads without panic
        let config = Config::default();
        assert!(config.autocomplete_enabled);
        assert_eq!(config.get_keybind("quit"), "ctrl+q");
    }

    #[test]
    fn test_buffer_creation_happy_path() {
        // Arrange & Act
        let buffer = EditorBuffer::new();

        // Assert
        assert_eq!(buffer.content.to_string(), "");
        assert!(!buffer.modified);
        assert_eq!(buffer.cursor_row, 0);
    }

    #[test]
    fn test_buffer_line_width_calculation() {
        let mut buffer = EditorBuffer::new();
        // Happy path with 1 line
        assert_eq!(buffer.line_number_width(), 3); // "1".len() + 2

        // Adicionando muitas linhas
        let content: String = (0..100).map(|_| "\n").collect();
        buffer.content = ropey::Rope::from_str(&content);
        // "101".len() + 2 = 5
        assert_eq!(buffer.line_number_width(), 5);
    }
}
