use std::path::PathBuf;

pub struct FileItem {
    pub path: PathBuf,
    pub is_dir: bool,
    pub name: String,
    pub depth: usize,
    pub expanded: bool,
}
