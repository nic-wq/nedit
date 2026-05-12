mod inner;
mod file_ops;
mod fuzzy;
mod live_script;
mod scripting;
mod theme;
mod types;
mod workspace;

pub(crate) const DOC_LUA: &str = include_str!("../../docs/lua.md");
pub(crate) const DOC_BINDS: &str = include_str!("../../docs/binds.md");
pub(crate) const DOC_MAIN: &str = include_str!("../../docs/docs.md");

pub use inner::App;
pub use types::{Focus, FuzzyMode, NotificationType};
pub use workspace::{Workspace, WorkspaceList};
