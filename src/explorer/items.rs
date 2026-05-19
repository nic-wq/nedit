use std::path::PathBuf;

pub struct FileItem {
    pub path: PathBuf,
    pub is_dir: bool,
    pub name: String,
    // depth is used to calculate the visual indentation in the tree view,
    // allowing for a clear hierarchical representation of the filesystem.
    pub depth: usize,
    pub expanded: bool,
}
