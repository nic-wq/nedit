use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use mlua::Lua;

use super::{LuaAction, LuaContext, ScriptRequest, ScriptResponse};

pub fn run_script(
    script: &str,
    ctx: LuaContext,
    current_buffer_path: &Option<PathBuf>,
    request_handler: Arc<dyn Fn(ScriptRequest) -> ScriptResponse + Send + Sync>,
) -> Result<Vec<LuaAction>, String> {
    fn inner(
        script: &str,
        ctx: LuaContext,
        current_buffer_path: &Option<PathBuf>,
        request_handler: Arc<dyn Fn(ScriptRequest) -> ScriptResponse + Send + Sync>,
    ) -> mlua::Result<Vec<LuaAction>> {
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

        let c_dir = ctx.current_dir.clone();
        nedit.set(
            "list_dir",
            lua.create_function(move |_, path: Option<String>| {
                let mut results = Vec::new();
                let target = path.map(PathBuf::from).unwrap_or_else(|| c_dir.clone());
                if let Ok(entries) = std::fs::read_dir(target) {
                    for entry in entries.flatten() {
                        results.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
                Ok(results)
            })?,
        )?;

        let c_dir2 = ctx.current_dir.clone();
        nedit.set(
            "read_file",
            lua.create_function(move |_, path: String| {
                let p = c_dir2.join(path);
                std::fs::read_to_string(p).or_else(|_| Ok("".to_string()))
            })?,
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

        let c_dir3 = ctx.current_dir.clone();
        let act3 = actions.clone();
        nedit.set(
            "write_file",
            lua.create_function(move |_, (path, text): (String, String)| {
                act3.lock()
                    .unwrap()
                    .push(LuaAction::WriteFile(c_dir3.join(path), text));
                Ok(())
            })?,
        )?;

        let c_dir4 = ctx.current_dir.clone();
        let act4 = actions.clone();
        nedit.set(
            "create_file",
            lua.create_function(move |_, (path, text): (String, String)| {
                act4.lock()
                    .unwrap()
                    .push(LuaAction::CreateFile(c_dir4.join(path), text));
                Ok(())
            })?,
        )?;

        let c_dir5 = ctx.current_dir.clone();
        let act5 = actions.clone();
        nedit.set(
            "delete_file",
            lua.create_function(move |_, path: String| {
                act5.lock()
                    .unwrap()
                    .push(LuaAction::DeleteFile(c_dir5.join(path)));
                Ok(())
            })?,
        )?;

        let req_h = request_handler.clone();
        nedit.set(
            "prompt",
            lua.create_function(move |_, (title, default): (String, Option<String>)| {
                let res = req_h(ScriptRequest::Prompt {
                    title,
                    default: default.unwrap_or_default(),
                });
                if let ScriptResponse::Prompt(val) = res {
                    Ok(Some(val))
                } else {
                    Ok(None)
                }
            })?,
        )?;

        let req_h2 = request_handler.clone();
        nedit.set(
            "menu",
            lua.create_function(move |_, (title, options): (String, Vec<String>)| {
                let res = req_h2(ScriptRequest::Menu { title, options });
                if let ScriptResponse::Menu(val) = res {
                    Ok(val)
                } else {
                    Ok(None)
                }
            })?,
        )?;

        lua.globals().set("nedit", nedit)?;
        lua.load(script).exec()?;

        let final_actions = actions.lock().unwrap().clone();

        if let Some(script_path) = current_buffer_path {
            let scripts_dir = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".config/nedit/scripts");

            let is_script_file = script_path
                .canonicalize()
                .map(|p| p.starts_with(&scripts_dir))
                .unwrap_or(false);

            if is_script_file {
                for action in &final_actions {
                    let affects_self = match action {
                        LuaAction::WriteFile(p, _)
                        | LuaAction::CreateFile(p, _)
                        | LuaAction::DeleteFile(p) => p == script_path,
                        LuaAction::WriteCurrentFile(_text) => {
                            let cur_path = ctx.current_file.clone();
                            !cur_path.is_empty() && cur_path == script_path.to_string_lossy()
                        }
                        _ => false,
                    };
                    if affects_self {
                        return Err(mlua::Error::RuntimeError(
                            "Script cannot modify itself. Use Edit Lua Script in Command Palette to edit scripts."
                                .to_string(),
                        ));
                    }
                }
            }
        }

        Ok(final_actions)
    }

    let actions = inner(script, ctx.clone(), current_buffer_path, request_handler).map_err(|e| e.to_string())?;

    if ctx.is_live_script {
        let target_path = current_buffer_path
            .as_ref()
            .and_then(|p| p.canonicalize().ok());

        for action in &actions {
            match action {
                LuaAction::WriteFile(path, _)
                | LuaAction::CreateFile(path, _)
                | LuaAction::DeleteFile(path) => {
                    if let Some(target) = &target_path {
                        let action_path = path.canonicalize().ok();
                        if action_path != Some(target.clone()) {
                            return Err(format!(
                                "Error: Live script can only modify the target file ({}), not {}",
                                target.display(),
                                path.display()
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(actions)
}

pub fn run_script_no_interactive(
    script: &str,
    ctx: LuaContext,
    current_buffer_path: &Option<PathBuf>,
) -> Result<Vec<LuaAction>, String> {
    run_script(
        script,
        ctx,
        current_buffer_path,
        Arc::new(|_| ScriptResponse::Prompt(String::new())),
    )
}
