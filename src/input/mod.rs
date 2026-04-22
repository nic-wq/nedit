mod templates;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, Focus};

#[allow(unused_imports)]
pub use templates::LUA_TEMPLATE;

pub fn handle_events(app: &mut App) -> anyhow::Result<()> {
    if event::poll(std::time::Duration::from_millis(16))? {
        match event::read()? {
            Event::Key(key) => handle_key_event(app, key),
            _ => {}
        }
    }
    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    if app.config.matches(key, "quit") {
        app.should_quit = true;
        return;
    }
    if app.config.matches(key, "select_all") {
        if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
            buffer.select_all();
        }
        return;
    }
    if app.config.matches(key, "select_line") {
        if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
            buffer.select_line();
        }
        return;
    }
    if app.config.matches(key, "toggle_explorer") {
        app.toggle_explorer();
        return;
    }
    if app.config.matches(key, "new_file") {
        if app.focus == Focus::Explorer {
            app.toggle_fuzzy(crate::app::FuzzyMode::NewFolder);
        } else {
            app.new_file();
        }
        return;
    }
    if app.config.matches(key, "close_tab") {
        app.close_current_buffer();
        return;
    }
    if app.config.matches(key, "open_file") {
        app.toggle_fuzzy(crate::app::FuzzyMode::Files);
        return;
    }
    if app.config.matches(key, "global_search") {
        app.toggle_fuzzy(crate::app::FuzzyMode::Content);
        return;
    }
    if app.config.matches(key, "local_search") {
        app.toggle_fuzzy(crate::app::FuzzyMode::Local);
        return;
    }
    if app.config.matches(key, "theme_select") {
        app.toggle_fuzzy(crate::app::FuzzyMode::Themes);
        return;
    }
    if app.config.matches(key, "open_help") {
        app.open_docs();
        return;
    }
    if app.config.matches(key, "toggle_focus") {
        app.focus = match app.focus {
            Focus::Explorer => Focus::Editor,
            Focus::Editor => Focus::Explorer,
        };
        return;
    }

    if key.code == KeyCode::Char('w')
        && key
            .modifiers
            .contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        app.toggle_fuzzy(crate::app::FuzzyMode::Workspaces);
        app.refresh_workspace_results();
        return;
    }

    if app.config.matches(key, "command_palette") {
        app.toggle_fuzzy(crate::app::FuzzyMode::CommandPalette);
        return;
    }

    if app.config.matches(key, "run_live_script") {
        handle_run_live_script(app);
        return;
    }

    if app.config.matches(key, "live_script_next") {
        if app.live_script_mode {
            if let (Some(target), Some(script)) =
                (app.target_buffer_idx, app.live_script_buffer_idx)
            {
                app.current_buffer_idx = if app.current_buffer_idx == target {
                    script
                } else {
                    target
                };
            }
        }
        return;
    }

    if app.config.matches(key, "live_script_prev") {
        if app.live_script_mode {
            if let (Some(target), Some(script)) =
                (app.target_buffer_idx, app.live_script_buffer_idx)
            {
                app.current_buffer_idx = if app.current_buffer_idx == script {
                    target
                } else {
                    script
                };
            }
        }
        return;
    }

    let can_switch_tabs = !app.live_script_mode
        || (app.live_script_mode
            && app.current_buffer_idx != app.live_script_buffer_idx.unwrap_or(usize::MAX));

    if can_switch_tabs {
        match (key.code, key.modifiers) {
            (KeyCode::Tab, KeyModifiers::CONTROL) => {
                app.switch_tab_relative(1);
                return;
            }
            (KeyCode::Tab, m)
                if m.contains(KeyModifiers::CONTROL) && m.contains(KeyModifiers::SHIFT) =>
            {
                app.switch_tab_relative(-1);
                return;
            }
            (KeyCode::Left, m) if m.contains(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                app.switch_tab_relative(-1);
                return;
            }
            (KeyCode::Right, m) if m.contains(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
                app.switch_tab_relative(1);
                return;
            }
            (KeyCode::Char(c), KeyModifiers::ALT) if c.is_digit(10) => {
                let idx = c.to_digit(10).unwrap() as usize;
                if idx > 0 {
                    app.switch_tab(idx - 1);
                }
                return;
            }
            _ => {}
        }
    }

    if app.is_fuzzy {
        handle_fuzzy_input(app, key);
        return;
    }

    match app.focus {
        Focus::Explorer => handle_explorer_input(app, key),
        Focus::Editor => {
            if !app.is_welcome && !app.buffers.is_empty() {
                handle_editor_input(app, key)
            }
        }
    }
}

fn handle_fuzzy_input(app: &mut App, key: KeyEvent) {
    if matches!(
        app.fuzzy_mode,
        crate::app::FuzzyMode::WorkspaceAddName
            | crate::app::FuzzyMode::WorkspaceAddPath
            | crate::app::FuzzyMode::Rename
            | crate::app::FuzzyMode::SaveAs
            | crate::app::FuzzyMode::NewFolder
    ) {
        match key.code {
            KeyCode::Esc => {
                app.is_fuzzy = false;
                app.fuzzy_query.clear();
                app.pending_path = None;
                app.move_dir = None;
                app.temp_ws_name = None;
            }
            KeyCode::Enter => {}
            KeyCode::Backspace => {
                if !app.fuzzy_query.is_empty() {
                    app.fuzzy_query.pop();
                }
            }
            KeyCode::Char(c) => {
                app.fuzzy_query.push(c);
            }
            _ => {}
        }
        if key.code != KeyCode::Enter {
            return;
        }
    }

    if app.fuzzy_mode == crate::app::FuzzyMode::Workspaces
        && key.code == KeyCode::Char('x')
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        if let Some(opt) = app.fuzzy_results.get(app.fuzzy_idx) {
            let name = opt.to_string_lossy().to_string();
            if name == "Exit Workspace" || name == "New Workspace..." {
                return;
            }
            app.workspaces.retain(|w| w.name != name);
            app.save_workspaces();
            app.refresh_workspace_results();
            if app.fuzzy_idx >= app.fuzzy_results.len() {
                app.fuzzy_idx = app.fuzzy_results.len().saturating_sub(1);
            }
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            if app.fuzzy_mode == crate::app::FuzzyMode::Themes {
                app.current_theme = app.original_theme.clone();
            }
            app.pending_path = None;
            app.move_dir = None;
            app.temp_ws_name = None;
            app.clear_notification();
            app.is_fuzzy = false;
        }
        KeyCode::Up => {
            if app.fuzzy_idx > 0 {
                app.fuzzy_idx -= 1;
                if app.fuzzy_mode == crate::app::FuzzyMode::Themes {
                    if let Some(theme) = app.fuzzy_themes.get(app.fuzzy_idx) {
                        app.apply_theme(theme.clone());
                    }
                }
            }
        }
        KeyCode::Down => {
            let max = match app.fuzzy_mode {
                crate::app::FuzzyMode::Local => app.fuzzy_lines.len(),
                crate::app::FuzzyMode::Content => app.fuzzy_global_results.len(),
                crate::app::FuzzyMode::Files => app.fuzzy_results.len(),
                crate::app::FuzzyMode::Themes => app.fuzzy_themes.len(),
                crate::app::FuzzyMode::SaveAs => 0,
                crate::app::FuzzyMode::Rename => 0,
                crate::app::FuzzyMode::DeleteConfirm => 0,
                crate::app::FuzzyMode::FileOptions => app.fuzzy_results.len(),
                crate::app::FuzzyMode::Workspaces => app.fuzzy_results.len(),
                crate::app::FuzzyMode::WorkspaceAddName => 0,
                crate::app::FuzzyMode::WorkspaceAddPath => 0,
                crate::app::FuzzyMode::CommandPalette => app.fuzzy_results.len(),
                crate::app::FuzzyMode::Move => app.fuzzy_results.len(),
                crate::app::FuzzyMode::RunScript => app.fuzzy_results.len(),
                crate::app::FuzzyMode::ScriptConfirm => app.pending_lua_actions.len(),
                crate::app::FuzzyMode::EditScript => app.fuzzy_results.len(),
                crate::app::FuzzyMode::DeleteScript => app.fuzzy_results.len(),
                crate::app::FuzzyMode::DocSelect => app.fuzzy_results.len(),
                crate::app::FuzzyMode::NewFolder => 0,
            };
            if max > 0 && app.fuzzy_idx < max - 1 {
                app.fuzzy_idx += 1;
                if app.fuzzy_mode == crate::app::FuzzyMode::Themes {
                    if let Some(theme) = app.fuzzy_themes.get(app.fuzzy_idx) {
                        app.apply_theme(theme.clone());
                    }
                }
            }
        }
        KeyCode::Enter => {
            if app.fuzzy_mode == crate::app::FuzzyMode::CommandPalette {
                if let Some(cmd) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    let keep_modal =
                        handle_command_palette_selection(app, cmd.to_string_lossy().as_ref());
                    if !keep_modal {
                        app.is_fuzzy = false;
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::FileOptions {
                if let Some(choice) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    let Some(item) = app.explorer.get_selected() else {
                        app.is_fuzzy = false;
                        return;
                    };
                    app.pending_path = Some(item.path.clone());
                    match choice.to_string_lossy().as_ref() {
                        "Rename" => {
                            app.fuzzy_mode = crate::app::FuzzyMode::Rename;
                            app.fuzzy_query = item.name.clone();
                        }
                        "Move" => {
                            app.fuzzy_mode = crate::app::FuzzyMode::Move;
                            app.move_dir = item.path.parent().map(|p| p.to_path_buf());
                            app.fuzzy_query.clear();
                            app.update_fuzzy();
                        }
                        "Delete" => {
                            app.fuzzy_mode = crate::app::FuzzyMode::DeleteConfirm;
                            app.fuzzy_query.clear();
                        }
                        _ => app.is_fuzzy = false,
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Workspaces {
                if let Some(path) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    let choice = path.to_string_lossy().to_string();
                    match choice.as_str() {
                        "New Workspace..." => {
                            app.temp_ws_name = None;
                            app.fuzzy_mode = crate::app::FuzzyMode::WorkspaceAddName;
                            app.fuzzy_query.clear();
                            app.fuzzy_results.clear();
                            app.fuzzy_idx = 0;
                        }
                        "Exit Workspace" => {
                            app.exit_workspace();
                            app.is_fuzzy = false;
                        }
                        name => {
                            app.switch_workspace(name.to_string());
                            app.is_fuzzy = false;
                        }
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::WorkspaceAddName {
                let name = app.fuzzy_query.trim().to_string();
                if name.is_empty() {
                    app.show_notification(
                        "Workspace name cannot be empty".to_string(),
                        crate::app::NotificationType::Error,
                    );
                } else {
                    app.temp_ws_name = Some(name);
                    app.fuzzy_mode = crate::app::FuzzyMode::WorkspaceAddPath;
                    app.fuzzy_query = app.explorer.root.to_string_lossy().to_string();
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::WorkspaceAddPath {
                let Some(name) = app.temp_ws_name.take() else {
                    app.show_notification(
                        "Workspace name is missing".to_string(),
                        crate::app::NotificationType::Error,
                    );
                    app.is_fuzzy = false;
                    return;
                };
                let raw_path = app.fuzzy_query.trim();
                let path = if raw_path.is_empty() {
                    app.explorer.root.clone()
                } else {
                    app.resolve_input_path(raw_path)
                };
                match app.create_workspace(name.clone(), path) {
                    Ok(()) => app.is_fuzzy = false,
                    Err(err) => {
                        app.temp_ws_name = Some(name);
                        app.show_notification(err, crate::app::NotificationType::Error);
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Rename {
                if let Some(old_path) = app.pending_path.take() {
                    let new_name = app.fuzzy_query.trim();
                    if new_name.is_empty() {
                        app.show_notification(
                            "New name cannot be empty".to_string(),
                            crate::app::NotificationType::Error,
                        );
                        app.pending_path = Some(old_path);
                        return;
                    }
                    if let Some(parent) = old_path.parent() {
                        let new_path = parent.join(new_name);
                        match std::fs::rename(&old_path, &new_path) {
                            Ok(()) => {
                                app.update_buffer_paths(&old_path, &new_path);
                                app.explorer.refresh();
                                app.show_notification(
                                    format!("Renamed to {}", new_path.display()),
                                    crate::app::NotificationType::Info,
                                );
                                app.is_fuzzy = false;
                            }
                            Err(err) => {
                                app.pending_path = Some(old_path);
                                app.show_notification(
                                    format!("Error renaming file: {}", err),
                                    crate::app::NotificationType::Error,
                                );
                            }
                        }
                    } else {
                        app.show_notification(
                            "Cannot rename this item".to_string(),
                            crate::app::NotificationType::Error,
                        );
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::DeleteConfirm {
                if let Some(path) = app.pending_path.take() {
                    let result = if path.is_dir() {
                        std::fs::remove_dir_all(&path)
                    } else {
                        std::fs::remove_file(&path)
                    };
                    match result {
                        Ok(()) => {
                            app.close_buffers_for_path(&path);
                            app.explorer.refresh();
                            app.show_notification(
                                format!("Deleted {}", path.display()),
                                crate::app::NotificationType::Info,
                            );
                            app.is_fuzzy = false;
                        }
                        Err(err) => {
                            app.pending_path = Some(path);
                            app.show_notification(
                                format!("Error deleting file: {}", err),
                                crate::app::NotificationType::Error,
                            );
                        }
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Move {
                if let Some(path) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    if path == std::path::PathBuf::from("..") {
                        if let Some(parent) = app
                            .move_dir
                            .as_ref()
                            .and_then(|dir| dir.parent())
                            .map(|p| p.to_path_buf())
                        {
                            app.move_dir = Some(parent);
                            app.update_fuzzy();
                        }
                    } else if path.is_dir() {
                        app.move_dir = Some(path);
                        app.update_fuzzy();
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::RunScript {
                if let Some(script_path) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    match std::fs::read_to_string(&script_path) {
                        Ok(script) => {
                            let cur_buf = &app.buffers[app.current_buffer_idx];
                            let ctx = crate::lua::LuaContext {
                                current_file: cur_buf
                                    .path
                                    .as_ref()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_default(),
                                current_content: cur_buf.content.to_string(),
                                current_selection: cur_buf.get_selected_text().unwrap_or_default(),
                                current_dir: app.explorer.root.clone(),
                                is_live_script: false,
                            };
                            match crate::lua::run_script(&script, ctx, &cur_buf.path) {
                                Ok(actions) => {
                                    if actions.is_empty() {
                                        app.show_notification(
                                            "Script did not perform any action".to_string(),
                                            crate::app::NotificationType::Info,
                                        );
                                        app.is_fuzzy = false;
                                        return;
                                    }
                                    app.pending_lua_actions = actions;
                                    app.is_fuzzy = true;
                                    app.fuzzy_mode = crate::app::FuzzyMode::ScriptConfirm;
                                    app.fuzzy_query.clear();
                                    app.fuzzy_idx = 0;
                                }
                                Err(err) => {
                                    let mut err_buf = crate::buffer::EditorBuffer::new();
                                    err_buf.content =
                                        ropey::Rope::from_str(&format!("Lua Error:\n{}", err));
                                    err_buf.is_read_only = true;
                                    app.buffers.push(err_buf);
                                    app.current_buffer_idx = app.buffers.len() - 1;
                                    app.is_fuzzy = false;
                                }
                            }
                        }
                        Err(err) => {
                            let mut err_buf = crate::buffer::EditorBuffer::new();
                            err_buf.content =
                                ropey::Rope::from_str(&format!("Could not read script:\n{}", err));
                            err_buf.is_read_only = true;
                            app.buffers.push(err_buf);
                            app.current_buffer_idx = app.buffers.len() - 1;
                            app.is_fuzzy = false;
                        }
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::EditScript {
                if let Some(path) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    app.open_file(path);
                    app.is_fuzzy = false;
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::DeleteScript {
                if let Some(path) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    match std::fs::remove_file(&path) {
                        Ok(_) => {
                            app.close_buffers_for_path(&path);
                            app.show_notification(
                                format!(
                                    "Script '{}' deleted successfully",
                                    path.file_name().unwrap_or_default().to_string_lossy()
                                ),
                                crate::app::NotificationType::Info,
                            );
                            app.explorer.refresh();
                        }
                        Err(err) => {
                            app.show_notification(
                                format!("Error deleting script: {}", err),
                                crate::app::NotificationType::Error,
                            );
                        }
                    }
                    app.is_fuzzy = false;
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::DocSelect {
                if let Some(path) = app.fuzzy_results.get(app.fuzzy_idx) {
                    let path_str = path.to_string_lossy().to_string();
                    let doc_type = if path_str.contains("lua") {
                        "lua"
                    } else if path_str.contains("binds") {
                        "binds"
                    } else {
                        "general"
                    };
                    app.open_doc(doc_type);
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::NewFolder {
                if !app.fuzzy_query.is_empty() {
                    let path = app.resolve_input_path(&app.fuzzy_query);
                    if let Err(e) = std::fs::create_dir_all(&path) {
                        app.show_notification(
                            format!("Error creating folder: {}", e),
                            crate::app::NotificationType::Error,
                        );
                    } else {
                        app.show_notification(
                            format!("Folder created: {}", app.fuzzy_query),
                            crate::app::NotificationType::Info,
                        );
                        app.explorer.refresh();
                    }
                }
                app.is_fuzzy = false;
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::ScriptConfirm {
                let actions = std::mem::take(&mut app.pending_lua_actions);
                apply_lua_actions(app, actions);
                app.is_fuzzy = false;
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::SaveAs {
                if !app.fuzzy_query.is_empty() {
                    let mut filename = app.fuzzy_query.trim().to_string();
                    let content = app.buffers[app.current_buffer_idx].content.to_string();
                    let is_lua_script = content
                        .lines()
                        .next()
                        .map(|l| l.trim().starts_with("-- Name:"))
                        .unwrap_or(false);
                    let is_live_script = Some(app.current_buffer_idx) == app.live_script_buffer_idx;

                    if is_lua_script || is_live_script {
                        if !filename.ends_with(".lua") {
                            filename.push_str(".lua");
                        }
                        let scripts_dir = std::env::var("HOME")
                            .map(std::path::PathBuf::from)
                            .unwrap_or_else(|_| std::path::PathBuf::from("."))
                            .join(".config/nedit/scripts");
                        let _ = std::fs::create_dir_all(&scripts_dir);
                        let path = scripts_dir.join(filename);
                        if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                            buffer.path = Some(path);
                            if let Err(err) = buffer.save() {
                                app.show_notification(
                                    format!("Could not save script: {}", err),
                                    crate::app::NotificationType::Error,
                                );
                                return;
                            }
                        }
                    } else {
                        let path = app.resolve_input_path(&filename);
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                            buffer.path = Some(path);
                            if let Err(err) = buffer.save() {
                                app.show_notification(
                                    format!("Could not save file: {}", err),
                                    crate::app::NotificationType::Error,
                                );
                                return;
                            }
                        }
                        app.explorer.refresh();
                    }
                    app.is_fuzzy = false;
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Local {
                if let Some((line_idx, _)) = app.fuzzy_lines.get(app.fuzzy_idx) {
                    if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                        buffer.cursor_row = *line_idx;
                        buffer.cursor_col = 0;
                    }
                }
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Content {
                if let Some((path, line_idx, _)) = app.fuzzy_global_results.get(app.fuzzy_idx) {
                    let path = path.clone();
                    let line_idx = *line_idx;
                    app.open_file(path);
                    if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                        buffer.cursor_row = line_idx;
                        buffer.cursor_col = 0;
                    }
                }
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Themes {
                if let Some(theme) = app.fuzzy_themes.get(app.fuzzy_idx) {
                    app.apply_theme(theme.clone());
                    app.save_current_theme();
                }
            } else {
                if let Some(path) = app.fuzzy_results.get(app.fuzzy_idx) {
                    app.open_file(path.clone());
                }
            }
            app.is_fuzzy = false;
        }
        KeyCode::Char(c) => {
            app.fuzzy_query.push(c);
            app.update_fuzzy();
        }
        KeyCode::Backspace => {
            app.fuzzy_query.pop();
            app.update_fuzzy();
        }
        KeyCode::Tab => {
            if app.fuzzy_mode == crate::app::FuzzyMode::Move {
                if let (Some(old_path), Some(dest_dir)) =
                    (app.pending_path.clone(), app.move_dir.clone())
                {
                    let dest_path = dest_dir.join(old_path.file_name().unwrap_or_default());
                    match std::fs::rename(&old_path, &dest_path) {
                        Ok(()) => {
                            app.update_buffer_paths(&old_path, &dest_path);
                            app.explorer.refresh();
                            app.pending_path = None;
                            app.is_fuzzy = false;
                            app.show_notification(
                                format!("Moved to {}", dest_path.display()),
                                crate::app::NotificationType::Info,
                            );
                        }
                        Err(err) => app.show_notification(
                            format!("Error moving file: {}", err),
                            crate::app::NotificationType::Error,
                        ),
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_explorer_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => app.explorer.previous(),
        KeyCode::Down => app.explorer.next(),
        KeyCode::Enter => {
            if let Some(item) = app.explorer.get_selected() {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if item.is_dir {
                        app.set_explorer_root(item.path.clone());
                    }
                } else if item.is_dir {
                    app.explorer.toggle_expand();
                } else {
                    app.open_file(item.path.clone());
                }
            }
        }
        KeyCode::Backspace => app.explorer.go_up_root(),
        KeyCode::Char('O') if key.modifiers.contains(KeyModifiers::SHIFT) => {
            if let Some(_item) = app.explorer.get_selected() {
                app.toggle_fuzzy(crate::app::FuzzyMode::FileOptions);
                app.fuzzy_results = vec![
                    std::path::PathBuf::from("Rename"),
                    std::path::PathBuf::from("Move"),
                    std::path::PathBuf::from("Delete"),
                ];
                app.fuzzy_idx = 0;
            }
        }
        _ => {}
    }
}

fn handle_editor_input(app: &mut App, key: KeyEvent) {
    let buffer = &mut app.buffers[app.current_buffer_idx];

    match (key.code, key.modifiers) {
        (KeyCode::Right, m)
            if m.contains(KeyModifiers::SHIFT) && !buffer.autocomplete_options.is_empty() =>
        {
            buffer.accept_autocomplete();
        }
        (KeyCode::Esc, _) if buffer.show_autocomplete_list => {
            buffer.show_autocomplete_list = false;
        }
        (KeyCode::Up, m) if m == KeyModifiers::CONTROL => {}
        (KeyCode::Down, m) if m == KeyModifiers::CONTROL => {}
        (KeyCode::Up, m) => {
            if m.contains(KeyModifiers::SHIFT) && buffer.selection_start.is_none() {
                buffer.selection_start = Some((buffer.cursor_row, buffer.cursor_col));
            } else if !m.contains(KeyModifiers::SHIFT) {
                buffer.selection_start = None;
            }
            buffer.move_cursor(-1, 0, 80);
        }
        (KeyCode::Down, m) => {
            if m.contains(KeyModifiers::SHIFT) && buffer.selection_start.is_none() {
                buffer.selection_start = Some((buffer.cursor_row, buffer.cursor_col));
            } else if !m.contains(KeyModifiers::SHIFT) {
                buffer.selection_start = None;
            }
            buffer.move_cursor(1, 0, 80);
        }
        (KeyCode::Left, m) => {
            if m.contains(KeyModifiers::SHIFT) && buffer.selection_start.is_none() {
                buffer.selection_start = Some((buffer.cursor_row, buffer.cursor_col));
            } else if !m.contains(KeyModifiers::SHIFT) {
                buffer.selection_start = None;
            }
            if m.contains(KeyModifiers::CONTROL) {
                buffer.move_word(-1);
            } else {
                buffer.move_cursor(0, -1, 80);
            }
        }
        (KeyCode::Right, m) => {
            if m.contains(KeyModifiers::SHIFT) && buffer.selection_start.is_none() {
                buffer.selection_start = Some((buffer.cursor_row, buffer.cursor_col));
            } else if !m.contains(KeyModifiers::SHIFT) {
                buffer.selection_start = None;
            }
            if m.contains(KeyModifiers::CONTROL) {
                buffer.move_word(1);
            } else {
                buffer.move_cursor(0, 1, 80);
            }
        }
        (KeyCode::Home, _) => buffer.move_to_line_start(),
        (KeyCode::End, _) => buffer.move_to_line_end(),
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT)
            if !buffer.is_read_only =>
        {
            if buffer.selection_start.is_some() {
                buffer.delete_selection();
            }
            buffer.insert_char(c);
            if app.config.autocomplete_enabled {
                buffer.update_autocomplete();
            }
        }
        (KeyCode::Enter, _) if !buffer.is_read_only => {
            if buffer.selection_start.is_some() {
                buffer.delete_selection();
            }
            buffer.insert_char('\n');
        }
        (KeyCode::Backspace, _) if !buffer.is_read_only => {
            if buffer.selection_start.is_some() {
                buffer.delete_selection();
            } else {
                buffer.delete_backspace();
            }
            if app.config.autocomplete_enabled {
                buffer.update_autocomplete();
            }
        }
        _ if app.config.matches(key, "save") && !buffer.is_read_only => {
            app.save_current_buffer();
        }
        _ if app.config.matches(key, "undo") && !buffer.is_read_only => buffer.undo(),
        _ if app.config.matches(key, "redo") && !buffer.is_read_only => buffer.redo(),
        _ if app.config.matches(key, "copy") => {
            app.buffers[app.current_buffer_idx].copy();
        }
        _ if app.config.matches(key, "paste") && !buffer.is_read_only => {
            app.buffers[app.current_buffer_idx].paste();
        }
        _ if app.config.matches(key, "cut") && !buffer.is_read_only => {
            app.buffers[app.current_buffer_idx].cut();
        }
        (KeyCode::Tab, KeyModifiers::NONE) if !buffer.is_read_only => {
            for _ in 0..4 {
                buffer.insert_char(' ');
            }
        }
        _ => {}
    }
}

fn handle_command_palette_selection(app: &mut App, cmd: &str) -> bool {
    match cmd {
        "Save" => app.save_current_buffer(),
        "New File" => app.new_file(),
        "Open File" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::Files);
            return true;
        }
        "Close Tab" => app.close_current_buffer(),
        "Toggle Explorer" => app.toggle_explorer(),
        "Global Search" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::Content);
            return true;
        }
        "Local Search" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::Local);
            return true;
        }
        "Switch Theme" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::Themes);
            return true;
        }
        "Workspaces" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::Workspaces);
            app.refresh_workspace_results();
            return true;
        }
        "Open Lua Script" | "Run Lua Script" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::RunScript);
            return true;
        }
        "Edit Lua Script" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::EditScript);
            return true;
        }
        "Delete Lua Script" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::DeleteScript);
            return true;
        }
        "Open Live Script" => app.open_live_script(),
        "Quit" => app.should_quit = true,
        "Undo" => {
            if let Some(buf) = app.buffers.get_mut(app.current_buffer_idx) {
                buf.undo();
            }
        }
        "Redo" => {
            if let Some(buf) = app.buffers.get_mut(app.current_buffer_idx) {
                buf.redo();
            }
        }
        "Copy" => {
            if let Some(buf) = app.buffers.get_mut(app.current_buffer_idx) {
                buf.copy();
            }
        }
        "Paste" => {
            if let Some(buf) = app.buffers.get_mut(app.current_buffer_idx) {
                buf.paste();
            }
        }
        "Cut" => {
            if let Some(buf) = app.buffers.get_mut(app.current_buffer_idx) {
                buf.cut();
            }
        }
        "Select All" => {
            if let Some(buf) = app.buffers.get_mut(app.current_buffer_idx) {
                buf.select_all();
            }
        }
        "Open Help" => {
            app.open_docs();
            return true;
        }
        _ => {}
    }
    false
}

fn apply_lua_actions(app: &mut App, actions: Vec<crate::lua::LuaAction>) {
    if actions.is_empty() {
        return;
    }

    let target_idx = if app.live_script_mode {
        app.target_buffer_idx.unwrap_or(app.current_buffer_idx)
    } else {
        app.current_buffer_idx
    };

    for action in actions {
        match action {
            crate::lua::LuaAction::WriteSelection(text) => {
                if let Some(buf) = app.buffers.get_mut(target_idx) {
                    if buf.selection_start.is_none() {
                        app.show_notification(
                            "Error: write_selection requires selected text.".to_string(),
                            crate::app::NotificationType::Error,
                        );
                        continue;
                    }
                    buf.delete_selection();
                    for c in text.chars() {
                        buf.insert_char(c);
                    }
                }
            }
            crate::lua::LuaAction::WriteCurrentFile(text) => {
                if app.live_script_mode {
                    if let Some(target_buf) = app.buffers.get(target_idx) {
                        if target_buf.path.is_none() {
                            app.show_notification(
                                "Error: target file has no path".to_string(),
                                crate::app::NotificationType::Error,
                            );
                            continue;
                        }
                    }
                }
                if let Some(buf) = app.buffers.get_mut(target_idx) {
                    buf.content = ropey::Rope::from_str(&text);
                    buf.cursor_row = 0;
                    buf.cursor_col = 0;
                }
            }
            crate::lua::LuaAction::WriteFile(path, text) => {
                let _ = std::fs::write(&path, text);
            }
            crate::lua::LuaAction::CreateFile(path, text) => {
                let _ = std::fs::write(&path, text);
            }
            crate::lua::LuaAction::DeleteFile(path) => {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    app.explorer.refresh();
}

fn handle_run_live_script(app: &mut App) {
    if !app.live_script_mode {
        return;
    }
    let script_idx = match app.live_script_buffer_idx {
        Some(idx) => idx,
        None => return,
    };
    let target_idx = match app.target_buffer_idx {
        Some(idx) => idx,
        None => return,
    };

    let script = app.buffers[script_idx].content.to_string();
    let target_buf = &app.buffers[target_idx];

    let ctx = crate::lua::LuaContext {
        current_file: target_buf
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        current_content: target_buf.content.to_string(),
        current_selection: target_buf.get_selected_text().unwrap_or_default(),
        current_dir: app.explorer.root.clone(),
        is_live_script: true,
    };

    let target_path = target_buf.path.clone();

    match crate::lua::run_script(&script, ctx, &target_path) {
        Ok(actions) => {
            if actions.is_empty() {
                app.show_notification(
                    "Script executed successfully".to_string(),
                    crate::app::NotificationType::Info,
                );
                return;
            }
            apply_lua_actions(app, actions);
            app.show_notification(
                "Script executed successfully".to_string(),
                crate::app::NotificationType::Info,
            );
        }
        Err(err) => {
            app.show_notification(
                format!("Lua Error: {}", err),
                crate::app::NotificationType::Error,
            );
        }
    }
}
