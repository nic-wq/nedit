mod app;
mod file_ops;
mod fuzzy;
mod live_script;
mod theme;
mod types;
mod workspace;

pub(crate) const DOC_LUA: &str = include_str!("../../docs/lua.md");
pub(crate) const DOC_BINDS: &str = include_str!("../../docs/binds.md");
pub(crate) const DOC_MAIN: &str = include_str!("../../docs/docs.md");

pub use app::App;
pub use types::{Focus, FuzzyMode, NotificationType};
pub use workspace::{Workspace, WorkspaceList};
