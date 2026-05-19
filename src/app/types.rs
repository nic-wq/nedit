#[derive(PartialEq, Eq)]
pub enum Focus {
    Explorer,
    Editor,
}

// We use FuzzyMode to represent any modal interaction that requires a searchable list or text input,
// consolidating various UI workflows into a single manageable state machine.
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
