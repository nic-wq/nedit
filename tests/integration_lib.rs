#[cfg(test)]
mod tests {
    use nedit::buffer::EditorBuffer;
    use nedit::config::Config;

    #[test]
    fn test_config_default_load() {
        // Caminho feliz: garantindo que a configuração padrão carregue sem pânico
        let config = Config::default();
        assert!(config.autocomplete_enabled);
        assert_eq!(config.get_keybind("quit"), "ctrl+q");
    }

    #[test]
    fn test_buffer_creation_happy_path() {
        // Organizar e Agir
        let buffer = EditorBuffer::new();

        // Assertar
        assert_eq!(buffer.content.to_string(), "");
        assert!(!buffer.modified);
        assert_eq!(buffer.cursor_row, 0);
    }

    #[test]
    fn test_buffer_line_width_calculation() {
        let mut buffer = EditorBuffer::new();
        // Caminho feliz com 1 linha
        assert_eq!(buffer.line_number_width(), 3); // "1".len() + 2

        // Adicionando muitas linhas
        let content: String = (0..100).map(|_| "\n").collect();
        buffer.content = ropey::Rope::from_str(&content);
        // "101".len() + 2 = 5
        assert_eq!(buffer.line_number_width(), 5);
    }
}
