use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum LuaAction {
    WriteSelection(String),
    WriteCurrentFile(String),
    WriteFile(PathBuf, String),
    CreateFile(PathBuf, String),
    DeleteFile(PathBuf),
}

impl LuaAction {
    pub fn description(&self) -> String {
        match self {
            LuaAction::WriteSelection(_) => "Replace selected text".to_string(),
            LuaAction::WriteCurrentFile(_) => "Overwrite current file".to_string(),
            LuaAction::WriteFile(p, _) => format!("Write to {}", p.display()),
            LuaAction::CreateFile(p, _) => format!("Create {}", p.display()),
            LuaAction::DeleteFile(p) => format!("Delete {}", p.display()),
        }
    }
}
