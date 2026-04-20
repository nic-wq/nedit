use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
    Frame,
};
use ratatui::prelude::Stylize;
use crate::app::{App, Focus, FuzzyMode};
use syntect::easy::HighlightLines;

struct UIColors {
    bg: Color,
    fg: Color,
    sel: Color,
    accent: Color,
    surface: Color,
}

fn get_colors(app: &App) -> UIColors {
    let theme = app.theme_set.themes.get(&app.current_theme)
        .unwrap_or_else(|| &app.theme_set.themes["base16-ocean.dark"]);
    
    UIColors {
        bg: theme.settings.background.map(|c| Color::Rgb(c.r, c.g, c.b)).unwrap_or(Color::Rgb(30, 30, 46)),
        fg: theme.settings.foreground.map(|c| Color::Rgb(c.r, c.g, c.b)).unwrap_or(Color::Rgb(205, 214, 244)),
        sel: theme.settings.selection.map(|c| Color::Rgb(c.r, c.g, c.b)).unwrap_or(Color::Rgb(69, 71, 90)),
        accent: Color::Rgb(137, 180, 250),
        surface: Color::Rgb(49, 50, 68),
    }
}

pub fn render(f: &mut Frame, app: &mut App) {
    let colors = get_colors(app);
    
    // Fill background
    f.render_widget(Block::default().bg(colors.bg), f.area());

    let chunks = if app.notification.is_some() {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main area
                Constraint::Length(1), // Status bar
                Constraint::Length(1), // Notification bar
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main area
                Constraint::Length(1), // Status bar
            ])
            .split(f.area())
    };

    let explorer_width = if app.show_explorer {
        let max_len = app.explorer.items.iter()
            .map(|item| item.depth * 2 + item.name.len() + 10)
            .max()
            .unwrap_or(20);
        let percent = (max_len as f32 / f.area().width as f32 * 100.0) as u16;
        percent.clamp(20, 45)
    } else {
        0
    };

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(explorer_width),
            Constraint::Percentage(100 - explorer_width),
        ])
        .split(chunks[0]);

    if app.show_explorer {
        draw_explorer(f, app, main_chunks[0], &colors);
    }

    // Editor Area
    let editor_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Tabs
            Constraint::Min(0),    // Content
        ])
        .split(main_chunks[1]);

    draw_tab_bar(f, app, editor_chunks[0], &colors);

    if app.is_welcome {
        draw_welcome_screen(f, app, editor_chunks[1], &colors);
    } else if app.live_script_mode {
        let target_idx = app.target_buffer_idx.unwrap_or(0);
        let script_idx = app.live_script_buffer_idx.unwrap_or(0);
        
        if target_idx < app.buffers.len() && script_idx < app.buffers.len() {
            let split_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Percentage(50),
                ])
                .split(editor_chunks[1]);
            
            // Handle scroll for target buffer
            if let Some(target_buf) = app.buffers.get_mut(target_idx) {
                let height = split_chunks[0].height as usize;
                let width = split_chunks[0].width as usize;
                if target_buf.cursor_row < target_buf.scroll_row {
                    target_buf.scroll_row = target_buf.cursor_row;
                } else if target_buf.cursor_row >= target_buf.scroll_row + height {
                    target_buf.scroll_row = target_buf.cursor_row.saturating_sub(height).saturating_add(1);
                }
                target_buf.move_cursor(0, 0, width);
            }
            
            // Handle scroll for script buffer
            if let Some(script_buf) = app.buffers.get_mut(script_idx) {
                let height = split_chunks[1].height as usize;
                let width = split_chunks[1].width as usize;
                if script_buf.cursor_row < script_buf.scroll_row {
                    script_buf.scroll_row = script_buf.cursor_row;
                } else if script_buf.cursor_row >= script_buf.scroll_row + height {
                    script_buf.scroll_row = script_buf.cursor_row.saturating_sub(height).saturating_add(1);
                }
                script_buf.move_cursor(0, 0, width);
            }
                
            draw_editor(f, app, split_chunks[0], target_idx, app.current_buffer_idx == target_idx, &colors);
            draw_editor(f, app, split_chunks[1], script_idx, app.current_buffer_idx == script_idx, &colors);
        } else {
            // Fallback if indices are invalid
            if !app.buffers.is_empty() {
                draw_editor(f, app, editor_chunks[1], app.current_buffer_idx.min(app.buffers.len() - 1), true, &colors);
            }
        }
    } else if !app.buffers.is_empty() {
        let buffer = &mut app.buffers[app.current_buffer_idx];
        let height = editor_chunks[1].height as usize;
        let width = editor_chunks[1].width as usize;
        if buffer.cursor_row < buffer.scroll_row {
            buffer.scroll_row = buffer.cursor_row;
        } else if buffer.cursor_row >= buffer.scroll_row + height {
            buffer.scroll_row = buffer.cursor_row.saturating_sub(height).saturating_add(1);
        }
        buffer.move_cursor(0, 0, width);
        
        draw_editor(f, app, editor_chunks[1], app.current_buffer_idx, true, &colors);
    }
    
    draw_status_bar(f, app, chunks[1], &colors);

    if let Some((ref msg, ref ntype)) = app.notification {
        if chunks.len() > 2 {
            draw_notification(f, msg, ntype, chunks[2], &colors);
        }
    }

    if app.is_fuzzy {
        draw_fuzzy_finder(f, app, &colors);
    }
}

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    if app.buffers.is_empty() { return; }

    let mut spans = Vec::new();

    for (i, buffer) in app.buffers.iter().enumerate() {
        let is_live_script = Some(i) == app.live_script_buffer_idx;
        
        if is_live_script {
            continue;
        }
        
        let name = buffer.path.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[No Name]".to_string());
        
        let modified = if buffer.modified { "*" } else { "" };
        let is_current = i == app.current_buffer_idx;
        let style = if is_current {
            Style::default().fg(colors.accent).add_modifier(Modifier::BOLD).bg(colors.sel)
        } else {
            Style::default().fg(colors.fg)
        };

        spans.push(Span::styled(format!(" {} {} ", name, modified), style));
        spans.push(Span::raw(" "));
    }

    f.render_widget(Paragraph::new(Line::from(spans)).bg(colors.bg), area);
}

fn draw_explorer(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    let items: Vec<ListItem> = app.explorer.items.iter().enumerate().map(|(i, item)| {
        let indent = "  ".repeat(item.depth);
        let icon = if item.is_dir {
            if item.expanded { "󰉖 " } else { "󰉋 " }
        } else {
            match item.path.extension().and_then(|e| e.to_str()) {
                Some("rs") => " ",
                Some("md") => " ",
                _ => "󰈔 ",
            }
        };

        let style = if i == app.explorer.selected_idx && app.focus == Focus::Explorer {
            Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg)
        };

        ListItem::new(format!("{}{}{} {}", indent, icon, item.name, if item.is_dir && !item.expanded { "›" } else if item.is_dir { "⌄" } else { "" }))
            .style(style)
    }).collect();

    let block = Block::default()
        .title(format!(" {} ", app.i18n.t("explorer")))
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(colors.surface));

    f.render_widget(List::new(items).block(block).bg(colors.bg), area);
}

fn draw_editor(f: &mut Frame, app: &App, area: Rect, buffer_idx: usize, is_focused: bool, colors: &UIColors) {
    let buffer = match app.buffers.get(buffer_idx) {
        Some(b) => b,
        None => return,
    };
    
    // Ensure scroll is correct for this area
    let mut buffer_scroll_row = buffer.scroll_row;
    let height = area.height as usize;
    if buffer.cursor_row < buffer_scroll_row {
        buffer_scroll_row = buffer.cursor_row;
    } else if buffer.cursor_row >= buffer_scroll_row + height {
        buffer_scroll_row = buffer.cursor_row.saturating_sub(height).saturating_add(1);
    }

    let theme = app.theme_set.themes.get(&app.current_theme)
        .unwrap_or_else(|| &app.theme_set.themes["base16-ocean.dark"]);
        
    let syntax = buffer.path.as_ref()
        .and_then(|p| p.extension())
        .and_then(|e| app.syntax_set.find_syntax_by_extension(e.to_str().unwrap_or("")))
        .unwrap_or_else(|| app.syntax_set.find_syntax_plain_text());
    
    let mut h = HighlightLines::new(syntax, theme);

    let line_count = buffer.content.len_lines();
    let mut lines = Vec::new();

    for i in buffer_scroll_row..(buffer_scroll_row + height).min(line_count) {
        let line_content = buffer.content.line(i).to_string();
        let mut spans = Vec::new();
        
        // Line number
        let line_num_style = if i == buffer.cursor_row && is_focused {
            Style::default().fg(colors.accent).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.surface)
        };
        spans.push(Span::styled(format!("{:3} ", i + 1), line_num_style));

        // Highlight and Selection
        let ranges: Vec<(syntect::highlighting::Style, &str)> = h.highlight_line(&line_content, &app.syntax_set).unwrap();
        let mut char_offset = 0;
        let mut visual_col = 0;
        
        for (s, text) in ranges {
            let fg = Color::Rgb(s.foreground.r, s.foreground.g, s.foreground.b);
            
            for c in text.chars() {
                // Horizontal scroll check: use visual_col for scrolling, but char_offset for selection logic
                if visual_col >= buffer.scroll_col && visual_col < buffer.scroll_col + (area.width as usize - 5) {
                    let mut style = Style::default().fg(fg);
                    
                    // Selection logic (character-accurate)
                    if let Some((start_row, start_col)) = buffer.selection_start {
                        let (r1, c1, r2, c2) = if (start_row, start_col) < (buffer.cursor_row, buffer.cursor_col) {
                            (start_row, start_col, buffer.cursor_row, buffer.cursor_col)
                        } else {
                            (buffer.cursor_row, buffer.cursor_col, start_row, start_col)
                        };

                        let is_selected = if i > r1 && i < r2 {
                            true
                        } else if i == r1 && i == r2 {
                            char_offset >= c1 && char_offset < c2
                        } else if i == r1 {
                            char_offset >= c1
                        } else if i == r2 {
                            char_offset < c2
                        } else {
                            false
                        };

                        if is_selected {
                            style = style.bg(colors.sel);
                        }
                    }
                    
                    let disp_text = if c == '\t' { " ".repeat(4) } else { c.to_string() };
                    spans.push(Span::styled(disp_text, style));
                }
                
                visual_col += if c == '\t' { 4 } else { 1 };
                char_offset += 1;
            }
        }

        // Ghost Text (Autocomplete suggestion)
        if i == buffer.cursor_row && !buffer.autocomplete_options.is_empty() && !buffer.show_autocomplete_list {
            if let Some(opt) = buffer.autocomplete_options.get(0) {
                let prefix = buffer.get_current_word_prefix();
                if opt.starts_with(&prefix) {
                    let ghost = &opt[prefix.len()..];
                    spans.push(Span::styled(ghost, Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)));
                }
            }
        }

        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines).bg(colors.bg), area);

    // Autocomplete List Popup
    if is_focused && buffer.show_autocomplete_list && !buffer.autocomplete_options.is_empty() {
        let list_width = buffer.autocomplete_options.iter().map(|o| o.len()).max().unwrap_or(10) as u16 + 4;
        let list_height = buffer.autocomplete_options.len().min(8) as u16;
        
        let popup_area = Rect {
            x: area.x + 4 + buffer.cursor_col as u16,
            y: area.y + (buffer.cursor_row - buffer_scroll_row) as u16 + 1,
            width: list_width,
            height: list_height,
        };

        // Constrain popup area to editor area
        let popup_area = Rect {
            x: popup_area.x.min(area.x + area.width - popup_area.width),
            y: if popup_area.y + popup_area.height > area.y + area.height {
                popup_area.y.saturating_sub(popup_area.height + 1)
            } else {
                popup_area.y
            },
            ..popup_area
        };

        let items: Vec<ListItem> = buffer.autocomplete_options.iter().enumerate().map(|(idx, opt)| {
            let style = if idx == buffer.autocomplete_idx {
                Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg)
            };
            ListItem::new(format!(" {}", opt)).style(style)
        }).collect();

        f.render_widget(Clear, popup_area);
        f.render_widget(List::new(items).block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(colors.accent))), popup_area);
    }

    // Cursor
    if is_focused && app.focus == Focus::Editor {
        f.set_cursor_position((
            area.x + 4 + (buffer.cursor_col - buffer.scroll_col) as u16,
            area.y + (buffer.cursor_row - buffer_scroll_row) as u16,
        ));
    }
}

fn draw_notification(f: &mut Frame, msg: &str, ntype: &crate::app::NotificationType, area: Rect, _colors: &UIColors) {
    let (bg, icon) = match ntype {
        crate::app::NotificationType::Error => (Color::Rgb(191, 97, 106), " Error "),
        crate::app::NotificationType::Info => (Color::Rgb(129, 161, 193), " Info "),
    };
    let text = format!("{}{}", icon, msg);
    f.render_widget(Paragraph::new(text).bg(bg).fg(Color::Rgb(30, 30, 46)), area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    let ws_text = if let Some(ws) = &app.current_workspace {
        format!(" 󰘳 {} ", ws)
    } else {
        String::new()
    };

    let left_text = if app.is_welcome || app.buffers.is_empty() {
        format!(" {} | {} {} | {} {} ", 
            app.i18n.t("welcome_to_nedit"),
            app.config.get_keybind("theme_select").to_uppercase(),
            app.i18n.t("select_themes"),
            app.config.get_keybind("open_help").to_uppercase(),
            app.i18n.t("for_help")
        )
    } else if let Some(buffer) = app.buffers.get(app.current_buffer_idx) {
        let file_name = buffer.path.as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[No Name]".to_string());
        
        let modified = if buffer.modified { "*" } else { "" };
        let read_only = if buffer.is_read_only { format!(" {}", app.i18n.t("read_only")) } else { "".to_string() };
        format!(" {} {} {} | {}: {} | {}: {}, {}: {} ", 
            file_name, modified, read_only, 
            app.i18n.t("theme"), app.current_theme, 
            app.i18n.t("row"), buffer.cursor_row + 1, 
            app.i18n.t("col"), buffer.cursor_col + 1
        )
    } else {
        String::new()
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(ws_text.chars().count() as u16),
        ])
        .split(area);

    f.render_widget(Paragraph::new(left_text).bg(colors.accent).fg(colors.bg), chunks[0]);
    f.render_widget(Paragraph::new(ws_text).bg(colors.accent).fg(colors.bg).alignment(ratatui::layout::Alignment::Right), chunks[1]);
}

fn draw_welcome_screen(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    let logo = r#"
    ███╗   ██╗███████╗██████╗ ██╗████████╗
    ████╗  ██║██╔════╝██╔══██╗██║╚══██╔══╝
    ██╔██╗ ██║█████╗  ██║  ██║██║   ██║   
    ██║╚██╗██║██╔══╝  ██║  ██║██║   ██║   
    ██║ ╚████║███████╗██████╔╝██║   ██║   
    ╚═╝  ╚═══╝╚══════╝╚═════╝ ╚═╝   ╚═╝   
    "#;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Length(10),
        ])
        .split(area);

    let logo_para = Paragraph::new(logo)
        .style(Style::default().fg(colors.accent))
        .alignment(ratatui::layout::Alignment::Center);
    
    f.render_widget(logo_para, chunks[0]);

    let shortcuts = vec![
        Line::from(vec![Span::styled(app.config.get_keybind("new_file").to_uppercase(), Style::default().fg(colors.accent)), Span::raw(format!("  {}", app.i18n.t("new_file")))]),
        Line::from(vec![Span::styled(app.config.get_keybind("toggle_explorer").to_uppercase(), Style::default().fg(colors.accent)), Span::raw(format!("  {}", app.i18n.t("file_explorer")))]),
        Line::from(vec![Span::styled(app.config.get_keybind("open_file").to_uppercase(), Style::default().fg(colors.accent)), Span::raw(format!("  {}", app.i18n.t("open_file_fuzzy")))]),
        Line::from(vec![Span::styled(app.config.get_keybind("global_search").to_uppercase(), Style::default().fg(colors.accent)), Span::raw(format!("  {}", app.i18n.t("global_search")))]),
        Line::from(vec![Span::styled(app.config.get_keybind("theme_select").to_uppercase(), Style::default().fg(colors.accent)), Span::raw(format!("  {}", app.i18n.t("select_theme")))]),
    ];

    let paragraph = Paragraph::new(shortcuts)
        .alignment(ratatui::layout::Alignment::Center);
    
    f.render_widget(paragraph, chunks[1]);
}

fn draw_fuzzy_finder(f: &mut Frame, app: &App, colors: &UIColors) {
    let is_small = matches!(app.fuzzy_mode, 
        FuzzyMode::WorkspaceAddName | FuzzyMode::WorkspaceAddPath | FuzzyMode::SaveAs | FuzzyMode::Rename | FuzzyMode::DeleteConfirm | FuzzyMode::NewFolder
    );

    let area = if is_small {
        let centered_y = (f.area().height.saturating_sub(3)) / 2;
        let centered_x = (f.area().width.saturating_sub(70)) / 2;
        Rect::new(centered_x, centered_y, 70.min(f.area().width), 3)
    } else {
        centered_rect(70, 50, f.area())
    };
    
    f.render_widget(Clear, area);
    
    let title = match app.fuzzy_mode {
        FuzzyMode::Content => format!("   {} ", app.i18n.t("global_search_content")),
        FuzzyMode::Local => format!("   {} ", app.i18n.t("local_search_file")),
        FuzzyMode::Files => format!("   {} ", app.i18n.t("fuzzy_finder_files")),
        FuzzyMode::Themes => format!(" 󰏘  {} ", app.i18n.t("select_color_theme")),
        FuzzyMode::SaveAs => format!(" 󰆓  {} ", app.i18n.t("save_as")),
        FuzzyMode::Rename => format!(" 󰏫  {} ", app.i18n.t("rename")),
        FuzzyMode::DeleteConfirm => format!(" 󰆴  {} ", app.i18n.t("delete_confirm")),
        FuzzyMode::FileOptions => format!(" 󰘳  {} ", app.i18n.t("file_options")),
        FuzzyMode::Workspaces => format!(" 󰘳  {} ", app.i18n.t("workspaces")),
        FuzzyMode::WorkspaceAddName => format!(" 󰏫  {} ", app.i18n.t("add_workspace_name")),
        FuzzyMode::WorkspaceAddPath => format!(" 󰆓  {} ", app.i18n.t("add_workspace_path")),
        FuzzyMode::CommandPalette => format!(" 󰘳  {} ", app.i18n.t("command_palette")),
        FuzzyMode::Move => format!(" 󰏫  {} ", app.i18n.t("move_file")),
        FuzzyMode::RunScript => format!(" 󰢱  Run Lua Script "),
        FuzzyMode::ScriptConfirm => format!(" 󰢱  Confirm Actions (Enter to Apply, Esc to Cancel) "),
        FuzzyMode::EditScript => format!(" 󰝎  Edit Lua Script "),
        FuzzyMode::DeleteScript => format!(" 󰆴  Delete Lua Script "),
        FuzzyMode::DocSelect => format!(" 󰈔  Select Documentation "),
        FuzzyMode::NewFolder => format!(" 󰉋  New Folder Name "),
    };

    let block = Block::default()
        .title(Line::from(title).style(Style::default().fg(colors.accent).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors.accent))
        .bg(colors.bg);
    
    let constraints = if is_small {
        vec![Constraint::Length(1)]
    } else {
        vec![
            Constraint::Length(1), // Input
            Constraint::Length(1), // Divider
            Constraint::Min(0),    // Results
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(area);

    let input = if app.fuzzy_mode == FuzzyMode::Move {
        let dir_str = app.move_dir.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        Paragraph::new(Line::from(vec![
            Span::styled(format!(" 󰉋 {} > ", dir_str), Style::default().fg(colors.accent)),
            Span::raw(&app.fuzzy_query),
            Span::styled(" (Tab: Move here, Enter: Open folder)", Style::default().fg(colors.surface)),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" 󰍉 ", Style::default().fg(colors.accent)),
            Span::raw(&app.fuzzy_query),
        ]))
    };
    f.render_widget(input, chunks[0]);
    
    if !is_small {
        f.render_widget(Block::default().borders(Borders::TOP).border_style(Style::default().fg(colors.surface)), chunks[1]);

        let list_height = chunks[2].height as usize;
        let start_idx = app.fuzzy_idx.saturating_sub(list_height / 2);
        
        let items: Vec<ListItem> = if app.fuzzy_mode == FuzzyMode::Local {
            let safe_start = start_idx.min(app.fuzzy_lines.len().saturating_sub(1));
            let end_idx = (safe_start + list_height).min(app.fuzzy_lines.len());
            if app.fuzzy_lines.is_empty() { vec![] } else {
                app.fuzzy_lines[safe_start..end_idx].iter().enumerate().map(|(idx, (line_num, text))| {
                    let i = safe_start + idx;
                    let style = if i == app.fuzzy_idx {
                        Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    ListItem::new(format!(" {}: {}", line_num + 1, text.trim())).style(style)
                }).collect()
            }
        } else if app.fuzzy_mode == FuzzyMode::Content {
            let safe_start = start_idx.min(app.fuzzy_global_results.len().saturating_sub(1));
            let end_idx = (safe_start + list_height).min(app.fuzzy_global_results.len());
            if app.fuzzy_global_results.is_empty() { vec![] } else {
                app.fuzzy_global_results[safe_start..end_idx].iter().enumerate().map(|(idx, (path, line_num, text))| {
                    let i = safe_start + idx;
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let style = if i == app.fuzzy_idx {
                        Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    ListItem::new(format!(" {} (L{}): {}", name, line_num + 1, text.trim())).style(style)
                }).collect()
            }
        } else if app.fuzzy_mode == FuzzyMode::Themes {
            let safe_start = start_idx.min(app.fuzzy_themes.len().saturating_sub(1));
            let end_idx = (safe_start + list_height).min(app.fuzzy_themes.len());
            if app.fuzzy_themes.is_empty() { vec![] } else {
                app.fuzzy_themes[safe_start..end_idx].iter().enumerate().map(|(idx, theme_name)| {
                    let i = safe_start + idx;
                    let style = if i == app.fuzzy_idx {
                        Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    let indicator = if theme_name == &app.current_theme { "󰄬 " } else { "  " };
                    ListItem::new(format!(" {} {}", indicator, theme_name)).style(style)
                }).collect()
            }
        } else if app.fuzzy_mode == FuzzyMode::ScriptConfirm {
            let actions = &app.pending_lua_actions;
            if actions.is_empty() {
                vec![ListItem::new(" (No actions — script ran with no changes)").style(Style::default().fg(colors.surface))]
            } else {
                let safe_start = start_idx.min(actions.len().saturating_sub(1));
                let end_idx = (safe_start + list_height).min(actions.len());
                actions[safe_start..end_idx].iter().enumerate().map(|(idx, action)| {
                    let i = safe_start + idx;
                    let style = if i == app.fuzzy_idx {
                        Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    ListItem::new(format!(" 󰢱  {}", action.description())).style(style)
                }).collect()
            }
        } else if matches!(app.fuzzy_mode, FuzzyMode::CommandPalette | FuzzyMode::FileOptions | FuzzyMode::Workspaces | FuzzyMode::RunScript | FuzzyMode::DocSelect) {
            if app.fuzzy_results.is_empty() {
                vec![]
            } else {
                let safe_start = start_idx.min(app.fuzzy_results.len().saturating_sub(1));
                let end_idx = (safe_start + list_height).min(app.fuzzy_results.len());
                app.fuzzy_results[safe_start..end_idx].iter().enumerate().map(|(idx, path)| {
                    let i = safe_start + idx;
                    let name = if app.fuzzy_mode == FuzzyMode::RunScript {
                        let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                        if let Ok(content) = std::fs::read_to_string(path) {
                            if let Some(first) = content.lines().next() {
                                let trimmed = first.trim();
                                if trimmed.starts_with("-- ") { trimmed[3..].trim().to_string() }
                                else { stem }
                            } else { stem }
                        } else { stem }
                    } else {
                        path.file_name().unwrap_or_default().to_string_lossy().to_string()
                    };
                    let style = if i == app.fuzzy_idx {
                        Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    let icon = match app.fuzzy_mode {
                        FuzzyMode::CommandPalette => {
                            match name.as_str() {
                                "Save" => "󰆓 ",
                                "New File" => "󰝒 ",
                                "Open File" => "󰈞 ",
                                "Close Tab" => "󰅖 ",
                                "Toggle Explorer" => "󰙅 ",
                                "Global Search" => "󰈗 ",
                                "Local Search" => "󰩊 ",
                                "Switch Theme" => "󰔎 ",
                                "Workspaces" => "󰉋 ",
                                "Open Lua Script" => "󰢱 ",
                                "Run Lua Script" => "󰐊 ",
                                "Edit Lua Script" => "󰏫 ",
                                "Delete Lua Script" => "󰆴 ",
                                "Quit" => "󰈆 ",
                                "Undo" => "󰕌 ",
                                "Redo" => "󰕍 ",
                                "Copy" => "󰆏 ",
                                "Paste" => "󰆑 ",
                                "Cut" => "󰆐 ",
                                "Select All" => "󰒅 ",
                                "Open Help" => "󰘥 ",
                                _ => "󰘳 ",
                            }
                        },
                        FuzzyMode::FileOptions => {
                            match name.as_str() {
                                "Rename" => "󰏫 ",
                                "Move" => "󰪹 ",
                                "Delete" => "󰆴 ",
                                _ => "󰘳 ",
                            }
                        },
                        FuzzyMode::Workspaces => {
                            match name.as_str() {
                                "Exit Workspace" => "󰈆 ",
                                "New Workspace..." => "󰉋 ",
                                _ => "󰉋 ",
                            }
                        },
                        FuzzyMode::RunScript => "󰢱 ",
                        FuzzyMode::DocSelect => {
                            match name.as_str() {
                                "docs.md" => "󰘥 ",
                                "lua.md" => "󰢱 ",
                                "binds.md" => "󰘳 ",
                                _ => "󰈔 ",
                            }
                        },
                        _ => "  ",
                    };
                    ListItem::new(format!(" {} {}", icon, name)).style(style)
                }).collect()
            }
        } else {
            if app.fuzzy_results.is_empty() {
                vec![]
            } else {
                let safe_start = start_idx.min(app.fuzzy_results.len().saturating_sub(1));
                let end_idx = (safe_start + list_height).min(app.fuzzy_results.len());
                app.fuzzy_results[safe_start..end_idx].iter().enumerate().map(|(idx, path)| {
                    let i = safe_start + idx;
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let rel_path = path.strip_prefix(&app.explorer.root).unwrap_or(path).to_string_lossy();
                    let style = if i == app.fuzzy_idx {
                        Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    let icon = if path.is_dir() { "\u{f016b} " } else { "\u{f0214} " };
                    ListItem::new(format!(" {} {} ({})", icon, name, rel_path)).style(style)
                }).collect()
            }
        };

        f.render_widget(List::new(items), chunks[2]);
    }

    f.render_widget(block, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
