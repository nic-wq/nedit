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

        let act3 = actions.clone();
        let notify_fn = lua.create_function(
            move |_,
                  (val_msg, opt_type, opt_duration): (
                mlua::Value,
                Option<mlua::Value>,
                Option<mlua::Value>,
            )| {
                let message = match val_msg {
                    mlua::Value::String(s) => s.to_str()?.to_string(),
                    mlua::Value::Integer(i) => i.to_string(),
                    mlua::Value::Number(n) => n.to_string(),
                    mlua::Value::Boolean(b) => b.to_string(),
                    other => format!("{:?}", other),
                };

                let mut kind = crate::app::NotificationType::Info;
                let mut raw_duration: Option<f64> = None;

                if let Some(val) = opt_type {
                    match val {
                        mlua::Value::String(s) => {
                            if let Ok(s_str) = s.to_str() {
                                if s_str.eq_ignore_ascii_case("error") {
                                    kind = crate::app::NotificationType::Error;
                                }
                            }
                        }
                        mlua::Value::Integer(i) => {
                            raw_duration = Some(i as f64);
                        }
                        mlua::Value::Number(n) => {
                            raw_duration = Some(n);
                        }
                        _ => {}
                    }
                }

                if raw_duration.is_none() {
                    if let Some(val) = opt_duration {
                        match val {
                            mlua::Value::Integer(i) => {
                                raw_duration = Some(i as f64);
                            }
                            mlua::Value::Number(n) => {
                                raw_duration = Some(n);
                            }
                            mlua::Value::String(s) => {
                                if let Ok(s_str) = s.to_str() {
                                    if let Ok(n) = s_str.trim().parse::<f64>() {
                                        raw_duration = Some(n);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                let duration = match raw_duration {
                    Some(d) if d > 0.0 => {
                        if d >= 100.0 {
                            std::time::Duration::from_millis(d as u64)
                        } else {
                            std::time::Duration::from_secs_f64(d)
                        }
                    }
                    _ => crate::app::toast::Toast::default_duration(kind),
                };

                act3.lock().unwrap().push(LuaAction::Notify {
                    message,
                    kind,
                    duration,
                });
                Ok(())
            },
        )?;

        nedit.set("notify", notify_fn.clone())?;
        nedit.set("show_notification", notify_fn.clone())?;
        nedit.set("toast", notify_fn)?;

        lua.globals().set("nedit", nedit)?;
        lua.load(script).exec()?;

        let final_actions = actions.lock().unwrap().clone();
        Ok(final_actions)
    }

    inner(script, ctx).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NotificationType;
    use std::time::Duration;

    fn make_ctx() -> LuaContext {
        LuaContext {
            current_file: String::new(),
            current_content: String::new(),
            current_selection: String::new(),
        }
    }

    #[test]
    fn test_notify_default() {
        let actions = run_script("nedit.notify('Hello!')", make_ctx()).unwrap();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            LuaAction::Notify {
                message,
                kind,
                duration,
            } => {
                assert_eq!(message, "Hello!");
                assert_eq!(*kind, NotificationType::Info);
                assert_eq!(*duration, Duration::from_millis(4000));
            }
            _ => panic!("Expected Notify action"),
        }
    }

    #[test]
    fn test_notify_with_error_and_duration_seconds() {
        let actions =
            run_script("nedit.notify('Something failed', 'error', 2.5)", make_ctx()).unwrap();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            LuaAction::Notify {
                message,
                kind,
                duration,
            } => {
                assert_eq!(message, "Something failed");
                assert_eq!(*kind, NotificationType::Error);
                assert_eq!(*duration, Duration::from_millis(2500));
            }
            _ => panic!("Expected Notify action"),
        }
    }

    #[test]
    fn test_notify_with_millis() {
        let actions = run_script("nedit.notify('Done', 'info', 5000)", make_ctx()).unwrap();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            LuaAction::Notify {
                message,
                kind,
                duration,
            } => {
                assert_eq!(message, "Done");
                assert_eq!(*kind, NotificationType::Info);
                assert_eq!(*duration, Duration::from_millis(5000));
            }
            _ => panic!("Expected Notify action"),
        }
    }

    #[test]
    fn test_notify_omitted_type_with_duration() {
        let actions = run_script("nedit.notify('Quick message', 3)", make_ctx()).unwrap();
        assert_eq!(actions.len(), 1);
        match &actions[0] {
            LuaAction::Notify {
                message,
                kind,
                duration,
            } => {
                assert_eq!(message, "Quick message");
                assert_eq!(*kind, NotificationType::Info);
                assert_eq!(*duration, Duration::from_millis(3000));
            }
            _ => panic!("Expected Notify action"),
        }
    }

    #[test]
    fn test_notify_aliases() {
        let script = r#"
            nedit.show_notification('Msg 1', 'info', 1)
            nedit.toast('Msg 2', 'error', 2)
        "#;
        let actions = run_script(script, make_ctx()).unwrap();
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].description(), "Show notification");
        assert_eq!(actions[1].description(), "Show notification");
    }
}
