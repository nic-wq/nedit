use std::path::PathBuf;

#[derive(Clone)]
pub struct LuaContext {
    pub current_file: String,
    pub current_content: String,
    pub current_selection: String,
    pub current_dir: PathBuf,
    pub is_live_script: bool,
}
