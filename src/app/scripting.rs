use std::path::PathBuf;
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;

use super::App;
use crate::lua::{run_script, LuaContext, ScriptRequest, ScriptResponse};

impl App {
    pub fn start_script(&mut self, script: String, ctx: LuaContext, path: Option<PathBuf>) {
        let (_req_tx, req_rx) = channel();
        let (res_tx, res_rx) = channel();
        let (act_tx, act_rx) = channel();

        self.script_request_rx = Some(req_rx);
        self.script_response_tx = Some(res_tx);
        self.script_action_rx = Some(act_rx);

        let res_rx = Arc::new(Mutex::new(res_rx));

        // We spawn scripts in a separate thread so that long-running or blocking Lua code 
        // doesn't freeze the main UI event loop.
        thread::spawn(move || {
            let res_rx_clone = res_rx.clone();
            let request_handler = Arc::new(move |_req: ScriptRequest| {
                res_rx_clone
                    .lock()
                    .unwrap()
                    .recv()
                    .unwrap_or(ScriptResponse::NoResponse)
            });

            match run_script(&script, ctx, &path, request_handler) {
                Ok(actions) => {
                    let _ = act_tx.send(actions);
                }
                Err(_err) => {
                    // Send error as a notification action?
                    // For now just print to stderr or something, but better to show in UI.
                    // We can add a LuaAction::ShowError.
                }
            }
        });
    }

    pub fn poll_script_messages(&mut self) {
        if let Some(rx) = &self.script_action_rx {
            if let Ok(actions) = rx.try_recv() {
                if actions.is_empty() {
                    self.show_notification(
                        "Script did not perform any action".to_string(),
                        crate::app::NotificationType::Info,
                    );
                } else {
                    self.apply_lua_actions(actions);
                }
                // Cleanup channels after completion
                self.script_request_rx = None;
                self.script_response_tx = None;
                self.script_action_rx = None;
                self.needs_redraw = true;
            }
        }
    }
}
