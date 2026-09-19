#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Focus {
    Explorer,
    Editor,
}

// We use FuzzyMode to represent any modal interaction that requires a searchable list or text input,
// consolidating various UI workflows into a single manageable state machine.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PendingAction {
    CloseTab,
    Quit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NotificationType {
    Error,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TabHitbox {
    pub buffer_idx: usize,
    pub y: u16,
    pub tab_start_x: u16,
    pub tab_end_x: u16,
    pub close_start_x: Option<u16>,
    pub close_end_x: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModalAction {
    ConfirmDelete,
    Cancel,
    SaveUnsaved,
    DiscardUnsaved,
    ReloadExternal,
    KeepExternal,
    ConfirmInput,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModalButtonHitbox {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub action: ModalAction,
}
