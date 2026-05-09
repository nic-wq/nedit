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

#[derive(Clone, Debug)]
pub enum RevertAction {
    RestoreBufferContent {
        buffer_idx: usize,
        content: String,
        cursor: (usize, usize),
    },
    RestoreFile {
        path: PathBuf,
        content: Option<String>, // None means delete the file (if it was created by script)
    },
}

#[derive(Clone, Debug)]
pub struct ScriptUndo {
    pub actions: Vec<RevertAction>,
}

#[derive(Debug)]
pub enum ScriptRequest {
    Prompt { title: String, default: String },
    Menu { title: String, options: Vec<String> },
}

pub enum ScriptResponse {
    Prompt(String),
    Menu(Option<String>),
}
