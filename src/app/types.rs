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
    Workspaces,
    WorkspaceAddName,
    WorkspaceAddPath,
    CommandPalette,
    Move,
    RunScript,
    EditScript,
    DeleteScript,
    DocSelect,
    NewFolder,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NotificationType {
    Error,
    Info,
}
