pub const TAB_WIDTH: usize = 4;

pub fn visual_width(c: char) -> usize {
    if c == '\t' { TAB_WIDTH } else { 1 }
}

pub fn visual_column_at_char_index(line: &str, char_index: usize) -> usize {
    let mut visual = 0;
    for (i, c) in line.chars().enumerate() {
        if i >= char_index {
            break;
        }
        visual += visual_width(c);
    }
    visual
}

/// Char index for the cursor so that the visual column is `target_visual` (clamped to the line).
pub fn char_index_from_visual_column(line: &str, target_visual: usize) -> usize {
    let mut visual = 0;
    for (i, c) in line.chars().enumerate() {
        if visual >= target_visual {
            return i;
        }
        visual += visual_width(c);
    }
    line.chars().count()
}
