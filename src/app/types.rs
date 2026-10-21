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
    Settings,
    SettingInput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingId {
    Autocomplete,
    IndentGuides,
    FilePreview,
    BracketMatching,
    NotificationPosition,
    Theme,
    PreviewMaxSize,
    Scale,
}

impl SettingId {
    pub const ALL: [SettingId; 8] = [
        SettingId::Autocomplete,
        SettingId::IndentGuides,
        SettingId::FilePreview,
        SettingId::BracketMatching,
        SettingId::NotificationPosition,
        SettingId::Theme,
        SettingId::PreviewMaxSize,
        SettingId::Scale,
    ];

    pub fn key(&self) -> &'static str {
        match self {
            SettingId::Autocomplete => "autocomplete",
            SettingId::IndentGuides => "indent_guides",
            SettingId::FilePreview => "preview_enabled",
            SettingId::BracketMatching => "bracket_matching",
            SettingId::NotificationPosition => "notification_position",
            SettingId::Theme => "theme",
            SettingId::PreviewMaxSize => "preview_max_size",
            SettingId::Scale => "scale",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            SettingId::Autocomplete => "󰈈 ",
            SettingId::IndentGuides => "󰘳 ",
            SettingId::FilePreview => "󰈔 ",
            SettingId::BracketMatching => "󰅩 ",
            SettingId::NotificationPosition => "󰂚 ",
            SettingId::Theme => "󰏘 ",
            SettingId::PreviewMaxSize => "󰋊 ",
            SettingId::Scale => "󰍍 ",
        }
    }

    pub fn i18n_key(&self) -> &'static str {
        match self {
            SettingId::Autocomplete => "setting_autocomplete",
            SettingId::IndentGuides => "setting_indent_guides",
            SettingId::FilePreview => "setting_preview_enabled",
            SettingId::BracketMatching => "setting_bracket_matching",
            SettingId::NotificationPosition => "setting_notification_position",
            SettingId::Theme => "setting_theme",
            SettingId::PreviewMaxSize => "setting_preview_max_size",
            SettingId::Scale => "setting_scale",
        }
    }

    pub fn default_label(&self) -> &'static str {
        match self {
            SettingId::Autocomplete => "Autocomplete",
            SettingId::IndentGuides => "Show Indent Guides",
            SettingId::FilePreview => "File Explorer Preview",
            SettingId::BracketMatching => "Highlight Matching Bracket",
            SettingId::NotificationPosition => "Notification Position",
            SettingId::Theme => "Color Theme",
            SettingId::PreviewMaxSize => "Preview Max Size",
            SettingId::Scale => "UI Scale",
        }
    }

    pub fn is_free_choice(&self) -> bool {
        matches!(self, SettingId::PreviewMaxSize | SettingId::Scale)
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMenuAction {
    // Editor actions
    Copy,
    Cut,
    Paste,
    DeleteSelection,
    SelectAll,
    SelectWord,
    // Explorer actions (reusing existing FileOptions)
    OpenFile,
    Rename,
    Delete,
    NewFile,
    NewFolder,
    Move,
    CopyPath,
    SetRoot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenuItem {
    pub label: &'static str,
    pub i18n_key: &'static str,
    pub icon: &'static str,
    pub shortcut: Option<&'static str>,
    pub action: ContextMenuAction,
    pub is_danger: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContextMenuTarget {
    Editor,
    ExplorerItem(std::path::PathBuf, bool),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenu {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub items: Vec<ContextMenuItem>,
    pub selected_idx: usize,
    pub target: ContextMenuTarget,
}

