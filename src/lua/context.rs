use std::path::PathBuf;

// LuaContext provides a snapshot of the editor state to the script at the start of its execution,
// ensuring that the script has all the necessary information without needing to query 
// the main thread constantly (which would be complex due to threading).
#[derive(Clone)]
pub struct LuaContext {
    pub current_file: String,
    pub current_content: String,
    pub current_selection: String,
    pub current_dir: PathBuf,
    pub is_live_script: bool,
}
