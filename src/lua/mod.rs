mod actions;
mod context;
mod lua;

pub use actions::{LuaAction, RevertAction, ScriptRequest, ScriptResponse, ScriptUndo};
pub use context::LuaContext;
// The lua module provides the engine for user-defined automation, supporting both 
// background batch processing and live scripts for interactive file editing.
pub use lua::{run_script, run_script_no_interactive};
