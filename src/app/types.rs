#[derive(PartialEq, Eq)]
pub enum Focus {
    Explorer,
    Editor,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum FuzzyMode {
    Files,
    Content,
    Local,
    Themes,
    SaveAs,
    FileOptions,
    Rename,
    DeleteConfirm,
    CommandPalette,
    Move,
    RunScript,
    EditScript,
    DeleteScript,
    DocSelect,
    NewFolder,
    ScriptMenu,
    ScriptInput,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Error,
    Info,
}
