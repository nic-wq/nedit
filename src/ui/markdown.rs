use tree_sitter_md::MarkdownParser;

fn kind_to_capture(kind: &str) -> Option<&'static str> {
    match kind {
        "atx_heading" | "setext_heading" => Some("text.title"),
        "fenced_code_block" | "indented_code_block" | "code_span" | "code_fence_content" => {
            Some("text.literal")
        }
        "emphasis" => Some("text.emphasis"),
        "strong_emphasis" => Some("text.strong"),
        "link_destination" | "uri_autolink" | "www_autolink" | "email_autolink" | "link_title" => {
            Some("text.uri")
        }
        "link_text" | "link_label" | "image_description" => Some("text.reference"),
        "list_marker_plus"
        | "list_marker_minus"
        | "list_marker_star"
        | "list_marker_dot"
        | "list_marker_parenthesis"
        | "atx_h1_marker"
        | "atx_h2_marker"
        | "atx_h3_marker"
        | "atx_h4_marker"
        | "atx_h5_marker"
        | "atx_h6_marker"
        | "fenced_code_block_delimiter"
        | "block_quote_marker"
        | "thematic_break"
        | "setext_h1_underline"
        | "setext_h2_underline" => Some("punctuation.special"),
        "backslash_escape" | "hard_line_break" => Some("string.escape"),
        "emphasis_delimiter" | "code_span_delimiter" => Some("punctuation.delimiter"),
        _ => None,
    }
}

/// Highlight a markdown document by walking the block+inline syntax tree.
/// Returns `(byte_start, byte_end, capture_name)` sorted in document order;
/// later ranges overwrite earlier ones when painting.
pub fn highlight_ranges(doc: &str) -> Vec<(usize, usize, &'static str)> {
    let mut parser = MarkdownParser::default();
    let tree = match parser.parse(doc.as_bytes(), None) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let mut ranges = Vec::new();
    let mut cursor = tree.walk();
    // Walk the combined block/inline tree in document order using MarkdownCursor
    // helpers; `goto_first_child` already descends into inline trees.
    let mut visited_root = false;
    loop {
        let node = cursor.node();
        if let Some(cap) = kind_to_capture(node.kind()) {
            ranges.push((node.start_byte(), node.end_byte(), cap));
        }
        if cursor.goto_first_child() {
            visited_root = false;
            continue;
        }
        // Try next sibling; if none, ascend until a sibling exists
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                visited_root = true;
                break;
            }
        }
        if visited_root {
            break;
        }
    }
    ranges
}
