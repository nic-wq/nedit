use std::sync::{Arc, Mutex};

use mlua::Lua;

use super::{LuaAction, LuaContext};

/// Live script API: the script pane always operates on the target buffer,
/// so the engine only exposes reading the target state (`current_file`,
/// `current_content`, `selection`) and writing it back (`write_selection`,
/// `write_current_file`). Everything is applied by the caller, which also
/// owns undo.
pub fn run_script(script: &str, ctx: LuaContext) -> Result<Vec<LuaAction>, String> {
    // We use an inner closure that returns mlua::Result to simplify error handling
    // using the '?' operator while still providing a consistent Result<_, String> to the caller.
    fn inner(script: &str, ctx: LuaContext) -> mlua::Result<Vec<LuaAction>> {
        let lua = Lua::new();
        let actions = Arc::new(Mutex::new(Vec::new()));

        let nedit = lua.create_table()?;

        let c_file = ctx.current_file.clone();
        nedit.set(
            "current_file",
            lua.create_function(move |_, ()| Ok(c_file.clone()))?,
        )?;

        let c_content = ctx.current_content.clone();
        nedit.set(
            "current_content",
            lua.create_function(move |_, ()| Ok(c_content.clone()))?,
        )?;

        let c_sel = ctx.current_selection.clone();
        nedit.set(
            "selection",
            lua.create_function(move |_, ()| Ok(c_sel.clone()))?,
        )?;

        let act1 = actions.clone();
        nedit.set(
            "write_selection",
            lua.create_function(move |_, text: String| {
                act1.lock().unwrap().push(LuaAction::WriteSelection(text));
                Ok(())
            })?,
        )?;

        let act2 = actions.clone();
        nedit.set(
            "write_current_file",
            lua.create_function(move |_, text: String| {
                act2.lock().unwrap().push(LuaAction::WriteCurrentFile(text));
                Ok(())
            })?,
        )?;

        lua.globals().set("nedit", nedit)?;
        lua.load(script).exec()?;

        let final_actions = actions.lock().unwrap().clone();
        Ok(final_actions)
    }

    inner(script, ctx).map_err(|e| e.to_string())
}
