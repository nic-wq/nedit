mod templates;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};

use crate::app::{App, Focus};

#[allow(unused_imports)]
pub use templates::LUA_TEMPLATE;

pub fn handle_events(app: &mut App) -> anyhow::Result<()> {
    if event::poll(std::time::Duration::from_millis(16))? {
        match event::read()? {
            Event::Key(key) => handle_key_event(app, key),
            Event::Mouse(mouse) => handle_mouse_event(app, mouse),
            _ => {}
        }
    }
    Ok(())
}

fn handle_mouse_event(app: &mut App, mouse: MouseEvent) {
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                buffer.scroll_row = buffer.scroll_row.saturating_sub(3);
            }
        }
        MouseEventKind::ScrollDown => {
            if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                buffer.scroll_row = buffer.scroll_row.saturating_add(3);
            }
        }
        MouseEventKind::Down(button) if button == event::MouseButton::Left => {
            if app.editor_area.contains(ratatui::layout::Position::new(mouse.column, mouse.row)) {
                app.focus = Focus::Editor;
                let rel_col = mouse.column.saturating_sub(app.editor_area.x) as usize;
                let rel_row = mouse.row.saturating_sub(app.editor_area.y) as usize;
                if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                    let target_row = buffer.scroll_row + rel_row;
                    let target_col = buffer.scroll_col + rel_col.saturating_sub(buffer.line_number_width());
                    buffer.cursor_row = target_row.min(buffer.content.len_lines().saturating_sub(1));
                    let line_len = buffer.content.line(buffer.cursor_row).len_chars().saturating_sub(1);
                    buffer.cursor_col = target_col.min(line_len);
                    buffer.selection_start = None;
                }
            } else if app.explorer_area.contains(ratatui::layout::Position::new(mouse.column, mouse.row)) {
                app.focus = Focus::Explorer;
                let rel_row = mouse.row.saturating_sub(app.explorer_area.y) as usize;
                let target_idx = app.explorer.scroll_offset + rel_row;
                if target_idx < app.explorer.items.len() {
                    app.explorer.selected_idx = target_idx;
                }
            }
        }
        MouseEventKind::Drag(button) if button == event::MouseButton::Left => {
            if app.editor_area.contains(ratatui::layout::Position::new(mouse.column, mouse.row)) {
                let rel_col = mouse.column.saturating_sub(app.editor_area.x) as usize;
                let rel_row = mouse.row.saturating_sub(app.editor_area.y) as usize;
                if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                    if buffer.selection_start.is_none() {
                        buffer.selection_start = Some((buffer.cursor_row, buffer.cursor_col));
                    }
                    let target_row = buffer.scroll_row + rel_row;
                    let target_col = buffer.scroll_col + rel_col.saturating_sub(buffer.line_number_width());
                    buffer.cursor_row = target_row.min(buffer.content.len_lines().saturating_sub(1));
                    let line_len = buffer.content.line(buffer.cursor_row).len_chars().saturating_sub(1);
                    buffer.cursor_col = target_col.min(line_len);
                }
            }
        }
        _ => {}
    }
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    if app.is_fuzzy {
        handle_fuzzy_input(app, key);
        return;
    }

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
        KeyCode::Tab => {
            if app.fuzzy_mode == crate::app::FuzzyMode::Move {
                if let (Some(old_path), Some(new_dir)) = (app.pending_path.take(), app.move_dir.take()) {
                    let new_path = new_dir.join(old_path.file_name().unwrap());
                    match std::fs::rename(&old_path, &new_path) {
                        Ok(()) => {
                            app.update_buffer_paths(&old_path, &new_path);
                            app.refresh_explorer();
                            app.show_notification(
                                format!("Moved to {}", new_path.display()),
                                crate::app::NotificationType::Info,
                            );
                        }
                        Err(err) => {
                            app.show_notification(
                                format!("Error moving file: {}", err),
                                crate::app::NotificationType::Error,
                            );
                        }
                    }
                }
                app.is_fuzzy = false;
            }
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
                crate::app::FuzzyMode::EditScript => app.fuzzy_results.len(),
                crate::app::FuzzyMode::DeleteScript => app.fuzzy_results.len(),
                crate::app::FuzzyMode::DocSelect => app.fuzzy_results.len(),
                crate::app::FuzzyMode::NewFolder => 0,
                crate::app::FuzzyMode::ScriptMenu => app.fuzzy_results.len(),
                crate::app::FuzzyMode::ScriptInput => 0,
            };
            if max > 0 && app.fuzzy_idx < max - 1 {
                app.fuzzy_idx += 1;
                if app.fuzzy_mode == crate::app::FuzzyMode::Themes {
                    if let Some(theme) = app.fuzzy_themes.get(app.fuzzy_idx) {
                        app.apply_theme(theme.clone());
                    }
                }
                if app.fuzzy_idx + 5 >= max && max >= app.fuzzy_limit {
                    app.load_more_fuzzy();
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
                            app.update_fuzzy(true);
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
                                app.refresh_explorer();
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
                            app.refresh_explorer();
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
                            app.update_fuzzy(true);
                        }
                    } else if path.is_dir() {
                        app.move_dir = Some(path);
                        app.update_fuzzy(true);
                    }
                }
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::RunScript {
                if let Some(script_path) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    match std::fs::read_to_string(&script_path) {
                        Ok(script) => {
                            let (ctx, cur_path) = if let Some(cur_buf) = app.buffers.get(app.current_buffer_idx) {
                                (crate::lua::LuaContext {
                                    current_file: cur_buf
                                        .path
                                        .as_ref()
                                        .map(|p| p.to_string_lossy().to_string())
                                        .unwrap_or_default(),
                                    current_content: cur_buf.content.to_string(),
                                    current_selection: cur_buf.get_selected_text().unwrap_or_default(),
                                    current_dir: app.explorer.root.clone(),
                                    is_live_script: false,
                                }, &cur_buf.path)
                            } else {
                                (crate::lua::LuaContext {
                                    current_file: String::new(),
                                    current_content: String::new(),
                                    current_selection: String::new(),
                                    current_dir: app.explorer.root.clone(),
                                    is_live_script: false,
                                }, &None)
                            };
                            app.start_script(script, ctx, cur_path.clone());
                            app.is_fuzzy = false;
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
                            app.refresh_explorer();
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
            } else if app.fuzzy_mode == crate::app::FuzzyMode::ScriptInput {
                let response = app.fuzzy_query.clone();
                if let Some(tx) = &app.script_response_tx {
                    let _ = tx.send(crate::lua::ScriptResponse::Prompt(response));
                }
                app.is_fuzzy = false;
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::ScriptMenu {
                let response = app.fuzzy_results.get(app.fuzzy_idx).map(|p| p.to_string_lossy().to_string());
                if let Some(tx) = &app.script_response_tx {
                    let _ = tx.send(crate::lua::ScriptResponse::Menu(response));
                }
                app.is_fuzzy = false;
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
                        app.refresh_explorer();
                    }
                }
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
                        app.refresh_explorer();
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
            app.update_fuzzy(true);
        }
        KeyCode::Backspace => {
            app.fuzzy_query.pop();
            app.update_fuzzy(true);
        }
        _ => {}
    }
}

fn handle_explorer_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            app.explorer.previous();
            let height = app.explorer_area.height.saturating_sub(2) as usize;
            if app.explorer.selected_idx < app.explorer.scroll_offset {
                app.explorer.scroll_offset = app.explorer.selected_idx;
            }
            if app.explorer.selected_idx == app.explorer.items.len().saturating_sub(1) { // Wrapped to bottom
                app.explorer.scroll_offset = app.explorer.selected_idx.saturating_sub(height).saturating_add(1);
            }
        }
        KeyCode::Down => {
            app.explorer.next();
            let height = app.explorer_area.height.saturating_sub(2) as usize;
            if app.explorer.selected_idx >= app.explorer.scroll_offset + height {
                app.explorer.scroll_offset = app.explorer.selected_idx.saturating_sub(height).saturating_add(1);
            }
            if app.explorer.selected_idx == 0 { // Wrapped
                app.explorer.scroll_offset = 0;
            }
        }
        KeyCode::Enter => {
            if let Some(item) = app.explorer.get_selected() {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if item.is_dir {
                        app.set_explorer_root(item.path.clone());
                    }
                } else if item.is_dir {
                    app.explorer.toggle_expand();
                    app.refresh_explorer();
                } else {
                    app.open_file(item.path.clone());
                }
            }
        }
        KeyCode::Backspace => {
            app.explorer.go_up_root();
            app.refresh_explorer();
        }
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
    let current_idx = app.current_buffer_idx;
    if app.buffers.get(current_idx).is_none() {
        return;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Right, m)
            if !app.buffers[current_idx].autocomplete_options.is_empty() && m == KeyModifiers::SHIFT =>
        {
            app.buffers[current_idx].accept_autocomplete();
            return;
        }
        (KeyCode::Esc, _) if app.buffers[current_idx].show_autocomplete_list => {
            app.buffers[current_idx].show_autocomplete_list = false;
        }
        (KeyCode::Up, m) if m == KeyModifiers::CONTROL => {}
        (KeyCode::Down, m) if m == KeyModifiers::CONTROL => {}
        (KeyCode::Up, m) => {
            let buffer = &mut app.buffers[current_idx];
            if m.contains(KeyModifiers::SHIFT) && buffer.selection_start.is_none() {
                buffer.selection_start = Some((buffer.cursor_row, buffer.cursor_col));
            } else if !m.contains(KeyModifiers::SHIFT) {
                buffer.selection_start = None;
            }
            buffer.move_cursor(-1, 0, 80);
        }
        (KeyCode::Down, m) => {
            let buffer = &mut app.buffers[current_idx];
            if m.contains(KeyModifiers::SHIFT) && buffer.selection_start.is_none() {
                buffer.selection_start = Some((buffer.cursor_row, buffer.cursor_col));
            } else if !m.contains(KeyModifiers::SHIFT) {
                buffer.selection_start = None;
            }
            buffer.move_cursor(1, 0, 80);
        }
        (KeyCode::Left, m) => {
            let buffer = &mut app.buffers[current_idx];
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
            let buffer = &mut app.buffers[current_idx];
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
        (KeyCode::Home, _) => app.buffers[current_idx].move_to_line_start(),
        (KeyCode::End, _) => app.buffers[current_idx].move_to_line_end(),
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT)
            if !app.buffers[current_idx].is_read_only =>
        {
            let buffer = &mut app.buffers[current_idx];
            if buffer.selection_start.is_some() {
                buffer.delete_selection();
            }
            buffer.insert_char(c);
            if app.config.autocomplete_enabled {
                buffer.update_autocomplete();
            }
        }
        (KeyCode::Enter, _) if !app.buffers[current_idx].is_read_only => {
            let buffer = &mut app.buffers[current_idx];
            if buffer.selection_start.is_some() {
                buffer.delete_selection();
            }
            buffer.insert_char('\n');
        }
        (KeyCode::Backspace, _) if !app.buffers[current_idx].is_read_only => {
            let buffer = &mut app.buffers[current_idx];
            if buffer.selection_start.is_some() {
                buffer.delete_selection();
            } else {
                buffer.delete_backspace();
            }
            if app.config.autocomplete_enabled {
                buffer.update_autocomplete();
            }
        }
        _ if app.config.matches(key, "save") && !app.buffers[current_idx].is_read_only => {
            app.save_current_buffer();
        }
        _ if app.config.matches(key, "undo") && !app.buffers[current_idx].is_read_only => app.buffers[current_idx].undo(),
        _ if app.config.matches(key, "redo") && !app.buffers[current_idx].is_read_only => app.buffers[current_idx].redo(),
        _ if app.config.matches(key, "copy") => {
            app.buffers[current_idx].copy();
        }
        _ if app.config.matches(key, "paste") && !app.buffers[current_idx].is_read_only => {
            app.buffers[current_idx].paste();
        }
        _ if app.config.matches(key, "cut") && !app.buffers[current_idx].is_read_only => {
            app.buffers[current_idx].cut();
        }
        (KeyCode::Tab, KeyModifiers::NONE) if !app.buffers[current_idx].is_read_only => {
            let buffer = &mut app.buffers[current_idx];
            for _ in 0..4 {
                buffer.insert_char(' ');
            }
        }
        _ => {}
    }

    if let Some(buffer) = app.buffers.get_mut(current_idx) {
        let height = app.editor_area.height as usize;
        if buffer.cursor_row < buffer.scroll_row {
            buffer.scroll_row = buffer.cursor_row;
        } else if buffer.cursor_row >= buffer.scroll_row + height {
            buffer.scroll_row = buffer.cursor_row.saturating_sub(height).saturating_add(1);
        }
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
        "New Lua Script" => {
            let home_dir = std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let scripts_dir = home_dir.join(".config/nedit/scripts");
            let _ = std::fs::create_dir_all(&scripts_dir);
            let name = format!("script_{}.lua", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let path = scripts_dir.join(name);
            let _ = std::fs::write(&path, "-- New Lua Script\n");
            app.open_file(path);
        }
        "Run Lua Script" => {
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
        "Undo Last Script" => app.undo_last_script(),
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

    match crate::lua::run_script_no_interactive(&script, ctx, &target_path) {
        Ok(actions) => {
            if actions.is_empty() {
                app.show_notification(
                    "Script executed successfully".to_string(),
                    crate::app::NotificationType::Info,
                );
                return;
            }
            app.apply_lua_actions(actions);
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
