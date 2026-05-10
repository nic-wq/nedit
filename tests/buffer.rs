use nedit::buffer::EditorBuffer;
use std::path::PathBuf;

#[test]
fn test_buffer_from_non_existent_path() {
    // Caso de borda: arquivo que não existe deve criar um buffer vazio com o caminho definido
    let path = PathBuf::from("este_arquivo_realmente_nao_deve_existir_12345.txt");
    let buffer = EditorBuffer::from_path(path.clone()).unwrap();

    assert_eq!(buffer.content.to_string(), "");
    assert_eq!(buffer.path, Some(path));
}

#[test]
fn test_buffer_to_char_idx() {
    let mut buffer = EditorBuffer::new();
    buffer.content = ropey::Rope::from_str("linha1\nlinha2\nlinha3");
    
    // Testando a conversão de linha/coluna para índice global
    // "linha1\n" tem 7 chars (considerando \n). 'l' de "linha2" está no índice 7.
    assert_eq!(buffer.to_char_idx(1, 0), 7);
    assert_eq!(buffer.to_char_idx(0, 0), 0);
}

#[test]
fn test_buffer_char_to_line_col() {
    let mut buffer = EditorBuffer::new();
    buffer.content = ropey::Rope::from_str("abc\ndef");
    
    // 'd' está no índice 4
    let (line, col) = buffer.char_to_line_col(4);
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

#[test]
fn test_buffer_collect_words() {
    let mut buffer = EditorBuffer::new();
    buffer.content = ropey::Rope::from_str("ola mundo ola rust_test");

    let words = buffer.collect_all_words();
    assert_eq!(words.get("ola"), Some(&2));
    assert_eq!(words.get("mundo"), Some(&1));
    assert_eq!(words.get("rust_test"), Some(&1));
}
