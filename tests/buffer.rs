use nedit::buffer::EditorBuffer;
use std::path::PathBuf;

#[test]
fn test_buffer_from_non_existent_path() {
    // Edge case: file that does not exist should create an empty buffer with the set path
    let path = PathBuf::from("this_file_really_should_not_exist_12345.txt");
    let buffer = EditorBuffer::from_path(path.clone()).unwrap();
    
    assert_eq!(buffer.content.to_string(), "");
    assert_eq!(buffer.path, Some(path));
}

#[test]
fn test_buffer_to_char_idx() {
    let mut buffer = EditorBuffer::new();
    buffer.content = ropey::Rope::from_str("line1\nline2\nline3");
    
    // Testing conversion of line/column to global index
    // "line1\n" is 6 chars. 'l' from "line2" is at index 6.
    assert_eq!(buffer.to_char_idx(1, 0), 6);
    assert_eq!(buffer.to_char_idx(0, 0), 0);
}

#[test]
fn test_buffer_char_to_line_col() {
    let mut buffer = EditorBuffer::new();
    buffer.content = ropey::Rope::from_str("abc\ndef");
    
    // 'd' is at index 4
    let (line, col) = buffer.char_to_line_col(4);
    assert_eq!(line, 1);
    assert_eq!(col, 0);
}

#[test]
fn test_buffer_collect_words() {
    let mut buffer = EditorBuffer::new();
    buffer.content = ropey::Rope::from_str("hello world hello rust_test");
    
    let words = buffer.collect_all_words();
    assert_eq!(words.get("hello"), Some(&2));
    assert_eq!(words.get("world"), Some(&1));
    assert_eq!(words.get("rust_test"), Some(&1));
}
