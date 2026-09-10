use std::cell::Cell;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use super::FileItem;
use crate::app::matcher::FuzzyMatcher;
use crate::line_input::LineInput;

/// List rows of the explorer panel for a given area height: 1 row is the explorer title,
/// 3 rows are the bordered search bar. Navigation scroll math and
/// rendering share this so the selection can never scroll out of view.
pub fn explorer_list_height(area_height: u16) -> usize {
    area_height.saturating_sub(4) as usize
}

/// Calculates the horizontal scroll offset for a single-line input field so that the
/// cursor at `cursor_col` is always visible within `avail_width` columns without
/// shifting unnecessarily or leaving unnecessary empty space.
pub fn follow_horizontal_scroll(
    cursor_col: usize,
    current_scroll: usize,
    total_width: usize,
    avail_width: usize,
) -> usize {
    if avail_width == 0 {
        return 0;
    }
    let max_scroll = (total_width + 1).saturating_sub(avail_width);
    let mut scroll = current_scroll.min(max_scroll);
    if cursor_col < scroll {
        scroll = cursor_col;
    } else if cursor_col >= scroll + avail_width {
        scroll = cursor_col + 1 - avail_width;
    }
    scroll.min(max_scroll)
}

/// Directory names skipped everywhere (search corpus and file index).
pub(crate) fn should_skip_dir_name(name: &str) -> bool {
    if name.starts_with('.') {
        return !matches!(name, ".github" | ".vscode" | ".gitlab");
    }
    matches!(
        name,
        "target"
            | "node_modules"
            | "dist"
            | "build"
            | "vendor"
            | "proc"
            | "sys"
            | "dev"
            | "run"
            | "venv"
            | "__pycache__"
            | "AppData"
    )
}

pub struct FileExplorer {
    pub root: PathBuf,
    pub items: Vec<FileItem>,
    pub selected_idx: usize,
    // We use a HashSet for expanded paths to provide O(1) lookups when deciding
    // whether to render a directory's children during the recursive load.
    pub expanded_paths: HashSet<PathBuf>,
    pub scroll_offset: usize,
    pub max_item_width: usize,
    /// Always-visible search field content (top of the panel).
    pub search_input: LineInput,
    /// Flat fuzzy-filtered view, authoritative while searching.
    /// Covers the whole tree (including collapsed folders), not just `items`.
    pub search_results: Vec<FileItem>,
    pub search_selected: usize,
    pub search_scroll: usize,
    /// Horizontal scroll offset for the explorer search input field.
    pub search_hscroll: Cell<usize>,
    /// Every file and directory under `root`, ignoring expansion state.
    /// Rebuilt on refresh; the search filter runs over this cache so each
    /// keystroke is an in-memory pass instead of a filesystem walk.
    pub search_corpus: Vec<(PathBuf, bool)>,
}

impl FileExplorer {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            items: Vec::new(),
            selected_idx: 0,
            expanded_paths: HashSet::new(),
            scroll_offset: 0,
            max_item_width: 20,
            search_input: LineInput::new(),
            search_results: Vec::new(),
            search_selected: 0,
            search_scroll: 0,
            search_hscroll: Cell::new(0),
            search_corpus: Vec::new(),
        }
    }

    /// Whether the search bar currently filters the panel.
    pub fn is_searching(&self) -> bool {
        !self.search_input.is_empty()
    }

    /// Visible rows: the filtered flat list while searching, else the tree.
    pub fn visible_items(&self) -> &[FileItem] {
        if self.is_searching() {
            &self.search_results
        } else {
            &self.items
        }
    }

    /// Selected row index within [`Self::visible_items`].
    pub fn visible_selected_idx(&self) -> usize {
        if self.is_searching() {
            self.search_selected
        } else {
            self.selected_idx
        }
    }

    /// Selected row within [`Self::visible_items`].
    pub fn visible_selected(&self) -> Option<&FileItem> {
        let items = self.visible_items();
        if items.is_empty() {
            return None;
        }
        items.get(self.visible_selected_idx().min(items.len() - 1))
    }

    /// Scroll offset keeping `selected` visible in a `height`-row window,
    /// moving as little as possible. Unlike `selected - height + 1`, this
    /// stays at 0 when everything fits, so wrapping never hides the top.
    pub fn follow_scroll(selected: usize, scroll: usize, len: usize, height: usize) -> usize {
        if len == 0 {
            return 0;
        }
        let selected = selected.min(len - 1);
        if height == 0 {
            return selected;
        }
        if selected < scroll {
            selected
        } else if selected >= scroll + height {
            selected + 1 - height
        } else {
            scroll
        }
    }

    /// Drop the search entirely, keeping the tree selection/scroll untouched.
    pub fn clear_search(&mut self) {
        self.search_input.clear();
        self.search_results.clear();
        self.search_selected = 0;
        self.search_scroll = 0;
        self.search_hscroll.set(0);
    }

    pub const MAX_CORPUS_ITEMS: usize = 20_000;
    pub const MAX_CORPUS_DEPTH: usize = 12;

    /// Walk the whole tree under `root`, ignoring expansion state, so the
    /// filter can surface files inside collapsed folders too.
    pub fn collect_search_corpus(&mut self) {
        self.search_corpus.clear();
        let mut stack = vec![(self.root.clone(), 0usize)];
        let mut visited: HashSet<PathBuf> = HashSet::new();

        if let Ok(canon) = fs::canonicalize(&self.root) {
            visited.insert(canon);
        } else {
            visited.insert(self.root.clone());
        }

        while let Some((dir, depth)) = stack.pop() {
            if self.search_corpus.len() >= Self::MAX_CORPUS_ITEMS {
                break;
            }
            if depth >= Self::MAX_CORPUS_DEPTH {
                continue;
            }

            let Ok(entries) = fs::read_dir(&dir) else {
                continue;
            };

            for entry in entries.flatten() {
                if self.search_corpus.len() >= Self::MAX_CORPUS_ITEMS {
                    break;
                }

                let Ok(file_type) = entry.file_type() else {
                    continue;
                };

                // NEVER recursively follow symlinks to avoid circular loops and runaway crawls.
                if file_type.is_symlink() {
                    let path = entry.path();
                    self.search_corpus.push((path, false));
                    continue;
                }

                let path = entry.path();
                let is_dir = file_type.is_dir();

                if is_dir {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if should_skip_dir_name(name) {
                            continue;
                        }
                    }

                    let canon = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                    if visited.insert(canon) {
                        stack.push((path.clone(), depth + 1));
                    }
                }

                self.search_corpus.push((path, is_dir));
            }
        }
    }

    /// Recompute the filtered view from the corpus. With `reset_selection`,
    /// jump to the top (used after query edits); otherwise preserve the
    /// selected path when it is still present (used after corpus refreshes).
    pub fn update_search_results(&mut self, reset_selection: bool) {
        let previous = (!reset_selection)
            .then(|| {
                self.search_results
                    .get(self.search_selected)
                    .map(|item| item.path.clone())
            })
            .flatten();
        self.search_results = self.filter_corpus();
        self.search_selected = previous
            .and_then(|path| self.search_results.iter().position(|i| i.path == path))
            .unwrap_or(0);
        // Without the viewport height the input handler re-follows on every
        // navigation key; here just never point past the end and never hide
        // the selection above.
        self.search_scroll = self
            .search_scroll
            .min(self.search_results.len().saturating_sub(1));
        if self.search_selected < self.search_scroll {
            self.search_scroll = self.search_selected;
        }
    }

    /// Fuzzy-match the corpus against the query, best score first.
    /// Same scoring engine as the Ctrl+O file finder.
    fn filter_corpus(&self) -> Vec<FileItem> {
        let query = self.search_input.text.trim().to_lowercase();
        if query.is_empty() {
            return Vec::new();
        }
        // Single matcher per keystroke, reused across all candidates.
        let mut matcher = FuzzyMatcher::new(&query);
        let mut scored: Vec<(i32, PathBuf, bool)> = Vec::new();
        for (path, is_dir) in &self.search_corpus {
            let rel = path
                .strip_prefix(&self.root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let fm = matcher.match_target(&rel);
            if fm.matched {
                scored.push((fm.score, path.clone(), *is_dir));
            }
        }
        scored.sort_by_key(|scored| std::cmp::Reverse(scored.0));
        scored
            .into_iter()
            .map(|(_, path, is_dir)| {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                FileItem {
                    path,
                    is_dir,
                    name,
                    depth: 0,
                    expanded: false,
                }
            })
            .collect()
    }

    pub fn refresh_sync(&mut self) {
        let selected_path = self.items.get(self.selected_idx).map(|i| i.path.clone());
        self.items.clear();
        self.max_item_width = 20;
        self.load_dir_recursive(&self.root.clone(), 0);
        self.collect_search_corpus();

        if let Some(path) = selected_path {
            if let Some(idx) = self.items.iter().position(|i| i.path == path) {
                self.selected_idx = idx;
            } else {
                self.selected_idx = self.selected_idx.min(self.items.len().saturating_sub(1));
            }
        }
    }

    pub fn load_dir_recursive(&mut self, path: &PathBuf, depth: usize) {
        let mut entries_vec = Vec::new();
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                let entry_path = entry.path();
                let is_dir = if file_type.is_symlink() {
                    false
                } else {
                    file_type.is_dir()
                };
                let name = entry_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let expanded = is_dir && self.expanded_paths.contains(&entry_path);

                entries_vec.push(FileItem {
                    path: entry_path,
                    is_dir,
                    name,
                    depth,
                    expanded,
                });
            }
        }

        entries_vec.sort_by(|a, b| {
            if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.cmp(&b.name)
            }
        });

        for item in entries_vec {
            let expanded = item.expanded;
            let item_path = item.path.clone();

            let width = item.depth * 2 + item.name.len() + 10;
            if width > self.max_item_width {
                self.max_item_width = width;
            }

            self.items.push(item);
            if expanded {
                self.load_dir_recursive(&item_path, depth + 1);
            }
        }
    }

    pub fn next(&mut self) {
        if !self.items.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.items.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.items.is_empty() {
            if self.selected_idx == 0 {
                self.selected_idx = self.items.len() - 1;
            } else {
                self.selected_idx -= 1;
            }
        }
    }

    pub fn toggle_expand(&mut self) {
        if let Some(item) = self.items.get_mut(self.selected_idx) {
            if item.is_dir {
                item.expanded = !item.expanded;
                if item.expanded {
                    self.expanded_paths.insert(item.path.clone());
                } else {
                    self.expanded_paths.remove(&item.path);
                }
            }
        }
    }

    pub fn go_up_root(&mut self) {
        if let Some(parent) = self.root.parent() {
            self.root = parent.to_path_buf();
            self.selected_idx = 0;
            self.scroll_offset = 0;
            self.invalidate_search();
        }
    }

    /// Drop filtered results and the corpus (e.g. the root changed and a
    /// refresh is on its way). The query text is kept: results rebuild as
    /// soon as the fresh corpus arrives.
    pub fn invalidate_search(&mut self) {
        self.search_results.clear();
        self.search_corpus.clear();
        self.search_selected = 0;
        self.search_scroll = 0;
    }

    pub fn get_selected(&self) -> Option<&FileItem> {
        self.items.get(self.selected_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::FileExplorer;
    use std::path::PathBuf;

    fn explorer_with(n: usize) -> FileExplorer {
        let mut explorer = FileExplorer::new(PathBuf::from("/root"));
        explorer.items = (0..n)
            .map(|i| super::super::FileItem {
                path: PathBuf::from(format!("/root/file{i}.txt")),
                is_dir: false,
                name: format!("file{i}.txt"),
                depth: 0,
                expanded: false,
            })
            .collect();
        explorer
    }

    #[test]
    fn follow_scroll_keeps_selection_visible() {
        // Selection below the window scrolls minimally.
        assert_eq!(FileExplorer::follow_scroll(10, 0, 30, 10), 1);
        // Selection above jumps back up.
        assert_eq!(FileExplorer::follow_scroll(5, 20, 30, 10), 5);
        // Inside the window: no movement.
        assert_eq!(FileExplorer::follow_scroll(25, 20, 30, 10), 20);
        // Single-row window always shows the selection.
        assert_eq!(FileExplorer::follow_scroll(7, 0, 30, 1), 7);
        // Empty list and out-of-range selection clamp safely.
        assert_eq!(FileExplorer::follow_scroll(0, 0, 0, 10), 0);
        assert_eq!(FileExplorer::follow_scroll(99, 0, 10, 5), 5);
    }

    #[test]
    fn wrap_to_bottom_never_hides_the_top_when_everything_fits() {
        // Reported bug: with 5 items fully visible, wrapping Up from the
        // first row pushed scroll to 1 and hid the first folder; it only
        // came back after wrapping Down to 0 (which reset scroll).
        // len=5, height=10: old formula gave 4-10+1 = 1 (wrong).
        assert_eq!(FileExplorer::follow_scroll(4, 0, 5, 10), 0);
        // Tall viewport, bottom selection: still 0.
        assert_eq!(FileExplorer::follow_scroll(2, 0, 3, 10), 0);
        // Long list still scrolls to reveal the wrapped bottom row.
        assert_eq!(FileExplorer::follow_scroll(29, 0, 30, 10), 20);
    }

    #[test]
    fn filter_covers_collapsed_folders_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/nested")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "fn main(){}").unwrap();
        std::fs::write(dir.path().join("src/nested/deep.txt"), "deep").unwrap();
        std::fs::write(dir.path().join("README.md"), "readme").unwrap();

        let mut explorer = FileExplorer::new(dir.path().to_path_buf());
        explorer.refresh_sync();
        // Nothing expanded: tree shows only top-level rows...
        assert!(explorer.items.iter().all(|i| i.depth == 0));
        // ...but the corpus (and the filter) sees everything.
        assert!(explorer.search_corpus.len() >= 4);

        // Deep file inside collapsed folders matches.
        explorer.search_input.set_text("deep".to_string());
        explorer.update_search_results(true);
        assert_eq!(explorer.search_results.len(), 1);
        assert_eq!(
            explorer.search_results[0].path,
            dir.path().join("src/nested/deep.txt")
        );

        // Directories match too.
        explorer.search_input.set_text("nested".to_string());
        explorer.update_search_results(true);
        assert!(explorer
            .search_results
            .iter()
            .any(|i| i.is_dir && i.path == dir.path().join("src/nested")));

        // Empty query disables the filtered view.
        explorer.search_input.clear();
        assert!(!explorer.is_searching());
        assert!(explorer.visible_items().as_ptr() == explorer.items.as_ptr());
    }

    #[test]
    fn search_selection_resets_and_clamps() {
        let mut explorer = explorer_with(0);
        explorer.search_corpus = vec![
            (PathBuf::from("/root/b.txt"), false),
            (PathBuf::from("/root/a.txt"), false),
        ];
        explorer.search_input.set_text("txt".to_string());
        explorer.update_search_results(true);
        assert_eq!(explorer.search_results.len(), 2);
        assert_eq!(explorer.search_selected, 0);
        assert_eq!(explorer.search_scroll, 0);
    }

    #[test]
    fn corpus_refresh_preserves_selected_path() {
        let mut explorer = explorer_with(0);
        explorer.search_corpus = vec![
            (PathBuf::from("/root/b.txt"), false),
            (PathBuf::from("/root/a.txt"), false),
        ];
        explorer.search_input.set_text("txt".to_string());
        explorer.update_search_results(true);
        // Select "a.txt" wherever it ranked...
        let pos = explorer
            .search_results
            .iter()
            .position(|i| i.name == "a.txt")
            .unwrap();
        explorer.search_selected = pos;
        // ...and keep it across a corpus refresh that drops "b.txt".
        explorer.search_corpus = vec![(PathBuf::from("/root/a.txt"), false)];
        explorer.update_search_results(false);
        assert_eq!(explorer.search_results.len(), 1);
        assert_eq!(explorer.search_results[0].name, "a.txt");
        assert_eq!(explorer.search_selected, 0);
    }

    #[test]
    fn horizontal_scroll_follows_cursor_both_ways() {
        use super::follow_horizontal_scroll;

        let avail = 10;
        // Text fits: scroll stays 0
        assert_eq!(follow_horizontal_scroll(0, 0, 5, avail), 0);
        assert_eq!(follow_horizontal_scroll(5, 0, 5, avail), 0);

        // Typing past the right edge scrolls right
        // cursor at 10 on avail 10 -> scroll becomes 1
        assert_eq!(follow_horizontal_scroll(10, 0, 10, avail), 1);
        // cursor at 25 on total 30 -> scroll becomes 25 + 1 - 10 = 16
        assert_eq!(follow_horizontal_scroll(25, 5, 30, avail), 16);

        // Moving left stays stable until reaching the left edge
        // cursor at 20 with scroll 16: still in [16, 26), scroll stays 16
        assert_eq!(follow_horizontal_scroll(20, 16, 30, avail), 16);
        // cursor moves past the left edge to 12 -> scroll adjusts to 12
        assert_eq!(follow_horizontal_scroll(12, 16, 30, avail), 12);
        // Home moves cursor to 0 -> scroll adjusts to 0
        assert_eq!(follow_horizontal_scroll(0, 12, 30, avail), 0);

        // Deleting text clamps scroll when total_width shrinks
        assert_eq!(follow_horizontal_scroll(5, 10, 8, avail), 0);
    }

    #[test]
    #[cfg(unix)]
    fn collect_search_corpus_never_loops_on_circular_symlinks() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("file.txt"), "hello").unwrap();

        // Create circular symlink: sub/loop -> dir
        let loop_link = sub.join("loop");
        let _ = symlink(dir.path(), &loop_link);

        let mut explorer = FileExplorer::new(dir.path().to_path_buf());
        explorer.collect_search_corpus();

        // It must terminate immediately, without exploding the corpus
        assert!(explorer.search_corpus.len() < 10);
        assert!(explorer.search_corpus.iter().any(|(p, _)| p == &sub.join("file.txt")));
    }
}
