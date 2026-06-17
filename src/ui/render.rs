use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use std::path::Path;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
use syntect::parsing::{ParseState, ScopeStack};

use crate::app::{App, Focus, FuzzyMode};
use crate::buffer::column::TAB_WIDTH;

use super::welcome::draw_welcome_screen;
use super::{centered_rect, get_colors, UIColors};

pub fn render(f: &mut Frame, app: &mut App) {
    let colors = get_colors(app);

    f.render_widget(Block::default().bg(colors.bg), f.area());

    let chunks = if app.notification.is_some() {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(f.area())
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(f.area())
    };

    // We calculate the explorer width dynamically based on the longest filename 
    // to minimize wasted space while ensuring names remain readable.
    let explorer_width = if app.show_explorer {
        let max_len = app.explorer.max_item_width;
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
        app.explorer_area = main_chunks[0];
        draw_explorer(f, app, main_chunks[0], &colors);
    } else {
        app.explorer_area = Rect::default();
    }

    let editor_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(main_chunks[1]);

    app.editor_area = editor_chunks[1];

    draw_tab_bar(f, app, editor_chunks[0], &colors);

    if app.is_welcome {
        draw_welcome_screen(f, app, editor_chunks[1], &colors);
    } else if app.live_script_mode {
        app.ensure_current_theme_loaded();

        let target_idx = app.target_buffer_idx.unwrap_or(0);
        let script_idx = app.live_script_buffer_idx.unwrap_or(0);
        let target_path = app.buffers.get(target_idx).and_then(|b| b.path.clone());
        let script_path = app.buffers.get(script_idx).and_then(|b| b.path.clone());
        app.ensure_syntax_for_path_loading(target_path.as_deref());
        app.ensure_syntax_for_path_loading(script_path.as_deref());

        if target_idx < app.buffers.len() && script_idx < app.buffers.len() {
            let split_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),
                    Constraint::Length(1),
                    Constraint::Percentage(50),
                ])
                .split(editor_chunks[1]);
            let target_area = split_chunks[0];
            let separator_area = split_chunks[1];
            let script_area = split_chunks[2];

            if let Some(target_buf) = app.buffers.get_mut(target_idx) {
                let width = target_area.width as usize;
                target_buf.move_cursor(0, 0, width);
            }

            if let Some(script_buf) = app.buffers.get_mut(script_idx) {
                let width = script_area.width as usize;
                script_buf.move_cursor(0, 0, width);
            }

            draw_editor(
                f,
                app,
                target_area,
                target_idx,
                app.current_buffer_idx == target_idx,
                &colors,
            );
            draw_split_separator(f, separator_area, &colors);
            draw_editor(
                f,
                app,
                script_area,
                script_idx,
                app.current_buffer_idx == script_idx,
                &colors,
            );
        } else if !app.buffers.is_empty() {
            draw_editor(
                f,
                app,
                editor_chunks[1],
                app.current_buffer_idx.min(app.buffers.len() - 1),
                true,
                &colors,
            );
        }
    } else if !app.buffers.is_empty() {
        app.ensure_current_theme_loaded();
        let current_path = app
            .buffers
            .get(app.current_buffer_idx)
            .and_then(|b| b.path.clone());
        app.ensure_syntax_for_path_loading(current_path.as_deref());

        {
            let buffer = &mut app.buffers[app.current_buffer_idx];
            let width = editor_chunks[1].width as usize;
            buffer.move_cursor(0, 0, width);
        }

        draw_editor(
            f,
            app,
            editor_chunks[1],
            app.current_buffer_idx,
            true,
            &colors,
        );
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

fn draw_split_separator(f: &mut Frame, area: Rect, colors: &UIColors) {
    f.render_widget(Block::default().bg(colors.surface), area);
}

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    if app.buffers.is_empty() {
        return;
    }

    let mut spans = Vec::new();
    for (i, buffer) in app.buffers.iter().enumerate() {
        let is_live_script = Some(i) == app.live_script_buffer_idx;
        if is_live_script {
            continue;
        }

        let name = buffer
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[No Name]".to_string());

        let modified = if buffer.modified { "*" } else { "" };
        let is_current = i == app.current_buffer_idx;
        let style = if is_current {
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD)
                .bg(colors.sel)
        } else {
            Style::default().fg(colors.fg)
        };

        let icon = if let Some(path) = &buffer.path {
            app.icon_registry.get_icon(path, false, false)
        } else {
            "󰈔 "
        };

        spans.push(Span::styled(
            format!(" {} {} {} ", icon, name, modified),
            style,
        ));
        spans.push(Span::raw(" "));
    }

    f.render_widget(Paragraph::new(Line::from(spans)).bg(colors.bg), area);
}

fn draw_explorer(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    if app.explorer.items.is_empty() {
        let block = Block::default()
            .title(format!(" {} ", app.i18n.t("explorer")))
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(colors.surface));
        f.render_widget(block.bg(colors.bg), area);
        return;
    }
    let list_height = area.height.saturating_sub(2) as usize;
    let scroll_offset = app
        .explorer
        .scroll_offset
        .min(app.explorer.items.len().saturating_sub(1));
    let visible_items = &app.explorer.items
        [scroll_offset..((scroll_offset + list_height).min(app.explorer.items.len()))];

    let items: Vec<ListItem> = visible_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let actual_idx = i + scroll_offset;
            let indent = "  ".repeat(item.depth);
            let icon = app
                .icon_registry
                .get_icon(&item.path, item.is_dir, item.expanded);

            let style = if actual_idx == app.explorer.selected_idx && app.focus == Focus::Explorer {
                Style::default()
                    .bg(colors.sel)
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.fg)
            };

            ListItem::new(format!(
                "{}{}{} {}",
                indent,
                icon,
                item.name,
                if item.is_dir && !item.expanded {
                    "›"
                } else if item.is_dir {
                    "⌄"
                } else {
                    ""
                }
            ))
            .style(style)
        })
        .collect();

    let block = Block::default()
        .title(format!(" {} ", app.i18n.t("explorer")))
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(colors.surface));

    f.render_widget(List::new(items).block(block).bg(colors.bg), area);
}

fn visual_leading_indent(line: &str) -> usize {
    let mut col = 0;
    for c in line.chars() {
        match c {
            ' ' => col += 1,
            '\t' => col += 4,
            _ => break,
        }
    }
    col
}

/// Returns `(active_level, scope_start, scope_end)` for the indent guide at the cursor.
/// Active level follows the parent visual scope of the cursor line.
/// Vertical scope uses each line's leading indent to bound the highlighted block.
fn active_indent_guide_scope(
    line_count: usize,
    line_indent: impl Fn(usize) -> usize,
    cursor_row: usize,
    tab_width: usize,
) -> (usize, usize, usize) {
    let indent_level = line_indent(cursor_row) / tab_width;
    let active_level = if indent_level > 1 {
        indent_level - 1
    } else {
        indent_level
    };
    if active_level == 0 {
        return (0, cursor_row, cursor_row);
    }

    let threshold = active_level * tab_width;
    let mut start = cursor_row;
    while start > 0 && line_indent(start - 1) >= threshold {
        start -= 1;
    }

    let mut end = cursor_row;
    while end + 1 < line_count && line_indent(end + 1) >= threshold {
        end += 1;
    }

    (active_level, start, end)
}

fn draw_editor(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    buffer_idx: usize,
    is_focused: bool,
    colors: &UIColors,
) {
    let buffer = match app.buffers.get(buffer_idx) {
        Some(b) => b,
        None => return,
    };

    let matching_bracket = buffer.find_matching_bracket();

    let mut buffer_scroll_row = buffer.scroll_row;
    let height = area.height as usize;
    let max_scroll = buffer.content.len_lines().saturating_sub(1);
    buffer_scroll_row = buffer_scroll_row.min(max_scroll);

    let (theme, syntax_set) = {
        let theme = app
            .theme_set
            .themes
            .get(&app.current_theme)
            .or_else(|| app.theme_set.themes.get("base16-ocean.dark"))
            .or_else(|| app.theme_set.themes.values().next())
            .expect("No themes loaded — check your theme directory");
        (theme, app.syntax_set.as_ref())
    };

    let buffer = match app.buffers.get_mut(buffer_idx) {
        Some(b) => b,
        None => return,
    };

    let line_count = buffer.content.len_lines();
    let mut syntax_highlighter = syntax_set.map(|syntax_set| {
        let syntax = buffer
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| syntax_set.find_syntax_by_extension(e.to_str().unwrap_or("")))
            .unwrap_or_else(|| syntax_set.find_syntax_plain_text());

        let highlighter = Highlighter::new(theme);

        let (ps, hs) = if buffer_scroll_row > 0 {
            let prev_row = buffer_scroll_row - 1;
            if prev_row < buffer.syntax_states.len() && buffer.syntax_states[prev_row].is_some() {
                let (ps, hs) = buffer.syntax_states[prev_row].as_ref().unwrap();
                (ps.clone(), hs.clone())
            } else {
                let mut last_known_row = None;
                for j in (0..prev_row.min(buffer.syntax_states.len())).rev() {
                    if buffer.syntax_states[j].is_some() {
                        last_known_row = Some(j);
                        break;
                    }
                }

                let (mut ps, mut hs) = if let Some(j) = last_known_row {
                    let (ps, hs) = buffer.syntax_states[j].as_ref().unwrap();
                    (ps.clone(), hs.clone())
                } else {
                    (
                        ParseState::new(syntax),
                        HighlightState::new(&highlighter, ScopeStack::new()),
                    )
                };

                let start_at = last_known_row.map(|j| j + 1).unwrap_or(0);
                for k in start_at..=prev_row {
                    if k >= buffer.content.len_lines() {
                        break;
                    }
                    let line_str = buffer.content.line(k).to_string();
                    let ops = ps.parse_line(&line_str, syntax_set).unwrap_or_default();
                    let _ =
                        HighlightIterator::new(&mut hs, &ops, &line_str, &highlighter).collect::<Vec<_>>();
                    if k < buffer.syntax_states.len() {
                        buffer.syntax_states[k] = Some((ps.clone(), hs.clone()));
                    }
                }
                (ps, hs)
            }
        } else {
            (
                ParseState::new(syntax),
                HighlightState::new(&highlighter, ScopeStack::new()),
            )
        };

        (highlighter, ps, hs)
    });

    let mut lines = Vec::new();
    let visible_width = area.width.saturating_sub(5) as usize;
    let (active_indent_level, active_scope_start, active_scope_end) = if is_focused {
        let line_without_newline = |row: usize| {
            let mut line = buffer.content.line(row).to_string();
            if line.ends_with('\n') {
                line.pop();
            }
            if line.ends_with('\r') {
                line.pop();
            }
            line
        };
        let line_indent = |row: usize| visual_leading_indent(&line_without_newline(row));
        active_indent_guide_scope(
            line_count,
            line_indent,
            buffer.cursor_row,
            TAB_WIDTH,
        )
    } else {
        (usize::MAX, 0, 0)
    };
    let show_guides = app.config.show_indent_guides;
    let selected_match_text = buffer.selection_start.and_then(|start| {
        let start_idx = buffer.to_char_idx(start.0, start.1);
        let end_idx = buffer.to_char_idx(buffer.cursor_row, buffer.cursor_col);
        let (selection_start, selection_end) = if start_idx < end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };
        let selection_len = selection_end.saturating_sub(selection_start);

        if selection_len == 0 || selection_len >= 100 {
            return None;
        }

        let text = buffer.content.slice(selection_start..selection_end).to_string();
        if text.trim().is_empty() {
            None
        } else {
            Some(text)
        }
    });
    let selected_match_chars = selected_match_text
        .as_ref()
        .map(|text| text.chars().collect::<Vec<_>>());

    for i in buffer_scroll_row..(buffer_scroll_row + height).min(line_count) {
        let original_line = buffer.content.line(i).to_string();
        let mut line_content = original_line.clone();
        if line_content.ends_with('\n') {
            line_content.pop();
        }
        if line_content.ends_with('\r') {
            line_content.pop();
        }

        let mut match_ranges = Vec::new();
        if let Some(word_chars) = selected_match_chars.as_ref() {
            let line_chars: Vec<char> = line_content.chars().collect();
            if !word_chars.is_empty() && line_chars.len() >= word_chars.len() {
                for i in 0..=(line_chars.len() - word_chars.len()) {
                    if line_chars[i..i + word_chars.len()] == word_chars[..] {
                        match_ranges.push(i..i + word_chars.len());
                    }
                }
            }
        }

        let mut spans = Vec::new();

        let line_num_style = if i == buffer.cursor_row && is_focused {
            Style::default()
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.surface)
        };
        spans.push(Span::styled(format!("{:3} ", i + 1), line_num_style));

        let ranges: Vec<(Color, &str)> =
            if let (Some((ref highlighter, ref mut ps, ref mut hs)), Some(syntax_set)) =
                (syntax_highlighter.as_mut(), syntax_set)
            {
                let ops = ps.parse_line(&original_line, syntax_set).unwrap_or_default();
                if i < buffer.syntax_states.len() {
                    buffer.syntax_states[i] = Some((ps.clone(), hs.clone()));
                }
                HighlightIterator::new(hs, &ops, &original_line, highlighter)
                    .map(|(s, text)| {
                        (
                            Color::Rgb(s.foreground.r, s.foreground.g, s.foreground.b),
                            text,
                        )
                    })
                    .collect()
            } else {
                vec![(colors.fg, line_content.as_str())]
            };
        let mut char_offset = 0;
        let mut visual_col = 0;
        let mut in_leading = show_guides;

        for (fg, text) in ranges {
            for c in text.chars() {
                if c == '\n' || c == '\r' {
                    continue;
                }

                let char_width = if c == '\t' { TAB_WIDTH } else { 1 };

                if in_leading && c != ' ' && c != '\t' {
                    in_leading = false;
                }

                if visual_col >= buffer.scroll_col && visual_col < buffer.scroll_col + visible_width
                {
                    if in_leading {
                        for col_offset in 0..char_width {
                            let col = visual_col + col_offset;
                            if col % TAB_WIDTH == 0 && col > 0 {
                                let indent_level = col / TAB_WIDTH;
                                let guide_color = if indent_level == active_indent_level
                                    && i >= active_scope_start
                                    && i <= active_scope_end
                                {
                                    colors.active_indent_guide
                                } else {
                                    colors.indent_guide
                                };
                                spans.push(Span::styled(
                                    "│",
                                    Style::default().fg(guide_color),
                                ));
                            } else {
                                spans.push(Span::styled(
                                    " ",
                                    Style::default().fg(colors.indent_guide),
                                ));
                            }
                        }
                    } else {
                        let mut style = Style::default().fg(fg);

                        if let Some((start_row, start_col)) = buffer.selection_start {
                            let (r1, c1, r2, c2) =
                                if (start_row, start_col) < (buffer.cursor_row, buffer.cursor_col)
                                {
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
                            } else {
                                let is_match = match_ranges
                                    .iter()
                                    .any(|range| {
                                        char_offset >= range.start && char_offset < range.end
                                    });
                                if is_match {
                                    style = style.bg(colors.surface);
                                }
                            }
                        } else {
                            let is_match = match_ranges
                                .iter()
                                .any(|range| {
                                    char_offset >= range.start && char_offset < range.end
                                });
                            if is_match {
                                style = style.bg(colors.surface);
                            }
                        }

                        if matching_bracket == Some((i, char_offset))
                            || (matching_bracket.is_some()
                                && i == buffer.cursor_row
                                && char_offset == buffer.cursor_col)
                        {
                            style = style.bg(colors.accent).fg(colors.bg);
                        }

                        let disp_text = if c == '\t' {
                            " ".repeat(TAB_WIDTH)
                        } else {
                            c.to_string()
                        };
                        spans.push(Span::styled(disp_text, style));
                    }
                }

                visual_col += char_width;
                char_offset += 1;
            }
        }

        if i == buffer.cursor_row && !buffer.autocomplete_options.is_empty() {
            if let Some(opt) = buffer.autocomplete_options.get(buffer.autocomplete_idx) {
                let prefix = buffer.get_current_word_prefix();
                if opt.starts_with(&prefix) {
                    let ghost = &opt[prefix.len()..];
                    spans.push(Span::styled(
                        ghost,
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::ITALIC),
                    ));
                }
            }
        }

        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines).bg(colors.bg), area);

    if is_focused && app.focus == Focus::Editor {
        let cursor_x = area.x + 4 + buffer.cursor_col.saturating_sub(buffer.scroll_col) as u16;
        let cursor_y = area.y + buffer.cursor_row.saturating_sub(buffer_scroll_row) as u16;
        if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
            f.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

fn draw_notification(
    f: &mut Frame,
    msg: &str,
    ntype: &crate::app::NotificationType,
    area: Rect,
    colors: &UIColors,
) {
    let (bg, icon) = match ntype {
        crate::app::NotificationType::Error => (colors.error, " Error "),
        crate::app::NotificationType::Info => (colors.accent, " Info "),
    };
    let text = format!("{}{}", icon, msg);
    f.render_widget(Paragraph::new(text).bg(bg).fg(colors.bg), area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    let mode_text = if app.is_welcome {
        " WELCOME "
    } else if app.is_fuzzy {
        " FUZZY "
    } else {
        match app.focus {
            Focus::Explorer => " EXPLORER ",
            Focus::Editor => " EDITOR ",
        }
    };
    let mode_color = colors.accent;

    let mode_span = Span::styled(
        mode_text,
        Style::default()
            .bg(mode_color)
            .fg(colors.bg)
            .add_modifier(Modifier::BOLD),
    );

    let mode_sep = Span::styled("", Style::default().bg(colors.surface).fg(mode_color));

    // File info
    let mut file_spans = Vec::new();
    if !app.is_welcome && !app.buffers.is_empty() {
        if let Some(buffer) = app.buffers.get(app.current_buffer_idx) {
            let path = buffer.path.as_deref().unwrap_or(Path::new("[No Name]"));
            let icon = app.icon_registry.get_icon(path, false, false);

            file_spans.push(Span::styled(
                format!(" {} ", icon),
                Style::default().fg(mode_color),
            ));

            let components: Vec<_> = path.components().collect();
            let path_str = if components.len() > 4 {
                let last_parts: Vec<_> = components.iter().rev().take(4).rev().collect();
                let mut p = String::new();
                for (i, part) in last_parts.iter().enumerate() {
                    if i > 0 {
                        p.push('/');
                    }
                    p.push_str(&part.as_os_str().to_string_lossy());
                }
                format!(".../{}", p)
            } else {
                path.to_string_lossy().to_string()
            };

            file_spans.push(Span::styled(path_str, Style::default().fg(colors.fg)));
            if buffer.modified {
                file_spans.push(Span::styled(
                    " ●",
                    Style::default().fg(colors.accent),
                ));
            }
        }
    }

    // Stats (Right side)
    let stats_text = if let Some(buffer) = app.buffers.get(app.current_buffer_idx) {
        format!("  {}:{} ", buffer.cursor_row + 1, buffer.cursor_col + 1)
    } else {
        String::new()
    };

    let left_spans = [vec![mode_span, mode_sep], file_spans].concat();
    let left_line = Line::from(left_spans);

    let right_line = Line::from(vec![Span::styled(
        stats_text,
        Style::default().fg(colors.fg),
    )]);

    // Calculate available width for shortcuts
    let left_width = left_line.width();
    let right_width = right_line.width();
    let middle_width = area
        .width
        .saturating_sub(left_width as u16)
        .saturating_sub(right_width as u16);

    // Shortcuts
    let shortcuts: Vec<(String, &str)> = if app.is_welcome {
        vec![
            (app.config.get_keybind("open_file").to_uppercase(), "Open File"),
            (app.config.get_keybind("theme_select").to_uppercase(), "Theme"),
            (app.config.get_keybind("open_help").to_uppercase(), "Docs"),
        ]
    } else if app.is_fuzzy {
        vec![("Enter".to_string(), "Select"), ("Esc".to_string(), "Close")]
    } else if app.focus == Focus::Explorer {
        vec![
            ("Enter".to_string(), "Open"),
            (app.config.get_keybind("new_file").to_uppercase(), "New"),
            ("Shift+O".to_string(), "Options"),
        ]
    } else {
        vec![
            (app.config.get_keybind("save").to_uppercase(), "Save"),
            (app.config.get_keybind("open_file").to_uppercase(), "Open"),
            (app.config.get_keybind("global_search").to_uppercase(), "Search"),
            (
                app.config.get_keybind("command_palette").to_uppercase(),
                "Palette",
            ),
        ]
    };

    let mut final_shortcut_spans = Vec::new();
    let mut current_shortcuts_width = 0;

    for (i, (key, desc)) in shortcuts.iter().enumerate() {
        let icon = app.icon_registry.get_command_icon(desc);
        let key_span = Span::styled(
            format!(" {} {}", icon, key),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        );
        let desc_span = Span::styled(format!(" {} ", desc), Style::default().fg(colors.fg));

        let mut item_width = key_span.width() + desc_span.width();
        if i > 0 {
            item_width += 1; // space
        }

        if current_shortcuts_width + item_width <= middle_width as usize {
            if i > 0 {
                final_shortcut_spans.push(Span::raw(" "));
                current_shortcuts_width += 1;
            }
            current_shortcuts_width += key_span.width();
            final_shortcut_spans.push(key_span);
            current_shortcuts_width += desc_span.width();
            final_shortcut_spans.push(desc_span);
        } else {
            break;
        }
    }

    let middle_line = Line::from(final_shortcut_spans);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(left_width as u16),
            Constraint::Min(0),
            Constraint::Length(right_width as u16),
        ])
        .split(area);

    f.render_widget(Paragraph::new(left_line).bg(colors.surface), chunks[0]);
    f.render_widget(
        Paragraph::new(middle_line)
            .bg(colors.surface)
            .alignment(Alignment::Center),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(right_line)
            .bg(colors.surface)
            .alignment(Alignment::Right),
        chunks[2],
    );
}

fn draw_fuzzy_finder(f: &mut Frame, app: &App, colors: &UIColors) {
    let is_small = matches!(
        app.fuzzy_mode,
        FuzzyMode::SaveAs
            | FuzzyMode::Rename
            | FuzzyMode::DeleteConfirm
            | FuzzyMode::NewFolder
            | FuzzyMode::UnsavedChanges
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
        FuzzyMode::CommandPalette => format!(" 󰘳  {} ", app.i18n.t("command_palette")),
        FuzzyMode::Move => format!(" 󰏫  {} ", app.i18n.t("move_file")),
        FuzzyMode::RunScript => format!(" 󰢱  Run Lua Script "),
        FuzzyMode::EditScript => format!(" 󰝎  Edit Lua Script "),
        FuzzyMode::DeleteScript => format!(" 󰆴  Delete Lua Script "),
        FuzzyMode::DocSelect => format!(" 󰈔  Select Documentation "),
        FuzzyMode::NewFolder => format!(" 󰉋  New Folder Name "),
        FuzzyMode::UnsavedChanges => format!(" 󰆓  {} ", app.i18n.t("unsaved_changes")),
    };

    let block = Block::default()
        .title(
            Line::from(title).style(
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(colors.accent))
        .bg(colors.bg);

    let constraints = if is_small {
        vec![Constraint::Length(1)]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(constraints)
        .split(area);

    let input = if app.fuzzy_mode == FuzzyMode::Move {
        let dir_str = app
            .move_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" 󰉋 {} > ", dir_str),
                Style::default().fg(colors.accent),
            ),
            Span::raw(&app.fuzzy_query),
            Span::styled(
                " (Tab: Move here, Enter: Open folder)",
                Style::default().fg(colors.surface),
            ),
        ]))
    } else if app.fuzzy_mode == FuzzyMode::DeleteConfirm {
        let path_str = app
            .pending_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        Paragraph::new(Line::from(vec![
            Span::styled(" 󰆴 ", Style::default().fg(colors.error)),
            Span::styled(
                "Confirm Delete: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(path_str),
            Span::styled(
                " (Enter: Confirm, Esc: Cancel)",
                Style::default().fg(colors.surface),
            ),
        ]))
    } else if app.fuzzy_mode == FuzzyMode::UnsavedChanges {
        let filename = app.pending_buffer_idx
            .and_then(|idx| app.buffers.get(idx))
            .and_then(|buf| buf.path.as_ref())
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .unwrap_or_else(|| "[No Name]".to_string());
        Paragraph::new(Line::from(vec![
            Span::styled(" 󰆓 ", Style::default().fg(colors.accent)),
            Span::styled(
                format!("Save changes to {}? ", filename),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " (S: Save, D: Discard, Esc: Cancel)",
                Style::default().fg(colors.accent),
            ),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled(" 󰍉 ", Style::default().fg(colors.accent)),
            Span::raw(&app.fuzzy_query),
        ]))
    };
    f.render_widget(input, chunks[0]);

    if !is_small {
        f.render_widget(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(colors.surface)),
            chunks[1],
        );

        let list_height = chunks[2].height as usize;
        let start_idx = app.fuzzy_idx.saturating_sub(list_height / 2);

        let items: Vec<ListItem> = if app.fuzzy_mode == FuzzyMode::Local {
            let safe_start = start_idx.min(app.fuzzy_lines.len().saturating_sub(1));
            let end_idx = (safe_start + list_height).min(app.fuzzy_lines.len());
            if app.fuzzy_lines.is_empty() {
                vec![]
            } else {
                app.fuzzy_lines[safe_start..end_idx]
                    .iter()
                    .enumerate()
                    .map(|(idx, (line_num, text))| {
                        let i = safe_start + idx;
                        let style = if i == app.fuzzy_idx {
                            Style::default()
                                .bg(colors.sel)
                                .fg(colors.accent)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(colors.fg)
                        };
                        ListItem::new(format!(" {}: {}", line_num + 1, text.trim())).style(style)
                    })
                    .collect()
            }
        } else if app.fuzzy_mode == FuzzyMode::Content {
            if !app.fuzzy_results.is_empty() {
                let safe_start = start_idx.min(app.fuzzy_results.len().saturating_sub(1));
                let end_idx = (safe_start + list_height).min(app.fuzzy_results.len());
                let prefer_home = app
                    .fuzzy_query
                    .trim()
                    .strip_prefix('@')
                    .map(|query| query.starts_with('~'))
                    .unwrap_or(false);
                app.fuzzy_results[safe_start..end_idx]
                    .iter()
                    .enumerate()
                    .map(|(idx, path)| {
                        let i = safe_start + idx;
                        let style = if i == app.fuzzy_idx {
                            Style::default()
                                .bg(colors.sel)
                                .fg(colors.accent)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(colors.fg)
                        };
                        let label = app.format_search_dir_for_query(path, prefer_home);
                        ListItem::new(format!(" {} {}/", "󰉋", label.trim_end_matches('/')))
                            .style(style)
                    })
                    .collect()
            } else {
                let safe_start = start_idx.min(app.fuzzy_global_results.len().saturating_sub(1));
                let end_idx = (safe_start + list_height).min(app.fuzzy_global_results.len());
                if app.fuzzy_global_results.is_empty() {
                    vec![]
                } else {
                    app.fuzzy_global_results[safe_start..end_idx]
                        .iter()
                        .enumerate()
                        .map(|(idx, (path, line_num, text))| {
                            let i = safe_start + idx;
                            let name = path.file_name().unwrap_or_default().to_string_lossy();
                            let style = if i == app.fuzzy_idx {
                                Style::default()
                                    .bg(colors.sel)
                                    .fg(colors.accent)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(colors.fg)
                            };
                            ListItem::new(format!(" {} (L{}): {}", name, line_num + 1, text.trim()))
                                .style(style)
                        })
                        .collect()
                }
            }
        } else if app.fuzzy_mode == FuzzyMode::Themes {
            let safe_start = start_idx.min(app.fuzzy_themes.len().saturating_sub(1));
            let end_idx = (safe_start + list_height).min(app.fuzzy_themes.len());
            if app.fuzzy_themes.is_empty() {
                vec![]
            } else {
                app.fuzzy_themes[safe_start..end_idx]
                    .iter()
                    .enumerate()
                    .map(|(idx, theme_name)| {
                        let i = safe_start + idx;
                        let style = if i == app.fuzzy_idx {
                            Style::default()
                                .bg(colors.sel)
                                .fg(colors.accent)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(colors.fg)
                        };
                        let indicator = if theme_name == &app.current_theme {
                            "󰄬 "
                        } else {
                            "  "
                        };
                        ListItem::new(format!(" {} {}", indicator, theme_name)).style(style)
                    })
                    .collect()
            }
        } else if matches!(
            app.fuzzy_mode,
            FuzzyMode::CommandPalette
                | FuzzyMode::FileOptions
                | FuzzyMode::RunScript
                | FuzzyMode::EditScript
                | FuzzyMode::DeleteScript
                | FuzzyMode::DocSelect
        ) {
            if app.fuzzy_results.is_empty() {
                vec![]
            } else {
                let safe_start = start_idx.min(app.fuzzy_results.len().saturating_sub(1));
                let end_idx = (safe_start + list_height).min(app.fuzzy_results.len());
                app.fuzzy_results[safe_start..end_idx]
                    .iter()
                    .enumerate()
                    .map(|(idx, path)| {
                        let i = safe_start + idx;
                        let name = if matches!(
                            app.fuzzy_mode,
                            FuzzyMode::RunScript | FuzzyMode::EditScript | FuzzyMode::DeleteScript
                        ) {
                            let stem = path
                                .file_stem()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            if let Ok(content) = std::fs::read_to_string(path) {
                                if let Some(first) = content.lines().next() {
                                    let trimmed = first.trim();
                                    if trimmed.starts_with("-- ") {
                                        trimmed[3..].trim().to_string()
                                    } else {
                                        stem
                                    }
                                } else {
                                    stem
                                }
                            } else {
                                stem
                            }
                        } else {
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        };
                        let style = if i == app.fuzzy_idx {
                            Style::default()
                                .bg(colors.sel)
                                .fg(colors.accent)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(colors.fg)
                        };
                        let icon = match app.fuzzy_mode {
                            FuzzyMode::CommandPalette
                            | FuzzyMode::FileOptions => app.icon_registry.get_command_icon(&name),
                            FuzzyMode::RunScript => "󰢱 ",
                            FuzzyMode::EditScript => "󰏫 ",
                            FuzzyMode::DeleteScript => "󰆴 ",
                            FuzzyMode::DocSelect => app.icon_registry.get_icon(path, false, false),
                            _ => "  ",
                        };
                        ListItem::new(format!(" {} {}", icon, name)).style(style)
                    })
                    .collect()
            }
        } else if app.fuzzy_results.is_empty() {
            vec![]
        } else {
            let safe_start = start_idx.min(app.fuzzy_results.len().saturating_sub(1));
            let end_idx = (safe_start + list_height).min(app.fuzzy_results.len());
            app.fuzzy_results[safe_start..end_idx]
                .iter()
                .enumerate()
                .map(|(idx, path)| {
                    let i = safe_start + idx;
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let rel_path = path
                        .strip_prefix(&app.explorer.root)
                        .unwrap_or(path)
                        .to_string_lossy();
                    let style = if i == app.fuzzy_idx {
                        Style::default()
                            .bg(colors.sel)
                            .fg(colors.accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    let icon = app.icon_registry.get_icon(path, path.is_dir(), false);
                    ListItem::new(format!(" {} {} ({})", icon, name, rel_path)).style(style)
                })
                .collect()
        };

        f.render_widget(List::new(items), chunks[2]);
    }

    f.render_widget(block, area);
}

#[cfg(test)]
mod indent_guide_tests {
    use super::active_indent_guide_scope;

    #[test]
    fn active_scope_uses_parent_guide_inside_nested_block() {
        // def (0) -> if (4) -> bar (8) -> baz (4)
        let indents = [0, 4, 8, 4];
        let line_indent = |row: usize| indents[row];
        let (level, start, end) = active_indent_guide_scope(4, line_indent, 2, 4);
        assert_eq!(level, 1);
        assert_eq!(start, 1);
        assert_eq!(end, 3);
    }

    #[test]
    fn active_scope_expands_for_same_indent_siblings() {
        let indents = [0, 4, 4, 4];
        let line_indent = |row: usize| indents[row];
        let (level, start, end) = active_indent_guide_scope(4, line_indent, 2, 4);
        assert_eq!(level, 1);
        assert_eq!(start, 1);
        assert_eq!(end, 3);
    }

    #[test]
    fn active_scope_uses_line_indent_not_cursor_column() {
        let indents = [0, 4, 8, 8];
        let line_indent = |row: usize| indents[row];
        let (level, start, end) = active_indent_guide_scope(4, line_indent, 3, 4);
        assert_eq!(level, 1);
        assert_eq!(start, 1);
        assert_eq!(end, 3);
    }
}
