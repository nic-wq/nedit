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
    DocSelect,
    /// Create a file or folder relative to the explorer selection.
    /// Trailing `/` creates a folder; otherwise creates an empty file and opens it.
    Create,
    UnsavedChanges,
    ExternalChange,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PendingAction {
    CloseTab,
    Quit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotificationType {
    Error,
    Info,
}
