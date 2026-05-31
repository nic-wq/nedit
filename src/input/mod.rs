mod templates;

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};

use crate::app::{App, Focus};

#[allow(unused_imports)]
pub use templates::LUA_TEMPLATE;

pub fn handle_events(app: &mut App) -> anyhow::Result<()> {
    // We use a short poll duration (16ms ~ 60fps) to keep the UI responsive 
    // without consuming excessive CPU when idle.
    if !event::poll(std::time::Duration::from_millis(16))? {
        return Ok(());
    }

    let mut pending_mouse_drag = None;
    let mut event_count = 0;

    loop {
        let event = event::read()?;
        handle_event(app, event, &mut pending_mouse_drag);

        event_count += 1;
        if event_count >= 4096 || !event::poll(std::time::Duration::ZERO)? {
            break;
        }
    }

    flush_pending_mouse_drag(app, &mut pending_mouse_drag);
    Ok(())
}

fn handle_event(app: &mut App, event: Event, pending_mouse_drag: &mut Option<MouseEvent>) {
    match event {
        Event::Key(key) if key.kind != KeyEventKind::Release => {
            flush_pending_mouse_drag(app, pending_mouse_drag);
            handle_key_event(app, key);
        }
        Event::Key(_) => {}
        Event::Mouse(mouse) if is_editor_left_drag(app, mouse) => {
            *pending_mouse_drag = Some(mouse);
        }
        Event::Mouse(mouse) => {
            flush_pending_mouse_drag(app, pending_mouse_drag);
            handle_mouse_event(app, mouse);
        }
        _ => {}
    }
}

fn flush_pending_mouse_drag(app: &mut App, pending_mouse_drag: &mut Option<MouseEvent>) {
    if let Some(mouse) = pending_mouse_drag.take() {
        handle_mouse_event(app, mouse);
    }
}

fn is_editor_left_drag(app: &App, mouse: MouseEvent) -> bool {
    matches!(mouse.kind, MouseEventKind::Drag(button) if button == event::MouseButton::Left)
        && app
            .editor_area
            .contains(ratatui::layout::Position::new(mouse.column, mouse.row))
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
            if app
                .editor_area
                .contains(ratatui::layout::Position::new(mouse.column, mouse.row))
            {
                app.focus = Focus::Editor;
                let rel_col = mouse.column.saturating_sub(app.editor_area.x) as usize;
                let rel_row = mouse.row.saturating_sub(app.editor_area.y) as usize;
                if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                    let target_row = buffer.scroll_row + rel_row;
                    let target_col =
                        buffer.scroll_col + rel_col.saturating_sub(buffer.line_number_width());
                    let row = target_row.min(buffer.content.len_lines().saturating_sub(1));
                    buffer.place_cursor(row, target_col);
                    buffer.selection_start = None;

                    let is_double_click = app.last_click_pos == (mouse.column, mouse.row)
                        && app.last_click_time.elapsed().as_millis() < 500;

                    app.last_click_pos = (mouse.column, mouse.row);
                    app.last_click_time = std::time::Instant::now();

                    if is_double_click {
                        buffer.select_word();
                    }
                }
            } else if app
                .explorer_area
                .contains(ratatui::layout::Position::new(mouse.column, mouse.row))
            {
                app.focus = Focus::Explorer;
                let rel_row = mouse.row.saturating_sub(app.explorer_area.y) as usize;
                let target_idx = app.explorer.scroll_offset + rel_row;
                if target_idx < app.explorer.items.len() {
                    app.explorer.selected_idx = target_idx;
                }
            }
        }
        MouseEventKind::Drag(button) if button == event::MouseButton::Left => {
            if app
                .editor_area
                .contains(ratatui::layout::Position::new(mouse.column, mouse.row))
            {
                let rel_col = mouse.column.saturating_sub(app.editor_area.x) as usize;
                let rel_row = mouse.row.saturating_sub(app.editor_area.y) as usize;
                if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                    if buffer.selection_start.is_none() {
                        buffer.selection_start = Some((buffer.cursor_row, buffer.cursor_col));
                    }
                    let target_row = buffer.scroll_row + rel_row;
                    let target_col =
                        buffer.scroll_col + rel_col.saturating_sub(buffer.line_number_width());
                    let row = target_row.min(buffer.content.len_lines().saturating_sub(1));
                    buffer.place_cursor(row, target_col);
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
        let modified_idx = app.buffers.iter().position(|b| b.modified);
        if let Some(idx) = modified_idx {
            app.pending_action = Some(crate::app::types::PendingAction::Quit);
            app.pending_buffer_idx = Some(idx);
            app.toggle_fuzzy(crate::app::FuzzyMode::UnsavedChanges);
        } else {
            app.should_quit = true;
        }
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

    if (key.code == KeyCode::Backspace && key.modifiers.contains(KeyModifiers::CONTROL))
        || (key.code == KeyCode::Char('h') && key.modifiers.contains(KeyModifiers::CONTROL))
    {
        if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
            if !buffer.is_read_only {
                buffer.delete_word();
                return;
            }
        }
    }
    if app.config.matches(key, "toggle_focus") {
        app.focus = match app.focus {
            Focus::Explorer => Focus::Editor,
            Focus::Editor => Focus::Explorer,
        };
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

    if app.config.matches(key, "set_as_root") {
        let path = app
            .explorer
            .get_selected()
            .filter(|i| i.is_dir)
            .map(|i| i.path.clone());
        if let Some(path) = path {
            app.set_explorer_root(path);
            return;
        }
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

fn handle_unsaved_changes_completion(app: &mut App) {
    let action = app.pending_action.take();
    let buffer_idx = app.pending_buffer_idx.take();

    match action {
        Some(crate::app::types::PendingAction::CloseTab) => {
            if let Some(idx) = buffer_idx {
                app.force_close_buffer(idx);
            }
            app.is_fuzzy = false;
        }
        Some(crate::app::types::PendingAction::Quit) => {
            let next_modified = app.buffers.iter().position(|b| b.modified);
            if let Some(idx) = next_modified {
                app.pending_action = Some(crate::app::types::PendingAction::Quit);
                app.pending_buffer_idx = Some(idx);
                // Stay in UnsavedChanges mode for the next buffer
            } else {
                app.should_quit = true;
                app.is_fuzzy = false;
            }
        }
        None => {
            app.is_fuzzy = false;
        }
    }
}

fn handle_fuzzy_input(app: &mut App, key: KeyEvent) {
    if matches!(
        app.fuzzy_mode,
        crate::app::FuzzyMode::Rename
            | crate::app::FuzzyMode::SaveAs
            | crate::app::FuzzyMode::NewFolder
            | crate::app::FuzzyMode::UnsavedChanges
    ) {
        match key.code {
            KeyCode::Esc => {
                app.is_fuzzy = false;
                app.fuzzy_query.clear();
                app.pending_path = None;
                app.move_dir = None;
                app.pending_action = None;
                app.pending_buffer_idx = None;
            }
            KeyCode::Enter => {}
            KeyCode::Backspace => {
                if !app.fuzzy_query.is_empty() {
                    app.fuzzy_query.pop();
                }
            }
            KeyCode::Char('s') | KeyCode::Char('S') if app.fuzzy_mode == crate::app::FuzzyMode::UnsavedChanges => {
                if let Some(idx) = app.pending_buffer_idx {
                    if idx < app.buffers.len() {
                        let has_path = app.buffers[idx].path.is_some();
                        if !has_path {
                            // If it has no path, we need to ask for a path first
                            app.current_buffer_idx = idx;
                            app.fuzzy_mode = crate::app::FuzzyMode::SaveAs;
                            app.fuzzy_query.clear();
                            return;
                        } else {
                            let _ = app.buffers[idx].save();
                        }
                    }
                }
                handle_unsaved_changes_completion(app);
            }
            KeyCode::Char('d') | KeyCode::Char('D') if app.fuzzy_mode == crate::app::FuzzyMode::UnsavedChanges => {
                if let Some(idx) = app.pending_buffer_idx {
                    if idx < app.buffers.len() {
                        app.buffers[idx].modified = false;
                    }
                }
                handle_unsaved_changes_completion(app);
            }
            KeyCode::Char(c) => {
                if app.fuzzy_mode != crate::app::FuzzyMode::UnsavedChanges {
                    app.fuzzy_query.push(c);
                }
            }
            _ => {}
        }
        if key.code != KeyCode::Enter {
            return;
        }
    }

    match key.code {
        KeyCode::Esc => {
            if app.fuzzy_mode == crate::app::FuzzyMode::Themes {
                app.current_theme = app.original_theme.clone();
            }
            app.pending_path = None;
            app.move_dir = None;
            app.clear_notification();
            app.is_fuzzy = false;
        }
        KeyCode::Tab => {
            if app.fuzzy_mode == crate::app::FuzzyMode::Move {
                if let (Some(old_path), Some(new_dir)) =
                    (app.pending_path.take(), app.move_dir.take())
                {
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
                crate::app::FuzzyMode::Content => {
                    if app.fuzzy_results.is_empty() {
                        app.fuzzy_global_results.len()
                    } else {
                        app.fuzzy_results.len()
                    }
                }
                crate::app::FuzzyMode::Files => app.fuzzy_results.len(),
                crate::app::FuzzyMode::Themes => app.fuzzy_themes.len(),
                crate::app::FuzzyMode::SaveAs => 0,
                crate::app::FuzzyMode::Rename => 0,
                crate::app::FuzzyMode::DeleteConfirm => 0,
                crate::app::FuzzyMode::FileOptions => app.fuzzy_results.len(),
                crate::app::FuzzyMode::CommandPalette => app.fuzzy_results.len(),
                crate::app::FuzzyMode::Move => app.fuzzy_results.len(),
                crate::app::FuzzyMode::RunScript => app.fuzzy_results.len(),
                crate::app::FuzzyMode::EditScript => app.fuzzy_results.len(),
                crate::app::FuzzyMode::DeleteScript => app.fuzzy_results.len(),
                crate::app::FuzzyMode::DocSelect => app.fuzzy_results.len(),
                crate::app::FuzzyMode::NewFolder => 0,
                crate::app::FuzzyMode::UnsavedChanges => 0,
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
                        "Set as Root" => {
                            app.set_explorer_root(item.path.clone());
                            app.is_fuzzy = false;
                        }
                        _ => app.is_fuzzy = false,
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
                            let (ctx, cur_path) =
                                if let Some(cur_buf) = app.buffers.get(app.current_buffer_idx) {
                                    (
                                        crate::lua::LuaContext {
                                            current_file: cur_buf
                                                .path
                                                .as_ref()
                                                .map(|p| p.to_string_lossy().to_string())
                                                .unwrap_or_default(),
                                            current_content: cur_buf.content.to_string(),
                                            current_selection: cur_buf
                                                .get_selected_text()
                                                .unwrap_or_default(),
                                            current_dir: app.explorer.root.clone(),
                                            is_live_script: false,
                                        },
                                        &cur_buf.path,
                                    )
                                } else {
                                    (
                                        crate::lua::LuaContext {
                                            current_file: String::new(),
                                            current_content: String::new(),
                                            current_selection: String::new(),
                                            current_dir: app.explorer.root.clone(),
                                            is_live_script: false,
                                        },
                                        &None,
                                    )
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
                }
                if app.pending_action.is_some() {
                    handle_unsaved_changes_completion(app);
                } else {
                    app.is_fuzzy = false;
                }
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Local {
                if let Some((line_idx, _)) = app.fuzzy_lines.get(app.fuzzy_idx) {
                    if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                        buffer.cursor_row = *line_idx;
                        buffer.move_to_line_start();
                        let height = app.editor_area.height as usize;
                        if buffer.cursor_row < buffer.scroll_row {
                            buffer.scroll_row = buffer.cursor_row;
                        } else if buffer.cursor_row >= buffer.scroll_row + height {
                            buffer.scroll_row = buffer.cursor_row.saturating_sub(height).saturating_add(1);
                        }
                    }
                }
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Content {
                if let Some(path) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    let prefer_home = app
                        .fuzzy_query
                        .trim()
                        .strip_prefix('@')
                        .map(|query| query.starts_with('~'))
                        .unwrap_or(false);
                    let path_text = app.format_search_dir_for_query(&path, prefer_home);
                    app.fuzzy_query = format!("@{}/", path_text.trim_end_matches('/'));
                    app.update_fuzzy(true);
                    return;
                }
                if let Some((path, line_idx, _)) = app.fuzzy_global_results.get(app.fuzzy_idx) {
                    let path = path.clone();
                    let line_idx = *line_idx;
                    app.open_file(path);
                    if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                        buffer.cursor_row = line_idx;
                        buffer.move_to_line_start();
                        let height = app.editor_area.height as usize;
                        if buffer.cursor_row < buffer.scroll_row {
                            buffer.scroll_row = buffer.cursor_row;
                        } else if buffer.cursor_row >= buffer.scroll_row + height {
                            buffer.scroll_row = buffer.cursor_row.saturating_sub(height).saturating_add(1);
                        }
                    }
                }
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Files {
                if let Some(path) = app.fuzzy_results.get(app.fuzzy_idx).cloned() {
                    let is_scoped_dir_pick = path.is_dir()
                        && app
                            .fuzzy_query
                            .trim()
                            .strip_prefix('@')
                            .map(|query| !query.chars().any(char::is_whitespace))
                            .unwrap_or(false);
                    if is_scoped_dir_pick {
                        let prefer_home = app
                            .fuzzy_query
                            .trim()
                            .strip_prefix('@')
                            .map(|query| query.starts_with('~'))
                            .unwrap_or(false);
                        let path_text = app.format_search_dir_for_query(&path, prefer_home);
                        app.fuzzy_query = format!("@{}/", path_text.trim_end_matches('/'));
                        app.update_fuzzy(true);
                        return;
                    }
                    app.open_file(path);
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
            if app.explorer.selected_idx == app.explorer.items.len().saturating_sub(1) {
                // Wrapped to bottom
                app.explorer.scroll_offset = app
                    .explorer
                    .selected_idx
                    .saturating_sub(height)
                    .saturating_add(1);
            }
        }
        KeyCode::Down => {
            app.explorer.next();
            let height = app.explorer_area.height.saturating_sub(2) as usize;
            if app.explorer.selected_idx >= app.explorer.scroll_offset + height {
                app.explorer.scroll_offset = app
                    .explorer
                    .selected_idx
                    .saturating_sub(height)
                    .saturating_add(1);
            }
            if app.explorer.selected_idx == 0 {
                // Wrapped
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
        KeyCode::Char(c) if is_explorer_file_options_shortcut(c, key.modifiers) => {
            let is_dir = app.explorer.get_selected().map(|i| i.is_dir).unwrap_or(false);
            app.toggle_fuzzy(crate::app::FuzzyMode::FileOptions);
            let mut options = vec![
                std::path::PathBuf::from("Rename"),
                std::path::PathBuf::from("Move"),
                std::path::PathBuf::from("Delete"),
            ];
            if is_dir {
                options.push(std::path::PathBuf::from("Set as Root"));
            }
            app.fuzzy_results = options;
            app.fuzzy_idx = 0;
        }
        _ => {}
    }
}

fn is_explorer_file_options_shortcut(c: char, modifiers: KeyModifiers) -> bool {
    c == 'O' || (c == 'o' && modifiers.contains(KeyModifiers::SHIFT))
}

fn handle_editor_input(app: &mut App, key: KeyEvent) {
    let current_idx = app.current_buffer_idx;
    if app.buffers.get(current_idx).is_none() {
        return;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Right, m)
            if !app.buffers[current_idx].autocomplete_options.is_empty()
                && m == KeyModifiers::SHIFT =>
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
        _ if app.config.matches(key, "undo") && !app.buffers[current_idx].is_read_only => {
            app.buffers[current_idx].undo()
        }
        _ if app.config.matches(key, "redo") && !app.buffers[current_idx].is_read_only => {
            app.buffers[current_idx].redo()
        }
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
        "New Lua Script" => {
            let home_dir = std::env::var("HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let scripts_dir = home_dir.join(".config/nedit/scripts");
            let _ = std::fs::create_dir_all(&scripts_dir);
            let name = format!(
                "script_{}.lua",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            );
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
