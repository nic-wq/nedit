#[allow(clippy::module_inception)]
mod app;
mod file_ops;
mod fuzzy;
mod fuzzy_input;
pub mod toast;
mod live_script;
pub mod matcher;
mod theme;
mod settings;
pub mod types;

// We include documentation files as static strings to ensure they are always available 
// within the binary, even if the user hasn't properly installed the docs folder.
pub(crate) const DOC_LUA: &str = include_str!("../../docs/lua.md");
pub(crate) const DOC_BINDS: &str = include_str!("../../docs/binds.md");
pub(crate) const DOC_MAIN: &str = include_str!("../../docs/docs.md");
pub(crate) const DOC_EXPECTED_BEHAVIOR: &str = include_str!("../../docs/expected_behavior.md");
pub(crate) const DOC_I18N: &str = include_str!("../../docs/i18n.md");

pub use app::{App, LargeFileLoadResult};
pub use file_ops::trim_memory;
pub use types::{
    ContextMenu, ContextMenuAction, ContextMenuItem, ContextMenuTarget, Focus, FuzzyMode,
    ModalAction, ModalButtonHitbox, NotificationType, SettingId, TabHitbox,
};
