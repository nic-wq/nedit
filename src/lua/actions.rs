use std::time::Duration;

use crate::app::NotificationType;

// We use an enum to represent script actions instead of applying them immediately.
// This allows us to validate actions, generate descriptions for the user,
// and implement an "undo" mechanism for script executions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LuaAction {
    WriteSelection(String),
    WriteCurrentFile(String),
    Notify {
        message: String,
        kind: NotificationType,
        duration: Duration,
    },
}

impl LuaAction {
    pub fn description(&self) -> String {
        match self {
            LuaAction::WriteSelection(_) => "Replace selected text".to_string(),
            LuaAction::WriteCurrentFile(_) => "Overwrite current file".to_string(),
            LuaAction::Notify { .. } => "Show notification".to_string(),
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
}

#[derive(Clone, Debug)]
pub struct ScriptUndo {
    pub actions: Vec<RevertAction>,
}
