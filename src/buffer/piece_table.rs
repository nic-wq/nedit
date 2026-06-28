use memmap2::Mmap;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

// ── Fenwick Tree (Binary Indexed Tree) for line counting ──────────────

struct FenwickTree {
    tree: Vec<usize>,
}

impl FenwickTree {
    fn new(size: usize) -> Self {
        Self {
            tree: vec![0; size + 1],
        }
    }

    fn update(&mut self, mut idx: usize, delta: isize) {
        idx += 1;
        while idx < self.tree.len() {
            self.tree[idx] = (self.tree[idx] as isize + delta) as usize;
            idx += idx & (!idx + 1);
        }
    }

    fn prefix_sum(&self, mut idx: usize) -> usize {
        idx += 1;
        let mut sum = 0;
        while idx > 0 {
            sum += self.tree[idx];
            idx -= idx & (!idx + 1);
        }
        sum
    }
}

// ── Piece Table ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Original,
    Add,
}

#[derive(Debug, Clone)]
struct Piece {
    source: Source,
    start: usize,
    len: usize,
}

pub struct PieceTable {
    pieces: Vec<Piece>,
    original_mmap: Option<Mmap>,
    original_len: usize,
    add_buffer: Vec<u8>,
    total_lines: usize,
    total_bytes: usize,
}

impl PieceTable {
    /// Create an empty piece table.
    pub fn new() -> Self {
        Self {
            pieces: Vec::new(),
            original_mmap: None,
            original_len: 0,
            add_buffer: Vec::new(),
            total_lines: 1,
            total_bytes: 0,
        }
    }

    /// Open a file with mmap and initialize as one Original piece.
    pub fn from_path(path: &Path) -> anyhow::Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };
        let len = mmap.len();
        let newline_count = byte_count_newlines(&mmap[..]);

        let mut pieces = Vec::new();
        if len > 0 {
            pieces.push(Piece {
                source: Source::Original,
                start: 0,
                len,
            });
        }

        Ok(Self {
            pieces,
            original_mmap: Some(mmap),
            original_len: len,
            add_buffer: Vec::new(),
            total_lines: if len == 0 { 1 } else { newline_count + 1 },
            total_bytes: len,
        })
    }

    pub fn len_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn len_lines(&self) -> usize {
        self.total_lines
    }

    /// Byte position to line number (0-based).
    pub fn byte_to_line(&self, byte_pos: usize) -> usize {
        if byte_pos == 0 {
            return 0;
        }
        if byte_pos >= self.total_bytes {
            return self.total_lines.saturating_sub(1);
        }
        // Binary search using line_to_byte
        let mut lo = 0;
        let mut hi = self.total_lines;
        while lo < hi {
            let mid = lo + (hi - lo + 1) / 2;
            if self.line_to_byte(mid) <= byte_pos {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        lo
    }

    /// Line number to starting byte position.
    pub fn line_to_byte(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        // Walk pieces, counting newlines until we find the target line
        let mut byte_pos = 0;
        let mut newlines_found = 0;

        for piece in &self.pieces {
            let bytes = self.piece_bytes(piece);
            for i in 0..bytes.len() {
                if bytes[i] == b'\n' {
                    newlines_found += 1;
                    if newlines_found == line {
                        return byte_pos + i + 1;
                    }
                }
            }
            byte_pos += piece.len;
        }

        self.total_bytes
    }

    /// Insert data at byte position.
    pub fn insert(&mut self, byte_pos: usize, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        let add_offset = self.add_buffer.len();
        self.add_buffer.extend_from_slice(data);
        let add_len = data.len();
        let newline_count = byte_count_newlines(data);

        // Find which piece contains byte_pos
        let mut acc = 0;
        let mut split_idx = self.pieces.len();
        let mut local_offset = 0;

        for (i, piece) in self.pieces.iter().enumerate() {
            if acc + piece.len >= byte_pos || i == self.pieces.len() - 1 {
                split_idx = i;
                local_offset = byte_pos.saturating_sub(acc).min(piece.len);
                break;
            }
            acc += piece.len;
        }

        let new_piece = Piece {
            source: Source::Add,
            start: add_offset,
            len: add_len,
        };

        let mut new_pieces = Vec::with_capacity(self.pieces.len() + 2);

        if self.pieces.is_empty() {
            // Empty table, just add the new piece
            new_pieces.push(new_piece);
        } else {
            let piece = &self.pieces[split_idx];
            let left_len = local_offset;
            let right_len = piece.len - left_len;

            // Pieces before the split
            new_pieces.extend_from_slice(&self.pieces[..split_idx]);

            // Left part of the split piece
            if left_len > 0 {
                new_pieces.push(Piece {
                    source: piece.source,
                    start: piece.start,
                    len: left_len,
                });
            }

            // The new inserted piece
            new_pieces.push(new_piece);

            // Right part of the split piece
            if right_len > 0 {
                new_pieces.push(Piece {
                    source: piece.source,
                    start: piece.start + left_len,
                    len: right_len,
                });
            }

            // Pieces after the split
            if split_idx + 1 < self.pieces.len() {
                new_pieces.extend_from_slice(&self.pieces[split_idx + 1..]);
            }
        }

        self.pieces = new_pieces;
        self.total_bytes += add_len;
        self.total_lines += newline_count;
    }

    /// Delete `len` bytes starting at `byte_pos`. Returns the deleted bytes.
    pub fn delete(&mut self, byte_pos: usize, len: usize) -> Vec<u8> {
        if len == 0 {
            return Vec::new();
        }

        let end = byte_pos + len;
        let mut deleted = Vec::with_capacity(len);
        let mut new_pieces = Vec::with_capacity(self.pieces.len());

        let mut acc = 0;
        for piece in &self.pieces {
            let piece_end = acc + piece.len;

            if piece_end <= byte_pos || acc >= end {
                // Piece is completely outside delete range
                new_pieces.push(piece.clone());
            } else {
                let piece_bytes = self.piece_bytes(piece);

                // Left part (before delete range)
                let left_start = byte_pos.saturating_sub(acc);
                let delete_start = left_start;
                let delete_end = (end - acc).min(piece.len);

                if left_start > 0 {
                    new_pieces.push(Piece {
                        source: piece.source,
                        start: piece.start,
                        len: left_start,
                    });
                }

                // The deleted part — collect bytes
                deleted.extend_from_slice(&piece_bytes[delete_start..delete_end]);

                // Right part (after delete range)
                let right_start = delete_end;
                if right_start < piece.len {
                    new_pieces.push(Piece {
                        source: piece.source,
                        start: piece.start + right_start,
                        len: piece.len - right_start,
                    });
                }
            }

            acc += piece_end;
        }

        self.pieces = new_pieces;

        let deleted_newlines = byte_count_newlines(&deleted);
        self.total_bytes -= len;
        self.total_lines = self.total_lines.saturating_sub(deleted_newlines);

        // Ensure at least 1 line
        if self.total_lines == 0 && self.total_bytes > 0 {
            self.total_lines = 1;
        }

        deleted
    }

    pub fn piece_count(&self) -> usize {
        self.pieces.len()
    }

    /// Read bytes from the buffer.
    pub fn read_bytes(&self, start: usize, end: usize) -> Vec<u8> {
        if start >= end || end > self.total_bytes {
            return Vec::new();
        }
        let mut result = Vec::with_capacity(end - start);
        let mut acc = 0;

        for piece in &self.pieces {
            let piece_start = acc;
            let piece_end = acc + piece.len;

            if piece_end <= start || piece_start >= end {
                // No overlap
            } else {
                let bytes = self.piece_bytes(piece);
                let overlap_start = start.saturating_sub(piece_start);
                let overlap_end = (end - piece_start).min(piece.len);
                result.extend_from_slice(&bytes[overlap_start..overlap_end]);
            }

            acc += piece.len;
            if acc >= end {
                break;
            }
        }

        result
    }

    /// Iterate over byte slices for a range.
    pub fn chunk_iter(&self, start: usize, len: usize) -> PieceChunkIter<'_> {
        PieceChunkIter {
            pt: self,
            pos: 0,
            chunk_start: start,
            chunk_end: start + len,
        }
    }

    /// Write the entire buffer to a file using streaming writes.
    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        let tmp_path = path.with_extension("nedit_tmp");
        {
            let file = File::create(&tmp_path)?;
            let mut writer = BufWriter::new(file);

            for piece in &self.pieces {
                let bytes = self.piece_bytes(piece);
                writer.write_all(bytes)?;
            }
            writer.flush()?;
        }

        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    /// Convert to String (lossy). Use sparingly.
    pub fn to_string_lossy(&self) -> String {
        let bytes = self.read_bytes(0, self.total_bytes);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Get line text (without trailing newline).
    pub fn line_text(&self, row: usize) -> String {
        let byte_start = self.line_to_byte(row);
        let byte_end = if row + 1 >= self.total_lines {
            self.total_bytes
        } else {
            self.line_to_byte(row + 1)
        };

        // Exclude trailing newline
        let end = if byte_end > byte_start {
            let last = self.read_bytes(byte_end - 1, byte_end);
            if last == [b'\n'] {
                byte_end - 1
            } else {
                byte_end
            }
        } else {
            byte_end
        };

        let bytes = self.read_bytes(byte_start, end);
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Char count in bytes [0..byte_pos].
    pub fn byte_to_char(&self, byte_pos: usize) -> usize {
        if byte_pos == 0 {
            return 0;
        }
        let clamped = byte_pos.min(self.total_bytes);
        let bytes = self.read_bytes(0, clamped);
        String::from_utf8_lossy(&bytes).chars().count()
    }

    /// Byte position for char index.
    pub fn char_to_byte(&self, char_pos: usize) -> usize {
        if char_pos == 0 {
            return 0;
        }
        let mut char_count = 0;
        let mut byte_pos = 0;

        for piece in &self.pieces {
            let bytes = self.piece_bytes(piece);
            for &b in bytes {
                // Check if this byte starts a new char
                if b < 0x80 || b >= 0xC0 {
                    if char_count >= char_pos {
                        return byte_pos;
                    }
                    char_count += 1;
                }
                byte_pos += 1;
            }
        }

        self.total_bytes
    }

    fn piece_bytes(&self, piece: &Piece) -> &[u8] {
        match piece.source {
            Source::Original => {
                self.original_mmap
                    .as_ref()
                    .map(|mmap| &mmap[piece.start..piece.start + piece.len])
                    .unwrap_or(&[])
            }
            Source::Add => &self.add_buffer[piece.start..piece.start + piece.len],
        }
    }
}

fn byte_count_newlines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b == b'\n').count()
}

// ── Chunk Iterator ────────────────────────────────────────────────────

pub struct PieceChunkIter<'a> {
    pt: &'a PieceTable,
    pos: usize,
    chunk_start: usize,
    chunk_end: usize,
}

impl<'a> Iterator for PieceChunkIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        while self.pos < self.pt.pieces.len() {
            let piece = &self.pt.pieces[self.pos];
            let acc: usize = self.pt.pieces[..self.pos]
                .iter()
                .map(|p| p.len)
                .sum();
            let piece_end = acc + piece.len;
            self.pos += 1;

            if piece_end <= self.chunk_start || acc >= self.chunk_end {
                continue;
            }

            let bytes = self.pt.piece_bytes(piece);
            let overlap_start = self.chunk_start.saturating_sub(acc);
            let overlap_end = (self.chunk_end - acc).min(piece.len);
            return Some(&bytes[overlap_start..overlap_end]);
        }
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_temp_file(content: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn test_new_empty() {
        let pt = PieceTable::new();
        assert_eq!(pt.len_bytes(), 0);
        assert_eq!(pt.len_lines(), 1);
    }

    #[test]
    fn test_from_path_small() {
        let f = make_temp_file(b"hello\nworld\n");
        let pt = PieceTable::from_path(f.path()).unwrap();
        assert_eq!(pt.len_bytes(), 12);
        assert_eq!(pt.len_lines(), 3);
    }

    #[test]
    fn test_insert_beginning() {
        let f = make_temp_file(b"world");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.insert(0, b"hello ");
        assert_eq!(pt.to_string_lossy(), "hello world");
    }

    #[test]
    fn test_insert_middle() {
        let f = make_temp_file(b"helloworld");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.insert(5, b" beautiful ");
        assert_eq!(pt.to_string_lossy(), "hello beautiful world");
    }

    #[test]
    fn test_insert_end() {
        let f = make_temp_file(b"hello");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.insert(5, b" world");
        assert_eq!(pt.to_string_lossy(), "hello world");
    }

    #[test]
    fn test_delete_range() {
        let f = make_temp_file(b"hello world");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        let deleted = pt.delete(5, 6);
        assert_eq!(String::from_utf8_lossy(&deleted), " world");
        assert_eq!(pt.to_string_lossy(), "hello");
    }

    #[test]
    fn test_delete_middle() {
        let f = make_temp_file(b"hello world");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.delete(5, 1);
        assert_eq!(pt.to_string_lossy(), "helloworld");
    }

    #[test]
    fn test_byte_to_line() {
        let f = make_temp_file(b"line1\nline2\nline3\n");
        let pt = PieceTable::from_path(f.path()).unwrap();
        assert_eq!(pt.byte_to_line(0), 0);
        assert_eq!(pt.byte_to_line(5), 0);
        assert_eq!(pt.byte_to_line(6), 1);
        assert_eq!(pt.byte_to_line(11), 1);
        assert_eq!(pt.byte_to_line(12), 2);
    }

    #[test]
    fn test_line_to_byte() {
        let f = make_temp_file(b"line1\nline2\nline3\n");
        let pt = PieceTable::from_path(f.path()).unwrap();
        assert_eq!(pt.line_to_byte(0), 0);
        assert_eq!(pt.line_to_byte(1), 6);
        assert_eq!(pt.line_to_byte(2), 12);
    }

    #[test]
    fn test_save_roundtrip() {
        let f = make_temp_file(b"hello world\nsecond line\n");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.insert(5, b" beautiful");

        let out = NamedTempFile::new().unwrap();
        pt.save_to(out.path()).unwrap();

        let saved = fs::read(out.path()).unwrap();
        assert_eq!(saved, b"hello beautiful world\nsecond line\n");
    }

    #[test]
    fn test_read_bytes() {
        let f = make_temp_file(b"hello world");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.insert(5, b" beautiful");
        let bytes = pt.read_bytes(0, 5);
        assert_eq!(&bytes, b"hello");
    }

    #[test]
    fn test_line_text() {
        let f = make_temp_file(b"line1\nline2\nline3\n");
        let pt = PieceTable::from_path(f.path()).unwrap();
        assert_eq!(pt.line_text(0), "line1");
        assert_eq!(pt.line_text(1), "line2");
        assert_eq!(pt.line_text(2), "line3");
    }

    #[test]
    fn test_utf8_insert() {
        let f = make_temp_file("hello".as_bytes());
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.insert(5, " world".as_bytes());
        assert_eq!(pt.to_string_lossy(), "hello world");
    }

    #[test]
    fn test_insert_newline() {
        let f = make_temp_file(b"hello");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.insert(5, b"\nworld");
        assert_eq!(pt.len_lines(), 2);
        assert_eq!(pt.line_text(0), "hello");
        assert_eq!(pt.line_text(1), "world");
    }

    #[test]
    fn test_multiple_inserts() {
        let mut pt = PieceTable::new();
        pt.insert(0, b"b");
        pt.insert(0, b"a");
        pt.insert(2, b"c");
        assert_eq!(pt.to_string_lossy(), "abc");
    }

    #[test]
    fn test_insert_delete_undo() {
        let f = make_temp_file(b"hello world");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        let deleted = pt.delete(5, 6);
        assert_eq!(pt.to_string_lossy(), "hello");
        pt.insert(5, &deleted);
        assert_eq!(pt.to_string_lossy(), "hello world");
    }

    #[test]
    fn test_chunk_iter() {
        let f = make_temp_file(b"hello world");
        let mut pt = PieceTable::from_path(f.path()).unwrap();
        pt.insert(5, b" beautiful");

        let chunks: Vec<&[u8]> = pt.chunk_iter(0, 5).collect();
        let combined: Vec<u8> = chunks.into_iter().flatten().copied().collect();
        assert_eq!(&combined, b"hello");
    }

    #[test]
    fn test_empty_file() {
        let pt = PieceTable::new();
        assert_eq!(pt.len_bytes(), 0);
        assert_eq!(pt.len_lines(), 1);
        assert_eq!(pt.to_string_lossy(), "");
        assert_eq!(pt.line_text(0), "");
    }

    #[test]
    fn test_byte_to_char_ascii() {
        let f = make_temp_file(b"hello");
        let pt = PieceTable::from_path(f.path()).unwrap();
        assert_eq!(pt.byte_to_char(0), 0);
        assert_eq!(pt.byte_to_char(3), 3);
        assert_eq!(pt.byte_to_char(5), 5);
    }

    #[test]
    fn test_char_to_byte_ascii() {
        let f = make_temp_file(b"hello");
        let pt = PieceTable::from_path(f.path()).unwrap();
        assert_eq!(pt.char_to_byte(0), 0);
        assert_eq!(pt.char_to_byte(3), 3);
        assert_eq!(pt.char_to_byte(5), 5);
    }
}
