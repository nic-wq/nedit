use std::path::PathBuf;
use std::sync::{mpsc::channel, Arc, Mutex};
use std::thread;

use super::App;
use crate::lua::{run_script, LuaContext, ScriptRequest, ScriptResponse};

impl App {
    pub fn start_script(&mut self, script: String, ctx: LuaContext, path: Option<PathBuf>) {
        let (req_tx, req_rx) = channel();
        let (res_tx, res_rx) = channel();
        let (act_tx, act_rx) = channel();

        self.script_request_rx = Some(req_rx);
        self.script_response_tx = Some(res_tx);
        self.script_action_rx = Some(act_rx);

        let req_tx_clone = req_tx.clone();
        let res_rx = Arc::new(Mutex::new(res_rx));

        thread::spawn(move || {
            let res_rx_clone = res_rx.clone();
            let request_handler = Arc::new(move |req: ScriptRequest| {
                let _ = req_tx_clone.send(req);
                res_rx_clone
                    .lock()
                    .unwrap()
                    .recv()
                    .unwrap_or(ScriptResponse::Prompt(String::new()))
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
        if let Some(rx) = &self.script_request_rx {
            if let Ok(req) = rx.try_recv() {
                match req {
                    ScriptRequest::Prompt { title, default } => {
                        self.fuzzy_query = default;
                        self.fuzzy_mode = crate::app::FuzzyMode::ScriptInput;
                        self.is_fuzzy = true;
                        self.script_request = Some(ScriptRequest::Prompt {
                            title,
                            default: String::new(),
                        });
                    }
                    ScriptRequest::Menu { title, options } => {
                        self.fuzzy_results = options.iter().map(PathBuf::from).collect();
                        self.fuzzy_mode = crate::app::FuzzyMode::ScriptMenu;
                        self.is_fuzzy = true;
                        self.fuzzy_idx = 0;
                        self.script_request = Some(ScriptRequest::Menu { title, options });
                    }
                }
            }
        }

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
            }
        }
    }
}
