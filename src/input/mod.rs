use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind,
};

use crate::app::{App, Focus};

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
        Event::Paste(text) => {
            flush_pending_mouse_drag(app, pending_mouse_drag);
            handle_paste(app, text);
        }
        Event::Mouse(mouse) if is_editor_left_drag(app, mouse) => {
            *pending_mouse_drag = Some(mouse);
        }
        Event::Mouse(mouse) => {
            flush_pending_mouse_drag(app, pending_mouse_drag);
            handle_mouse_event(app, mouse);
        }
        _ => {}
    }
    app.needs_redraw = true;
}

fn handle_paste(app: &mut App, text: String) {
    if app.is_fuzzy {
        if app.fuzzy_has_editable_input() {
            // Single-line field: collapse line breaks and insert at the cursor,
            // replacing any selection like a regular edit.
            let sanitized = text.replace("\r\n", " ").replace(['\r', '\n'], " ");
            app.fuzzy_clamp();
            app.fuzzy_insert_str(&sanitized);
            if app.fuzzy_is_live_mode() {
                app.schedule_fuzzy_update(true);
            }
        }
        return;
    }
    if app.is_welcome || app.buffers.is_empty() {
        return;
    }
    if app.focus != Focus::Editor {
        return;
    }
    if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
        if buffer.is_read_only {
            return;
        }
        // Use raw insertion so bulk paste preserves exact whitespace and is one undo.
        buffer.insert_text(&text);
        if app.config.autocomplete_enabled {
            buffer.update_autocomplete();
        }
        // Keep cursor visible after paste.
        let height = app.editor_area.height as usize;
        if buffer.cursor_row < buffer.scroll_row {
            buffer.scroll_row = buffer.cursor_row;
        } else if buffer.cursor_row >= buffer.scroll_row + height && height > 0 {
            buffer.scroll_row = buffer.cursor_row.saturating_sub(height).saturating_add(1);
        }
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
        MouseEventKind::Down(event::MouseButton::Left) => {
            // Toasts float above everything: their close button wins the click.
            if app.dismiss_toast_at(mouse.column, mouse.row) {
                return;
            }
            if app
                .editor_area
                .contains(ratatui::layout::Position::new(mouse.column, mouse.row))
            {
                app.focus = Focus::Editor;
                // Limpar preview se clicou no editor
                if let Some(idx) = app.preview_buffer_idx {
                    app.clear_preview(idx);
                }
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
                app.update_preview_from_explorer_selection();
            }
        }
        MouseEventKind::Drag(event::MouseButton::Left)
            if app
                .editor_area
                .contains(ratatui::layout::Position::new(mouse.column, mouse.row)) =>
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
        // Same create flow from editor or explorer — path is relative to the
        // explorer selection (or root), so focus no longer changes the action.
        app.toggle_fuzzy(crate::app::FuzzyMode::Create);
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
        // Limpar preview ao sair do explorer para o editor
        if app.focus == Focus::Explorer {
            if let Some(idx) = app.preview_buffer_idx.take() {
                app.clear_preview(idx);
            }
        }
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

    // Tab cycling is unified on Ctrl+Alt+←/→ (and Ctrl+Tab): it works from any
    // buffer, including the live script pane, which is pinned as the last
    // tab — so plain cycling always reaches the script and back.
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
        (KeyCode::Char(c), KeyModifiers::ALT) if c.is_ascii_digit() => {
            let idx = c.to_digit(10).unwrap() as usize;
            if idx > 0 {
                app.switch_tab(idx - 1);
            }
            return;
        }
        _ => {}
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

/// Text editing keys for fuzzy modal inputs (search fields, Rename, SaveAs,
/// Create). Returns true when the key was consumed.
///
/// Up/Down/Enter/Esc/Tab are intentionally left to the modal logic (result
/// navigation, confirm, ...). Only runs for modes with an editable field —
/// `UnsavedChanges`, `ExternalChange` and `DeleteConfirm` react to shortcut
/// keys instead, so typing must not touch the query there.
fn handle_fuzzy_text_keys(app: &mut App, key: KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    if key.modifiers.contains(KeyModifiers::ALT) {
        return false;
    }
    app.fuzzy_clamp();
    let mut mutated = false;
    match (key.code, ctrl, shift) {
        (KeyCode::Left, false, extend) => app.fuzzy_move_left(extend),
        (KeyCode::Right, false, extend) => app.fuzzy_move_right(extend),
        (KeyCode::Left, true, extend) => app.fuzzy_move_word(-1, extend),
        (KeyCode::Right, true, extend) => app.fuzzy_move_word(1, extend),
        (KeyCode::Home, _, extend) => app.fuzzy_home(extend),
        (KeyCode::End, _, extend) => app.fuzzy_end(extend),
        (KeyCode::Delete, false, _) => {
            app.fuzzy_delete_fwd();
            mutated = true;
        }
        (KeyCode::Delete, true, _) => {
            app.fuzzy_delete_word_fwd();
            mutated = true;
        }
        (KeyCode::Backspace, false, _) => {
            app.fuzzy_backspace();
            mutated = true;
        }
        (KeyCode::Backspace, true, _) => {
            app.fuzzy_delete_word_back();
            mutated = true;
        }
        (KeyCode::Char('a') | KeyCode::Char('A'), true, _) => app.fuzzy_select_all(),
        (KeyCode::Char('e') | KeyCode::Char('E'), true, _) => app.fuzzy_end(false),
        (KeyCode::Char('b') | KeyCode::Char('B'), true, _) => app.fuzzy_move_left(false),
        (KeyCode::Char('f') | KeyCode::Char('F'), true, _) => app.fuzzy_move_right(false),
        (KeyCode::Char('u') | KeyCode::Char('U'), true, _) => {
            app.fuzzy_kill_to_start();
            mutated = true;
        }
        (KeyCode::Char('k') | KeyCode::Char('K'), true, _) => {
            app.fuzzy_kill_to_end();
            mutated = true;
        }
        (KeyCode::Char('w') | KeyCode::Char('W'), true, _) => {
            app.fuzzy_delete_word_back();
            mutated = true;
        }
        (KeyCode::Char(c), false, _) => {
            app.fuzzy_insert_str(&c.to_string());
            mutated = true;
        }
        _ => return false,
    }
    if mutated && app.fuzzy_is_live_mode() {
        // Debounce: run after 80ms pause, like the generic typing path.
        app.schedule_fuzzy_update(true);
    }
    true
}

fn handle_fuzzy_input(app: &mut App, key: KeyEvent) {
    if app.fuzzy_has_editable_input() && handle_fuzzy_text_keys(app, key) {
        return;
    }
    if matches!(
        app.fuzzy_mode,
        crate::app::FuzzyMode::Rename
            | crate::app::FuzzyMode::SaveAs
            | crate::app::FuzzyMode::Create
            | crate::app::FuzzyMode::UnsavedChanges
            | crate::app::FuzzyMode::ExternalChange
    ) {
        match key.code {
            KeyCode::Esc => {
                if app.fuzzy_mode == crate::app::FuzzyMode::ExternalChange {
                    // Keep local — update mtime to new file time to avoid re-popup.
                    if let Some(idx) = app.pending_buffer_idx {
                        if let Some(path) = app.buffers.get(idx).and_then(|b| b.path.clone()) {
                            app.record_file_mtime(&path);
                        }
                    }
                }
                app.is_fuzzy = false;
                app.clear_fuzzy_query();
                app.pending_path = None;
                app.move_dir = None;
                app.pending_action = None;
                app.pending_buffer_idx = None;
            }
            KeyCode::Enter => {}
            KeyCode::Char('s') | KeyCode::Char('S')
                if app.fuzzy_mode == crate::app::FuzzyMode::UnsavedChanges =>
            {
                if let Some(idx) = app.pending_buffer_idx {
                    if idx < app.buffers.len() {
                        let has_path = app.buffers[idx].path.is_some();
                        if !has_path {
                            // If it has no path, we need to ask for a path first
                            app.current_buffer_idx = idx;
                            app.fuzzy_mode = crate::app::FuzzyMode::SaveAs;
                            app.clear_fuzzy_query();
                            return;
                        } else {
                            let _ = app.buffers[idx].save();
                            if let Some(p) = app.buffers[idx].path.clone() {
                                app.record_file_mtime(&p);
                            }
                        }
                    }
                }
                handle_unsaved_changes_completion(app);
            }
            KeyCode::Char('d') | KeyCode::Char('D')
                if app.fuzzy_mode == crate::app::FuzzyMode::UnsavedChanges =>
            {
                if let Some(idx) = app.pending_buffer_idx {
                    if idx < app.buffers.len() {
                        app.buffers[idx].modified = false;
                    }
                }
                handle_unsaved_changes_completion(app);
            }
            KeyCode::Char('r') | KeyCode::Char('R')
                if app.fuzzy_mode == crate::app::FuzzyMode::ExternalChange =>
            {
                if let Some(idx) = app.pending_buffer_idx {
                    app.reload_buffer_from_disk(idx);
                }
                app.is_fuzzy = false;
                app.clear_fuzzy_query();
                app.pending_path = None;
                app.pending_buffer_idx = None;
            }
            KeyCode::Char('k') | KeyCode::Char('K')
                if app.fuzzy_mode == crate::app::FuzzyMode::ExternalChange =>
            {
                // Keep local version — just update mtime.
                if let Some(idx) = app.pending_buffer_idx {
                    if let Some(path) = app.buffers.get(idx).and_then(|b| b.path.clone()) {
                        app.record_file_mtime(&path);
                    }
                }
                app.is_fuzzy = false;
                app.clear_fuzzy_query();
                app.pending_path = None;
                app.pending_buffer_idx = None;
            }
            // NOTE: text input (Char/Backspace/Delete/arrows/...) for the
            // editable modes is handled by handle_fuzzy_text_keys above.
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
        KeyCode::Tab if app.fuzzy_mode == crate::app::FuzzyMode::Move => {
            if let (Some(old_path), Some(new_dir)) = (app.pending_path.take(), app.move_dir.take())
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
        KeyCode::Up if app.fuzzy_idx > 0 => {
            app.fuzzy_idx -= 1;
            if app.fuzzy_mode == crate::app::FuzzyMode::Themes {
                if let Some(theme) = app.fuzzy_themes.get(app.fuzzy_idx) {
                    app.apply_theme(theme.clone());
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
                crate::app::FuzzyMode::DocSelect => app.fuzzy_results.len(),
                crate::app::FuzzyMode::Create => 0,
                crate::app::FuzzyMode::UnsavedChanges => 0,
                crate::app::FuzzyMode::ExternalChange => 0,
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
                        "New File" => {
                            app.fuzzy_mode = crate::app::FuzzyMode::Create;
                            app.clear_fuzzy_query();
                            app.pending_path = None;
                        }
                        "New Folder" => {
                            // Prefill trailing slash so Enter creates a directory.
                            app.fuzzy_mode = crate::app::FuzzyMode::Create;
                            app.set_fuzzy_query("/".to_string());
                            app.pending_path = None;
                        }
                        "Rename" => {
                            app.fuzzy_mode = crate::app::FuzzyMode::Rename;
                            app.set_fuzzy_query(item.name.clone());
                        }
                        "Move" => {
                            app.fuzzy_mode = crate::app::FuzzyMode::Move;
                            app.move_dir = item.path.parent().map(|p| p.to_path_buf());
                            app.clear_fuzzy_query();
                            app.update_fuzzy(true);
                        }
                        "Delete" => {
                            app.fuzzy_mode = crate::app::FuzzyMode::DeleteConfirm;
                            app.clear_fuzzy_query();
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
                    if path == *".." {
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
            } else if app.fuzzy_mode == crate::app::FuzzyMode::Create {
                if !app.fuzzy_query.trim().is_empty() {
                    app.create_path_from_input(&app.fuzzy_query.clone());
                }
                app.is_fuzzy = false;
                return;
            } else if app.fuzzy_mode == crate::app::FuzzyMode::SaveAs {
                if !app.fuzzy_query.is_empty() {
                    let filename = app.fuzzy_query.trim().to_string();
                    // Live script panes save like any other buffer: wherever
                    // the user points, with no forced scripts directory.
                    let path = app.resolve_input_path(&filename);
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if let Some(buffer) = app.buffers.get_mut(app.current_buffer_idx) {
                        buffer.path = Some(path.clone());
                        if let Err(err) = buffer.save() {
                            app.show_notification(
                                format!("Could not save file: {}", err),
                                crate::app::NotificationType::Error,
                            );
                            return;
                        }
                        app.record_file_mtime(&path);
                    }
                    app.refresh_explorer();
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
                            buffer.scroll_row =
                                buffer.cursor_row.saturating_sub(height).saturating_add(1);
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
                    app.set_fuzzy_query(format!("@{}/", path_text.trim_end_matches('/')));
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
                            buffer.scroll_row =
                                buffer.cursor_row.saturating_sub(height).saturating_add(1);
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
                        app.set_fuzzy_query(format!("@{}/", path_text.trim_end_matches('/')));
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
        // Reachable for non-editable modes and for key combos with
        // modifiers that handle_fuzzy_text_keys leaves alone (e.g. Alt+char).
        // Keep it cursor-aware so the state never diverges.
        KeyCode::Char(c) => {
            app.fuzzy_clamp();
            app.fuzzy_insert_str(&c.to_string());
            app.schedule_fuzzy_update(true); // debounce: run after 80ms pause
        }
        KeyCode::Backspace => {
            app.fuzzy_clamp();
            app.fuzzy_backspace();
            app.schedule_fuzzy_update(true); // debounce: run after 80ms pause
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
            app.update_preview_from_explorer_selection();
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
            app.update_preview_from_explorer_selection();
        }
        KeyCode::Enter => {
            if let Some(item) = app.explorer.get_selected() {
                let path = item.path.clone();
                let is_dir = item.is_dir;
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    if is_dir {
                        app.set_explorer_root(path);
                    }
                } else if is_dir {
                    app.explorer.toggle_expand();
                    app.refresh_explorer();
                } else {
                    app.open_file(path);
                }
            }
        }
        KeyCode::Backspace => {
            app.explorer.go_up_root();
            app.refresh_explorer();
            // Limpar preview ao navegar para diretório pai
            if let Some(idx) = app.preview_buffer_idx {
                app.clear_preview(idx);
            }
        }
        KeyCode::Char(c) if is_explorer_file_options_shortcut(c, key.modifiers) => {
            let is_dir = app
                .explorer
                .get_selected()
                .map(|i| i.is_dir)
                .unwrap_or(false);
            app.toggle_fuzzy(crate::app::FuzzyMode::FileOptions);
            let mut options = vec![
                std::path::PathBuf::from("New File"),
                std::path::PathBuf::from("New Folder"),
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
            // Clear selection without a history entry so replace+insert is one undo.
            buffer.clear_selection_content();
            buffer.insert_char(c);
            if app.config.autocomplete_enabled {
                buffer.update_autocomplete();
            }
        }
        (KeyCode::Enter, _) if !app.buffers[current_idx].is_read_only => {
            let buf = &mut app.buffers[current_idx];
            if !buf.autocomplete_options.is_empty() {
                buf.accept_autocomplete();
                return;
            }
            buf.clear_selection_content();
            buf.insert_char('\n');
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
            if !buffer.autocomplete_options.is_empty() {
                buffer.accept_autocomplete();
                return;
            }
            buffer.insert_text("    ");
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
        "New File" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::Create);
            return true;
        }
        "New Folder" => {
            app.toggle_fuzzy(crate::app::FuzzyMode::Create);
            app.set_fuzzy_query("/".to_string());
            return true;
        }
        "New Untitled" => app.new_file(),
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
    };

    match crate::lua::run_script(&script, ctx) {
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

#[cfg(test)]
mod tests {
    use super::{handle_fuzzy_input, handle_paste};
    use crate::app::{App, FuzzyMode};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_mods(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            handle_fuzzy_input(app, key(KeyCode::Char(c)));
        }
    }

    #[test]
    fn fuzzy_typing_arrows_and_ctrl_a_replace() {
        let mut app = App::new(&[]);
        app.toggle_fuzzy(FuzzyMode::Files);
        type_text(&mut app, "hello");
        assert_eq!(app.fuzzy_query, "hello");
        assert_eq!(app.fuzzy_cursor, 5);

        // Move left twice and type mid-query.
        handle_fuzzy_input(&mut app, key(KeyCode::Left));
        handle_fuzzy_input(&mut app, key(KeyCode::Left));
        handle_fuzzy_input(&mut app, key(KeyCode::Char('X')));
        assert_eq!(app.fuzzy_query, "helXlo");
        assert_eq!(app.fuzzy_cursor, 4);

        // Ctrl+A selects everything; typing replaces it.
        handle_fuzzy_input(
            &mut app,
            key_mods(KeyCode::Char('a'), KeyModifiers::CONTROL),
        );
        assert_eq!(app.fuzzy_selection_range(), Some((0, 6)));
        type_text(&mut app, "hi");
        assert_eq!(app.fuzzy_query, "hi");
        assert_eq!(app.fuzzy_selection_range(), None);

        // Ctrl+A + Backspace clears the field.
        handle_fuzzy_input(
            &mut app,
            key_mods(KeyCode::Char('a'), KeyModifiers::CONTROL),
        );
        handle_fuzzy_input(&mut app, key(KeyCode::Backspace));
        assert_eq!(app.fuzzy_query, "");
        assert_eq!(app.fuzzy_cursor, 0);
    }

    #[test]
    fn fuzzy_shift_selection_and_delete_key() {
        let mut app = App::new(&[]);
        app.toggle_fuzzy(FuzzyMode::CommandPalette);
        type_text(&mut app, "save");
        handle_fuzzy_input(&mut app, key(KeyCode::Home));
        assert_eq!(app.fuzzy_cursor, 0);
        handle_fuzzy_input(
            &mut app,
            key_mods(KeyCode::Right, KeyModifiers::SHIFT),
        );
        handle_fuzzy_input(
            &mut app,
            key_mods(KeyCode::Right, KeyModifiers::SHIFT),
        );
        assert_eq!(app.fuzzy_selection_range(), Some((0, 2)));
        handle_fuzzy_input(&mut app, key(KeyCode::Delete));
        assert_eq!(app.fuzzy_query, "ve");
        assert_eq!(app.fuzzy_cursor, 0);

        // Ctrl+Right jumps words; Ctrl+W deletes the previous word.
        handle_fuzzy_input(&mut app, key(KeyCode::End));
        type_text(&mut app, " new file");
        handle_fuzzy_input(
            &mut app,
            key_mods(KeyCode::Left, KeyModifiers::CONTROL),
        );
        assert_eq!(app.fuzzy_cursor, 7);
        handle_fuzzy_input(
            &mut app,
            key_mods(KeyCode::Char('w'), KeyModifiers::CONTROL),
        );
        assert_eq!(app.fuzzy_query, "ve file");
    }

    #[test]
    fn fuzzy_text_modes_share_editing_keys() {
        // Rename is a plain text field (no live list) but gets the same keys.
        let mut app = App::new(&[]);
        app.toggle_fuzzy(FuzzyMode::Rename);
        type_text(&mut app, "old.rs");
        handle_fuzzy_input(
            &mut app,
            key_mods(KeyCode::Char('a'), KeyModifiers::CONTROL),
        );
        type_text(&mut app, "new.rs");
        assert_eq!(app.fuzzy_query, "new.rs");

        // Home + Ctrl+K deletes to the end.
        handle_fuzzy_input(&mut app, key(KeyCode::Home));
        handle_fuzzy_input(
            &mut app,
            key_mods(KeyCode::Char('k'), KeyModifiers::CONTROL),
        );
        assert_eq!(app.fuzzy_query, "");
    }

    #[test]
    fn fuzzy_shortcut_modals_ignore_text_keys() {
        // UnsavedChanges reacts to S/D shortcuts; typing must not edit text.
        let mut app = App::new(&[]);
        app.toggle_fuzzy(FuzzyMode::UnsavedChanges);
        assert!(!app.fuzzy_has_editable_input());
        handle_fuzzy_input(&mut app, key(KeyCode::Left));
        handle_fuzzy_input(&mut app, key(KeyCode::Char('x')));
        assert_eq!(app.fuzzy_query, "");
    }

    #[test]
    fn ctrl_alt_arrows_cycle_tabs_with_script_last() {
        use crate::buffer::EditorBuffer;

        let mut app = App::new(&[]);
        app.buffers.push(EditorBuffer::new());
        app.open_live_script();
        let script = app.live_script_buffer_idx.unwrap();
        assert_eq!(script, app.buffers.len() - 1);

        // From the target, Ctrl+Alt+Right lands on the script pane.
        app.current_buffer_idx = 0;
        super::handle_key_event(
            &mut app,
            key_mods(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::ALT),
        );
        assert_eq!(app.current_buffer_idx, script);
        // And wraps back around, even from the script pane itself.
        super::handle_key_event(
            &mut app,
            key_mods(KeyCode::Right, KeyModifiers::CONTROL | KeyModifiers::ALT),
        );
        assert_eq!(app.current_buffer_idx, 0);
        super::handle_key_event(
            &mut app,
            key_mods(KeyCode::Left, KeyModifiers::CONTROL | KeyModifiers::ALT),
        );
        assert_eq!(app.current_buffer_idx, script);

        // Shift+Alt is gone: it must not switch tabs anymore.
        super::handle_key_event(
            &mut app,
            key_mods(KeyCode::Right, KeyModifiers::SHIFT | KeyModifiers::ALT),
        );
        assert_eq!(app.current_buffer_idx, script);
    }

    #[test]
    fn clicking_toast_close_button_dismisses_it() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
        use ratatui::layout::Rect;

        fn click(col: u16, row: u16) -> MouseEvent {
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: col,
                row,
                modifiers: KeyModifiers::NONE,
            }
        }

        let mut app = App::new(&[]);
        app.show_notification("Hi".to_string(), crate::app::NotificationType::Info);
        app.screen_area = Rect::new(0, 0, 80, 24);
        // Close button of the default bottom-right "Hi" toast.
        super::handle_mouse_event(&mut app, click(76, 19));
        assert!(app.notifications.is_empty());

        // Clicks anywhere else leave toasts alone.
        app.show_notification("Hi".to_string(), crate::app::NotificationType::Info);
        super::handle_mouse_event(&mut app, click(0, 0));
        assert_eq!(app.notifications.len(), 1);
    }

    #[test]
    fn fuzzy_paste_inserts_at_cursor() {
        let mut app = App::new(&[]);
        app.toggle_fuzzy(FuzzyMode::Files);
        type_text(&mut app, "ac");
        handle_fuzzy_input(&mut app, key(KeyCode::Left));
        handle_paste(&mut app, "b".to_string());
        assert_eq!(app.fuzzy_query, "abc");
        assert_eq!(app.fuzzy_cursor, 2);
    }
}
