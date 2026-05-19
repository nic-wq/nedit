mod app;
mod file_ops;
mod fuzzy;
mod live_script;
mod scripting;
mod theme;
mod types;

// We include documentation files as static strings to ensure they are always available 
// within the binary, even if the user hasn't properly installed the docs folder.
pub(crate) const DOC_LUA: &str = include_str!("../../docs/lua.md");
pub(crate) const DOC_BINDS: &str = include_str!("../../docs/binds.md");
pub(crate) const DOC_MAIN: &str = include_str!("../../docs/docs.md");

pub use app::App;
pub use types::{Focus, FuzzyMode, NotificationType};
