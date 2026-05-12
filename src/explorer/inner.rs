use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

use super::items::FileItem;

pub struct FileExplorer {
    pub root: PathBuf,
    pub items: Vec<FileItem>,
    pub selected_idx: usize,
    pub expanded_paths: HashSet<PathBuf>,
    pub scroll_offset: usize,
    pub max_item_width: usize,
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
        }
    }

    pub fn refresh_sync(&mut self) {
        let selected_path = self.items.get(self.selected_idx).map(|i| i.path.clone());
        self.items.clear();
        self.max_item_width = 20;
        self.load_dir_recursive(&self.root.clone(), 0);

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
                let entry_path = entry.path();
                let is_dir = entry_path.is_dir();
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
        }
    }

    pub fn get_selected(&self) -> Option<&FileItem> {
        self.items.get(self.selected_idx)
    }
}
