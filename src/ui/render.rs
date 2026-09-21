use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use std::path::Path;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus, FuzzyMode, ModalAction, ModalButtonHitbox, TabHitbox};
use crate::buffer::{column::TAB_WIDTH, EditorBuffer};

use super::welcome::draw_welcome_screen;
use super::{get_colors, UIColors};

fn syntect_foreground_or(fg: syntect::highlighting::Color, fallback: Color) -> Color {
    if fg.a == 0 || (fg.r == 0 && fg.g == 0 && fg.b == 0) {
        fallback
    } else {
        Color::Rgb(fg.r, fg.g, fg.b)
    }
}

struct MarkdownPalette {
    heading: Color,
    code_span: Color,
    emphasis: Color,
    strong: Color,
    url: Color,
    bullet: Color,
}

impl MarkdownPalette {
    fn build(theme: &syntect::highlighting::Theme, fallback: Color) -> Self {
        let highlighter = Highlighter::new(theme);
        let scope_color = |scope: &str| -> Color {
            match Scope::new(scope) {
                Ok(scope) => {
                    let mut stack = ScopeStack::new();
                    stack.push(scope);
                    syntect_foreground_or(
                        highlighter.style_for_stack(stack.as_slice()).foreground,
                        fallback,
                    )
                }
                Err(_) => fallback,
            }
        };
        Self {
            heading: scope_color("entity.name.function"),
            code_span: scope_color("string"),
            emphasis: scope_color("variable.parameter"),
            strong: scope_color("keyword.control"),
            url: scope_color("constant.character.escape"),
            bullet: scope_color("keyword.operator"),
        }
    }

    fn color_for(&self, capture: &str) -> Color {
        match capture {
            "text.title" => self.heading,
            "text.literal" => self.code_span,
            "text.emphasis" => self.emphasis,
            "text.strong" => self.strong,
            "text.uri" | "text.reference" | "string.escape" => self.url,
            "punctuation.special" | "punctuation.delimiter" => self.bullet,
            _ => self.heading,
        }
    }
}

/// Build colored spans for one markdown line from whole-document tree-sitter
/// capture ranges. Later overlapping ranges win over earlier ones.
fn markdown_line_spans(
    md_ranges: &[(usize, usize, &'static str)],
    line_start: usize,
    line_text: &str,
    palette: &MarkdownPalette,
    fallback: Color,
) -> Vec<(Color, String)> {
    let line_end = line_start + line_text.len();
    let mut char_colors: Vec<Color> = line_text.chars().map(|_| fallback).collect();

    for &(range_start, range_end, capture) in md_ranges {
        if range_end <= line_start || range_start >= line_end {
            continue;
        }
        let clipped_start = range_start.max(line_start);
        let clipped_end = range_end.min(line_end);
        let rel_start = clipped_start - line_start;
        let rel_end = clipped_end - line_start;
        let start_idx = line_text
            .char_indices()
            .take_while(|(offset, _)| *offset < rel_start)
            .count();
        let end_idx = line_text
            .char_indices()
            .take_while(|(offset, _)| *offset < rel_end)
            .count();
        let color = palette.color_for(capture);
        for slot in char_colors.iter_mut().skip(start_idx).take(end_idx - start_idx) {
            *slot = color;
        }
    }

    let mut spans = Vec::new();
    let mut current_color = char_colors.first().copied().unwrap_or(fallback);
    let mut current_text = String::new();
    for (c, color) in line_text.chars().zip(char_colors) {
        if color != current_color && !current_text.is_empty() {
            spans.push((current_color, std::mem::take(&mut current_text)));
            current_color = color;
        }
        current_text.push(c);
    }
    if !current_text.is_empty() {
        spans.push((current_color, current_text));
    }
    spans
}

pub const DEFAULT_EXPLORER_WIDTH_PERCENT: u16 = 20;

/// Truncates string `s` so that it occupies at most `max_chars` characters,
/// appending `...` if truncated.
pub fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        s.to_string()
    } else if max_chars <= 3 {
        s.chars().take(max_chars).collect()
    } else {
        let keep = max_chars.saturating_sub(3);
        let prefix: String = s.chars().take(keep).collect();
        format!("{prefix}...")
    }
}

/// Calculates the horizontal marquee offset for a long filename being viewed/selected.
///
/// 1. Pauses at offset 0 for `pause_start_ms` (1.0s) so the start is immediately legible.
/// 2. Rolls one character every `step_ms` (220ms) until reaching the end of the text.
/// 3. Pauses at the end (`max_offset`) for `pause_end_ms` (1.2s) so the extension/tail is legible.
/// 4. Restarts smoothly from the beginning.
pub fn calculate_marquee_offset(
    name_len: usize,
    available_width: usize,
    elapsed: std::time::Duration,
) -> (usize, bool) {
    if name_len <= available_width || available_width == 0 {
        return (0, false);
    }

    let max_offset = name_len.saturating_sub(available_width);
    let step_ms = 220;
    let pause_start_ms = 1000;
    let pause_end_ms = 1200;
    let scroll_duration_ms = max_offset as u128 * step_ms;
    let total_cycle_ms = pause_start_ms + scroll_duration_ms + pause_end_ms;

    let cycle_pos = elapsed.as_millis() % total_cycle_ms;

    let offset = if cycle_pos < pause_start_ms {
        0
    } else if cycle_pos < pause_start_ms + scroll_duration_ms {
        ((cycle_pos - pause_start_ms) / step_ms) as usize
    } else {
        max_offset
    };

    (offset.min(max_offset), true)
}

/// Formats an explorer filename with horizontal marquee scrolling when selected,
/// or truncates with an ellipsis (`…`) when not selected.
pub fn format_explorer_name(
    name: &str,
    available_width: usize,
    is_selected: bool,
    elapsed: std::time::Duration,
) -> (String, bool) {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= available_width || available_width == 0 {
        return (name.to_string(), false);
    }

    if is_selected {
        let (offset, animating) = calculate_marquee_offset(chars.len(), available_width, elapsed);
        let visible: String = chars.iter().skip(offset).take(available_width).collect();
        (visible, animating)
    } else {
        let keep = available_width.saturating_sub(1);
        let mut s: String = chars.iter().take(keep).collect();
        s.push('…');
        (s, false)
    }
}

pub fn render(f: &mut Frame, app: &mut App) {
    let colors = get_colors(app);

    // Remember the full frame for overlay hit-testing (toast close button).
    app.screen_area = f.area();

    f.render_widget(Block::default().bg(colors.bg), f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    // Explorer sidebar has a consistent standard width (20%) so large file names
    // never distort the panel layout or squeeze the editor pane.
    let explorer_width = if app.show_explorer {
        DEFAULT_EXPLORER_WIDTH_PERCENT
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
        let target = app
            .buffers
            .get(target_idx)
            .map(|buffer| (buffer.path.clone(), buffer.is_large_file || buffer.is_loading));
        let script = app
            .buffers
            .get(script_idx)
            .map(|buffer| (buffer.path.clone(), buffer.is_large_file || buffer.is_loading));
        if let Some((path, false)) = target {
            app.ensure_syntax_for_path_loading(path.as_deref());
        }
        if let Some((path, false)) = script {
            app.ensure_syntax_for_path_loading(path.as_deref());
        }

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

            let target_body_area = editor_body_area(app, target_area, target_idx);
            let script_body_area = editor_body_area(app, script_area, script_idx);

            if let Some(target_buf) = app.buffers.get_mut(target_idx) {
                let width = target_body_area.width as usize;
                target_buf.move_cursor(0, 0, width);
            }

            if let Some(script_buf) = app.buffers.get_mut(script_idx) {
                let width = script_body_area.width as usize;
                script_buf.move_cursor(0, 0, width);
            }

            draw_editor_pane(
                f,
                app,
                target_area,
                target_idx,
                app.current_buffer_idx == target_idx,
                &colors,
            );
            draw_split_separator(f, separator_area, &colors);
            draw_editor_pane(
                f,
                app,
                script_area,
                script_idx,
                app.current_buffer_idx == script_idx,
                &colors,
            );
        } else if !app.buffers.is_empty() {
            draw_editor_pane(
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
        let current = app
            .buffers
            .get(app.current_buffer_idx)
            .map(|buffer| (buffer.path.clone(), buffer.is_large_file || buffer.is_loading));
        if let Some((path, false)) = current {
            app.ensure_syntax_for_path_loading(path.as_deref());
        }

        {
            let body_area = editor_body_area(app, editor_chunks[1], app.current_buffer_idx);
            let buffer = &mut app.buffers[app.current_buffer_idx];
            let width = body_area.width as usize;
            buffer.move_cursor(0, 0, width);
        }

        draw_editor_pane(
            f,
            app,
            editor_chunks[1],
            app.current_buffer_idx,
            true,
            &colors,
        );
    }

    draw_status_bar(f, app, chunks[1], &colors);

    if app.is_fuzzy {
        draw_fuzzy_finder(f, app, &colors);
    }

    if app.context_menu.is_some() {
        draw_context_menu(f, app, &colors);
    }

    // Toasts float above everything, including modals.
    draw_toasts(f, app, &colors);
}

fn draw_split_separator(f: &mut Frame, area: Rect, colors: &UIColors) {
    f.render_widget(Block::default().bg(colors.surface), area);
}

fn should_draw_header_status_bar(app: &App, buffer_idx: usize) -> bool {
    if app.is_welcome || app.buffers.is_empty() {
        return false;
    }

    if let Some(buf) = app.buffers.get(buffer_idx) {
        if buf.is_loading {
            return false;
        }
    }

    !app.live_script_mode && buffer_idx < app.buffers.len()
}

fn split_header_area(area: Rect, show_header: bool) -> (Option<Rect>, Rect) {
    if !show_header || area.height == 0 {
        return (None, area);
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    (Some(chunks[0]), chunks[1])
}

fn editor_body_area(app: &App, area: Rect, buffer_idx: usize) -> Rect {
    let (_, body_area) = split_header_area(area, should_draw_header_status_bar(app, buffer_idx));
    body_area
}

fn draw_editor_pane(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    buffer_idx: usize,
    is_focused: bool,
    colors: &UIColors,
) {
    let (header_area, body_area) =
        split_header_area(area, should_draw_header_status_bar(app, buffer_idx));

    if is_focused {
        app.editor_area = body_area;
    }

    if let Some(header_area) = header_area {
        draw_header_status_bar(f, app, header_area, buffer_idx, colors);
    }

    draw_editor(f, app, body_area, buffer_idx, is_focused, colors);
}

fn draw_header_status_bar(
    f: &mut Frame,
    app: &mut App,
    area: Rect,
    buffer_idx: usize,
    colors: &UIColors,
) {
    let Some(buffer) = app.buffers.get_mut(buffer_idx) else {
        return;
    };

    let metrics = if buffer.is_preview {
        format!("[PREVIEW] {}", file_metrics(buffer))
    } else {
        file_metrics(buffer)
    };

    let right_line = Line::from(vec![Span::styled(
        format!(" {} ", metrics),
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD),
    )]);

    let right_width = right_line.width().min(area.width as usize) as u16;
    let left_chunk = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(right_width)])
        .split(area);

    f.render_widget(Paragraph::new(Line::from(Vec::<Span>::new())).bg(colors.surface), left_chunk[0]);
    f.render_widget(
        Paragraph::new(right_line)
            .bg(colors.surface)
            .alignment(Alignment::Right),
        left_chunk[1],
    );
}

fn draw_tab_bar(f: &mut Frame, app: &mut App, area: Rect, colors: &UIColors) {
    app.tab_hitboxes.clear();
    if app.buffers.is_empty() {
        return;
    }

    let is_bar_hovered = app.mouse_pos.map(|(_, my)| my == area.y).unwrap_or(false);
    let mouse_x = app.mouse_pos.map(|(mx, _)| mx).unwrap_or(0);

    let mut spans = Vec::new();
    let mut cur_x = area.x;
    let max_x = area.x.saturating_add(area.width);

    for (i, buffer) in app.buffers.iter().enumerate() {
        let is_live_script = Some(i) == app.live_script_buffer_idx;
        let is_preview = buffer.is_preview;
        if is_live_script || is_preview {
            continue;
        }

        if cur_x >= max_x {
            break;
        }

        let full_name = buffer
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "[No Name]".to_string());
        let name = truncate_with_ellipsis(&full_name, 20);

        let is_current = i == app.current_buffer_idx;
        let tab_style = if is_current {
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

        let mod_str = if buffer.modified { " *" } else { "" };
        let tab_content = format!(" {icon}{name}{mod_str} ");
        let tab_content_len = UnicodeWidthStr::width(tab_content.as_str()) as u16;

        let close_btn_w = 3u16;
        let is_tab_hovered = is_bar_hovered
            && mouse_x >= cur_x
            && mouse_x < cur_x + tab_content_len + close_btn_w;

        if is_tab_hovered {
            let close_start_x = cur_x + tab_content_len;
            let close_end_x = close_start_x + close_btn_w;
            let is_close_hovered = mouse_x >= close_start_x && mouse_x < close_end_x;

            let close_style = if is_close_hovered {
                Style::default()
                    .fg(colors.error)
                    .add_modifier(Modifier::BOLD)
                    .bg(if is_current { colors.sel } else { colors.bg })
            } else {
                Style::default()
                    .fg(colors.text_muted())
                    .bg(if is_current { colors.sel } else { colors.bg })
            };

            spans.push(Span::styled(tab_content, tab_style));
            spans.push(Span::styled("󰅖 ", close_style));

            let total_w = tab_content_len + close_btn_w;
            app.tab_hitboxes.push(TabHitbox {
                buffer_idx: i,
                y: area.y,
                tab_start_x: cur_x,
                tab_end_x: (cur_x + total_w).min(max_x),
                close_start_x: Some(close_start_x),
                close_end_x: Some(close_end_x),
            });
            cur_x += total_w;
        } else {
            spans.push(Span::styled(tab_content, tab_style));
            let total_w = tab_content_len;
            app.tab_hitboxes.push(TabHitbox {
                buffer_idx: i,
                y: area.y,
                tab_start_x: cur_x,
                tab_end_x: (cur_x + total_w).min(max_x),
                close_start_x: None,
                close_end_x: None,
            });
            cur_x += total_w;
        }

        spans.push(Span::raw(" "));
        cur_x += 1;
    }

    f.render_widget(Paragraph::new(Line::from(spans)).bg(colors.bg), area);
}

/// Always-visible explorer search bar (under the explorer title).
/// Renders a bordered rounded input box with a search icon and horizontal scrolling
/// to prevent text overflow and cursor clipping.
fn draw_explorer_search_bar(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let is_focused = app.focus == Focus::Explorer;
    let border_color = colors.accent;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .bg(colors.bg);
    f.render_widget(block.clone(), area);

    let inner = block.inner(area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let icon = "󰍉 ";
    let icon_color = colors.accent;
    let icon_span = Span::styled(icon, Style::default().fg(icon_color));
    let icon_width = UnicodeWidthStr::width(icon);

    if inner.width <= icon_width as u16 {
        f.render_widget(Paragraph::new(Line::from(vec![icon_span])).bg(colors.bg), inner);
        if is_focused {
            f.set_cursor_position((inner.x, inner.y));
        }
        return;
    }

    let avail_width = (inner.width as usize).saturating_sub(icon_width);
    let query = &app.explorer.search_input.text;
    let cursor_char = app.explorer.search_input.cursor;
    let cursor_byte = app.explorer.search_input.byte_idx(cursor_char);
    let cursor_col = UnicodeWidthStr::width(&query[..cursor_byte]);
    let total_query_width = UnicodeWidthStr::width(query.as_str());

    let current_scroll = app.explorer.search_hscroll.get();
    let scroll = crate::explorer::follow_horizontal_scroll(
        cursor_col,
        current_scroll,
        total_query_width,
        avail_width,
    );
    app.explorer.search_hscroll.set(scroll);

    let mut spans = vec![icon_span];
    if query.is_empty() {
        if !is_focused && avail_width >= 9 {
            spans.push(Span::styled("Search...", Style::default().fg(colors.fg)));
        }
    } else {
        let sel_range = app.explorer.search_input.selection_range();
        let window_start = scroll;
        let window_end = scroll + avail_width;

        let mut current_col = 0;
        let mut current_style: Option<Style> = None;
        let mut current_text = String::new();

        for (char_idx, ch) in query.chars().enumerate() {
            let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            let ch_start = current_col;
            let ch_end = current_col + ch_width;
            current_col = ch_end;

            if ch_end <= window_start {
                continue;
            }
            if ch_start >= window_end {
                break;
            }

            let is_sel = match sel_range {
                Some((s, e)) => char_idx >= s && char_idx < e,
                None => false,
            };
            let style = if is_sel {
                Style::default().bg(colors.sel).fg(colors.accent)
            } else {
                Style::default().fg(colors.fg)
            };

            if current_style != Some(style) {
                if !current_text.is_empty() {
                    if let Some(st) = current_style {
                        spans.push(Span::styled(std::mem::take(&mut current_text), st));
                    }
                }
                current_style = Some(style);
            }

            if ch_start < window_start {
                for _ in 0..ch_end.saturating_sub(window_start) {
                    current_text.push(' ');
                }
            } else if ch_end > window_end {
                for _ in 0..window_end.saturating_sub(ch_start) {
                    current_text.push(' ');
                }
            } else {
                current_text.push(ch);
            }
        }
        if !current_text.is_empty() {
            if let Some(st) = current_style {
                spans.push(Span::styled(current_text, st));
            }
        }
    }

    f.render_widget(Paragraph::new(Line::from(spans)).bg(colors.bg), inner);

    if is_focused {
        let cursor_rel = cursor_col.saturating_sub(scroll);
        let cursor_x = inner
            .x
            .saturating_add(icon_width as u16)
            .saturating_add(cursor_rel as u16);
        let max_cursor_x = inner.x.saturating_add(inner.width.saturating_sub(1));
        f.set_cursor_position((cursor_x.min(max_cursor_x), inner.y));
    }
}

fn draw_explorer(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    if area.height == 0 || area.width == 0 {
        return;
    }

    let border_color = if app.focus == Focus::Explorer {
        colors.accent
    } else {
        colors.surface
    };
    let outer_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color));
    f.render_widget(outer_block.clone(), area);

    let inner = outer_block.inner(area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }

    // Row 0: Explorer title with match counter when searching.
    let title_style = if app.focus == Focus::Explorer {
        Style::default()
            .fg(colors.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(colors.fg)
            .add_modifier(Modifier::BOLD)
    };
    let title_spans = if app.explorer.is_searching() {
        vec![
            Span::styled(format!(" {}", app.i18n.t("explorer")), title_style),
            Span::styled(
                format!(" ({})", app.explorer.search_results.len()),
                Style::default()
                    .fg(colors.accent)
                    .add_modifier(Modifier::BOLD),
            ),
        ]
    } else {
        vec![Span::styled(
            format!(" {}", app.i18n.t("explorer")),
            title_style,
        )]
    };
    let title_area = Rect::new(inner.x, inner.y, inner.width, 1);
    f.render_widget(
        Paragraph::new(Line::from(title_spans)).bg(colors.bg),
        title_area,
    );

    // Rows 1..=3: Bordered search bar under the title.
    let (box_x, box_width) = if inner.width > 4 {
        (inner.x.saturating_add(1), inner.width.saturating_sub(2))
    } else {
        (inner.x, inner.width)
    };
    let bar_height = 3.min(inner.height.saturating_sub(1));
    if bar_height > 0 {
        let bar_area = Rect::new(box_x, inner.y.saturating_add(1), box_width, bar_height);
        draw_explorer_search_bar(f, app, bar_area, colors);
    }

    // Row 4+: Tree or search matches.
    let searching = app.explorer.is_searching();
    let list_height = crate::explorer::explorer_list_height(area.height);
    let (len, selected, scroll) = if searching {
        (
            app.explorer.search_results.len(),
            app.explorer.search_selected,
            app.explorer.search_scroll,
        )
    } else {
        (
            app.explorer.items.len(),
            app.explorer.selected_idx,
            app.explorer.scroll_offset,
        )
    };
    let scroll = scroll.min(len.saturating_sub(1));
    let selected = selected.min(len.saturating_sub(1));
    let end = (scroll + list_height).min(len);

    let mut marquee_animating = false;
    let marquee_elapsed = if len > 0 {
        let sel_item = if searching {
            &app.explorer.search_results[selected]
        } else {
            &app.explorer.items[selected]
        };
        let mut marquee = app.explorer.marquee_state.borrow_mut();
        if marquee.0.as_ref() != Some(&sel_item.path) {
            *marquee = (Some(sel_item.path.clone()), std::time::Instant::now());
        }
        marquee.1.elapsed()
    } else {
        std::time::Duration::ZERO
    };

    let mut items: Vec<ListItem> = Vec::new();
    if searching && len == 0 {
        items.push(ListItem::new(" No matches").style(Style::default().fg(colors.text_muted())));
    }
    for actual_idx in scroll..end {
        let item = if searching {
            &app.explorer.search_results[actual_idx]
        } else {
            &app.explorer.items[actual_idx]
        };
        let is_selected = actual_idx == selected;
        let style = if is_selected {
            Style::default()
                .bg(colors.sel)
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg)
        };
        if searching {
            let icon = app
                .icon_registry
                .get_icon(&item.path, item.is_dir, item.expanded);
            let parent = item
                .path
                .parent()
                .and_then(|p| p.strip_prefix(&app.explorer.root).ok())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let prefix = format!(" {icon}");
            let prefix_w = UnicodeWidthStr::width(prefix.as_str());
            let parent_suffix = if !parent.is_empty() {
                format!("  {parent}")
            } else {
                String::new()
            };
            let parent_w = UnicodeWidthStr::width(parent_suffix.as_str());

            // Reserve space for parent path if it fits comfortably alongside the name,
            // otherwise prioritize displaying the file name cleanly.
            let avail_w = if !parent.is_empty() && (inner.width as usize) > prefix_w + parent_w + 6 {
                (inner.width as usize).saturating_sub(prefix_w + parent_w)
            } else {
                (inner.width as usize).saturating_sub(prefix_w)
            };

            let (name_display, animating) = format_explorer_name(
                &item.name,
                avail_w.max(1),
                is_selected,
                marquee_elapsed,
            );
            if is_selected && animating {
                marquee_animating = true;
            }

            let mut spans = vec![Span::raw(format!("{prefix}{name_display}"))];
            if !parent.is_empty()
                && prefix_w + UnicodeWidthStr::width(name_display.as_str()) + parent_w
                    <= inner.width as usize
            {
                spans.push(Span::styled(
                    parent_suffix,
                    Style::default().fg(colors.text_muted()),
                ));
            }
            items.push(ListItem::new(Line::from(spans)).style(style));
        } else {
            let indent = "  ".repeat(item.depth);
            let icon = app
                .icon_registry
                .get_icon(&item.path, item.is_dir, item.expanded);
            let arrow = if item.is_dir && !item.expanded {
                "›"
            } else if item.is_dir {
                "⌄"
            } else {
                ""
            };
            let prefix = format!(" {indent}{icon}");
            let arrow_suffix = if arrow.is_empty() {
                String::new()
            } else {
                format!(" {arrow}")
            };
            let prefix_w = UnicodeWidthStr::width(prefix.as_str());
            let arrow_w = UnicodeWidthStr::width(arrow_suffix.as_str());
            let avail_w = (inner.width as usize).saturating_sub(prefix_w + arrow_w);

            let (name_display, animating) = format_explorer_name(
                &item.name,
                avail_w.max(1),
                is_selected,
                marquee_elapsed,
            );
            if is_selected && animating {
                marquee_animating = true;
            }

            items.push(
                ListItem::new(format!("{prefix}{name_display}{arrow_suffix}")).style(style),
            );
        }
    }

    app.explorer.has_active_marquee.set(marquee_animating);

    if list_height > 0 {
        let list_top = inner.y.saturating_add(4);
        let list_area = Rect::new(inner.x, list_top, inner.width, list_height as u16);
        f.render_widget(List::new(items).bg(colors.bg), list_area);
    }
}

fn visual_leading_indent_chars<I: IntoIterator<Item = char>>(chars: I) -> usize {
    let mut col = 0;
    for c in chars {
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
/// Pick the syntect grammar for a buffer: an explicit `syntax_override` wins
/// (used by pathless buffers like the live script pane), then the file
/// extension, then plain text. Unknown overrides fall through gracefully.
fn resolve_buffer_syntax<'a>(
    syntax_set: &'a SyntaxSet,
    buffer: &EditorBuffer,
) -> &'a SyntaxReference {
    buffer
        .syntax_override
        .as_deref()
        .and_then(|ext| syntax_set.find_syntax_by_extension(ext))
        .or_else(|| {
            buffer
                .path
                .as_ref()
                .and_then(|p| p.extension())
                .and_then(|e| e.to_str())
                .and_then(|ext| syntax_set.find_syntax_by_extension(ext))
        })
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text())
}

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

fn file_metrics(buffer: &mut EditorBuffer) -> String {
    if buffer.is_loading {
        return String::new();
    }

    let lines = buffer.content.len_lines();
    if buffer.is_large_file {
        return format!("[Lines: {} | Cols: —]", lines);
    }

    if buffer.max_visual_width.is_none() {
        buffer.update_max_visual_width();
    }
    let cols = buffer.max_visual_width.unwrap();
    format!("[Lines: {} | Cols: {}]", lines, cols)
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

    if buffer.is_loading {
        f.render_widget(Block::default().bg(colors.bg), area);
        if area.height > 0 && area.width > 0 {
            let center_y = area.y + area.height / 2;
            let loading_area = Rect {
                x: area.x,
                y: center_y,
                width: area.width,
                height: 1,
            };
            let loading_widget = Paragraph::new(Line::from(vec![
                Span::styled(
                    "[Loading]",
                    Style::default()
                        .fg(colors.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .alignment(Alignment::Center);
            f.render_widget(loading_widget, loading_area);
        }
        return;
    }

    let matching_bracket = if app.config.highlight_matching_bracket {
        if buffer.is_large_file {
            buffer.find_matching_bracket_with_limit(Some(8_192))
        } else {
            buffer.find_matching_bracket()
        }
    } else {
        None
    };

    let mut buffer_scroll_row = buffer.scroll_row;
    let height = area.height as usize;
    let max_scroll = buffer.content.len_lines().saturating_sub(1);
    buffer_scroll_row = buffer_scroll_row.min(max_scroll);

    let (theme, syntax_set) = {
        let theme = app
            .theme_set
            .themes
            .get(&app.current_theme)
            .or_else(|| app.theme_set.themes.get("NEdit Dark Complete"))
            .or_else(|| app.theme_set.themes.get("NEdit Dark"))
            .or_else(|| app.theme_set.themes.values().next())
            .expect("No themes loaded — check your theme directory");
        (theme, app.syntax_set.as_ref())
    };

    let buffer = match app.buffers.get_mut(buffer_idx) {
        Some(b) => b,
        None => return,
    };

    let line_count = buffer.content.len_lines();
    // Syntect's built-in Markdown grammar is pathologically slow (~100x slower
    // than other languages due to backtracking regexes), so Markdown files are
    // highlighted with tree-sitter instead. Every other extension keeps using
    // syntect.
    let is_markdown = buffer
        .path
        .as_ref()
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
        .unwrap_or(false);
    let mut syntax_highlighter = syntax_set
        .filter(|_| !buffer.is_large_file && !buffer.is_loading && !is_markdown)
        .map(|syntax_set| {
        let syntax = resolve_buffer_syntax(syntax_set, buffer);

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

    let md_highlight = if is_markdown && !buffer.is_large_file && !buffer.is_loading {
        let visible: std::ops::Range<usize> =
            buffer_scroll_row..(buffer_scroll_row + height).min(line_count);
        let needs_highlight = visible.clone().any(|row| {
            matches!(buffer.rendered_spans.get(row), Some(None)) || row >= buffer.rendered_spans.len()
        });
        if needs_highlight {
            let doc = buffer.content.to_string();
            let mut ranges = super::markdown::highlight_ranges(&doc);
            ranges.sort_by_key(|(start, _, _)| *start);
            let palette = MarkdownPalette::build(theme, colors.fg);
            Some((ranges, palette))
        } else {
            None
        }
    } else {
        None
    };

    let mut lines = Vec::new();
    let visible_width = area.width.saturating_sub(5) as usize;
    let (active_indent_level, active_scope_start, active_scope_end) = if is_focused && !buffer.is_large_file {
        let line_indent = |row: usize| visual_leading_indent_chars(buffer.content.line(row).chars());
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
    }).or_else(|| buffer.get_word_at_cursor());
    let selected_match_chars = selected_match_text
        .as_ref()
        .map(|text| text.chars().collect::<Vec<_>>());

    let mut line_chars = Vec::new();
    for i in buffer_scroll_row..(buffer_scroll_row + height).min(line_count) {
        let original_line = buffer.content.line(i).to_string();
        let line_content = original_line.trim_end_matches(&['\r', '\n'][..]);

        let mut match_ranges = Vec::new();
        if let Some(word_chars) = selected_match_chars.as_ref() {
            if !word_chars.is_empty() {
                line_chars.clear();
                line_chars.extend(line_content.chars());
                if line_chars.len() >= word_chars.len() {
                    for idx in 0..=(line_chars.len() - word_chars.len()) {
                        if line_chars[idx..idx + word_chars.len()] == word_chars[..] {
                            match_ranges.push(idx..idx + word_chars.len());
                        }
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

        let ranges: Vec<(Color, String)> =
            if let Some(cached) = buffer.rendered_spans.get(i).and_then(|c| c.as_ref()) {
                cached.clone()
            } else if let (Some((ref highlighter, ref mut ps, ref mut hs)), Some(syntax_set)) =
                (syntax_highlighter.as_mut(), syntax_set)
            {
                let ops = ps.parse_line(&original_line, syntax_set).unwrap_or_default();
                if i < buffer.syntax_states.len() {
                    buffer.syntax_states[i] = Some((ps.clone(), hs.clone()));
                }
                let result: Vec<(Color, String)> =
                    HighlightIterator::new(hs, &ops, &original_line, highlighter)
                        .map(|(s, text)| {
                            (
                                syntect_foreground_or(s.foreground, colors.fg),
                                text.to_string(),
                            )
                        })
                        .collect();
                if i < buffer.rendered_spans.len() {
                    buffer.rendered_spans[i] = Some(result.clone());
                }
                result
            } else if let (Some((ref md_ranges, ref palette)), true) =
                (md_highlight.as_ref(), is_markdown)
            {
                let line_start = buffer.content.line_to_byte(i);
                let result = markdown_line_spans(md_ranges, line_start, &original_line, palette, colors.fg);
                if i < buffer.rendered_spans.len() {
                    buffer.rendered_spans[i] = Some(result.clone());
                }
                result
            } else {
                let result = vec![(colors.fg, line_content.to_string())];
                if i < buffer.rendered_spans.len() {
                    buffer.rendered_spans[i] = Some(result.clone());
                }
                result
            };
        let mut char_offset = 0;
        let mut visual_col = 0;
        let mut in_leading = show_guides;

        for (fg, text) in &ranges {
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
                        let mut style = Style::default().fg(*fg);

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

        if buffer.show_autocomplete_list && buffer.autocomplete_options.len() > 1 {
            draw_autocomplete_popup(f, buffer, area, colors, cursor_x, cursor_y);
        }
    }
}

fn draw_autocomplete_popup(
    f: &mut Frame,
    buffer: &EditorBuffer,
    area: Rect,
    colors: &UIColors,
    cursor_x: u16,
    cursor_y: u16,
) {
    if buffer.autocomplete_options.len() <= 1 {
        return;
    }

    let prefix = buffer.get_current_word_prefix();
    let prefix_len = UnicodeWidthStr::width(prefix.as_str()) as u16;

    let max_visible = 5;
    let total = buffer.autocomplete_options.len();
    let visible_count = total.min(max_visible);

    let max_word_len = buffer
        .autocomplete_options
        .iter()
        .map(|w| UnicodeWidthStr::width(w.as_str()))
        .max()
        .unwrap_or(8);

    let popup_width = ((max_word_len + 6) as u16).max(14).min(area.width.saturating_sub(2));
    let popup_height = (visible_count as u16) + 2;

    let ideal_x = cursor_x.saturating_sub(prefix_len);
    let popup_x = if ideal_x + popup_width > area.x + area.width {
        (area.x + area.width).saturating_sub(popup_width)
    } else {
        ideal_x.max(area.x)
    };

    let popup_y = if cursor_y + 1 + popup_height <= area.y + area.height {
        cursor_y + 1
    } else {
        cursor_y.saturating_sub(popup_height)
    };

    let popup_rect = Rect::new(popup_x, popup_y, popup_width, popup_height);

    f.render_widget(Clear, popup_rect);

    let buf = f.buffer_mut();
    for y in popup_rect.top()..popup_rect.bottom() {
        for x in popup_rect.left()..popup_rect.right() {
            buf[(x, y)].set_char(' ').set_bg(colors.bg).set_fg(colors.fg);
        }
    }

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(colors.accent).bg(colors.bg))
        .style(Style::default().bg(colors.bg));

    if total > max_visible {
        block = block.title_bottom(
            Line::from(format!(" ({}/{}) ", buffer.autocomplete_idx + 1, total))
                .alignment(Alignment::Right)
                .style(Style::default().fg(colors.accent).bg(colors.bg)),
        );
    }

    let inner = block.inner(popup_rect);
    f.render_widget(block, popup_rect);

    let window_start = if buffer.autocomplete_idx >= max_visible {
        (buffer.autocomplete_idx + 1).saturating_sub(max_visible)
    } else {
        0
    };
    let window_end = (window_start + max_visible).min(total);

    let mut lines = Vec::new();
    let inner_width = inner.width as usize;
    for idx in window_start..window_end {
        let opt = &buffer.autocomplete_options[idx];
        let is_selected = idx == buffer.autocomplete_idx;

        let marker = if is_selected { "▸ " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(colors.accent)
                .bg(colors.sel)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(colors.fg).bg(colors.bg)
        };

        let raw_text = format!("{}{}", marker, opt);
        let raw_width = UnicodeWidthStr::width(raw_text.as_str());
        let pad_len = inner_width.saturating_sub(raw_width);
        let line_text = format!("{}{}", raw_text, " ".repeat(pad_len));

        lines.push(Line::from(Span::styled(line_text, style)));
    }

    f.render_widget(Paragraph::new(lines).bg(colors.bg), inner);
}

fn draw_context_menu(f: &mut Frame, app: &App, colors: &UIColors) {
    let Some(menu) = &app.context_menu else {
        return;
    };

    let area = Rect::new(menu.x, menu.y, menu.width, menu.height);
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(colors.accent))
        .bg(colors.bg);

    f.render_widget(block, area);

    let inner_x = area.x + 1;
    let inner_w = area.width.saturating_sub(2);
    let start_y = area.y + 1;

    let mouse_y = app.mouse_pos.map(|(_, my)| my);
    let mouse_x = app.mouse_pos.map(|(mx, _)| mx);

    for (i, item) in menu.items.iter().enumerate() {
        let row_y = start_y + i as u16;
        if row_y >= area.y + area.height - 1 {
            break;
        }

        let is_mouse_hover = mouse_y == Some(row_y)
            && mouse_x.map(|mx| mx >= inner_x && mx < inner_x + inner_w).unwrap_or(false);
        let is_selected = menu.selected_idx == i || is_mouse_hover;

        let icon_span = Span::styled(
            format!("{} ", item.icon),
            if item.is_danger {
                Style::default().fg(colors.error)
            } else {
                Style::default().fg(colors.accent)
            },
        );

        let label_text = app.i18n.t(item.i18n_key);
        let label_span = Span::styled(
            label_text,
            if is_selected {
                Style::default().fg(Color::Rgb(0, 0, 0)).add_modifier(Modifier::BOLD)
            } else if item.is_danger {
                Style::default().fg(colors.error)
            } else {
                Style::default().fg(colors.fg)
            },
        );

        let shortcut_str = item.shortcut.unwrap_or("");
        let shortcut_len = UnicodeWidthStr::width(shortcut_str) as u16;
        let prefix_len = 2 + UnicodeWidthStr::width(label_text) as u16;
        let spaces_needed = (inner_w as usize).saturating_sub(prefix_len as usize + shortcut_len as usize + 2);
        let padding_span = Span::raw(" ".repeat(spaces_needed));

        let shortcut_span = Span::styled(
            shortcut_str,
            if is_selected {
                Style::default().fg(Color::Rgb(50, 50, 50))
            } else {
                Style::default().fg(colors.text_muted())
            },
        );

        let row_style = if is_selected {
            if item.is_danger {
                Style::default().bg(colors.error)
            } else {
                Style::default().bg(colors.accent)
            }
        } else {
            Style::default().bg(colors.bg)
        };

        let pointer = if is_selected { "▸" } else { " " };
        let pointer_span = Span::styled(
            pointer,
            if is_selected {
                Style::default().fg(Color::Rgb(0, 0, 0)).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(colors.bg)
            },
        );

        let line = Line::from(vec![
            pointer_span,
            Span::raw(" "),
            icon_span,
            label_span,
            padding_span,
            shortcut_span,
        ]);

        f.render_widget(Paragraph::new(line).style(row_style), Rect::new(inner_x, row_y, inner_w, 1));
    }
}

/// Floating notification toasts stacked in the configured corner.
///
/// Each toast is borderless chrome over the editor: no filled card and no
/// clearing halo, just a type-colored rounded border with the message wrapped
/// to fit the screen and a progress bar on the last line that drains as the
/// toast approaches dismissal. The toast footprint is painted with the editor
/// background so moving toasts never leave ghost text behind.
/// Width adapts to the longest content within bounds; toasts that would
/// overflow the screen are dropped; expired toasts are skipped (the periodic
/// tick prunes them right after).
fn draw_toasts(f: &mut Frame, app: &App, colors: &UIColors) {
    use crate::app::toast::{layout_visible_toasts, toast_close_cell, toast_title};

    // Geometry comes from the shared layout helper, so hit-testing clicks
    // later always matches exactly what was painted here.
    let layouts = layout_visible_toasts(
        &app.notifications,
        f.area(),
        app.config.notification_position,
    );
    for layout in &layouts {
        let toast = &app.notifications[layout.source_idx];
        let rect = layout.rect;
        let inner = layout.inner;
        let kind_color = match toast.kind {
            crate::app::NotificationType::Error => colors.error,
            crate::app::NotificationType::Info => colors.accent,
        };
        // Flat editor-background patch: erases whatever was behind the toast
        // (no ghosting) without the hard rectangular halo that `Clear` leaves,
        // since `Clear` resets to the terminal default instead of the theme bg.
        f.render_widget(Block::default().bg(colors.bg), rect);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(kind_color).bg(colors.bg))
            .style(Style::default().bg(colors.bg))
            .title(Span::styled(
                toast_title(toast.kind),
                Style::default()
                    .fg(kind_color)
                    .bg(colors.bg)
                    .add_modifier(Modifier::BOLD),
            ));
        f.render_widget(block, rect);
        let text_area = Rect::new(
            rect.x.saturating_add(2),
            rect.y.saturating_add(1),
            rect.width.saturating_sub(4),
            layout.body.len() as u16,
        );
        f.render_widget(
            Paragraph::new(layout.body.join("\n"))
                .style(Style::default().fg(colors.fg).bg(colors.bg)),
            text_area,
        );
        // Draining progress bar: full when born, empty at dismissal.
        let filled = (toast.remaining_frac() * inner as f32).round() as usize;
        let filled = filled.min(inner);
        let bar: String = "█".repeat(filled) + &"░".repeat(inner.saturating_sub(filled));
        let bar_area = Rect::new(
            rect.x.saturating_add(2),
            rect.y
                .saturating_add(1)
                .saturating_add(layout.body.len() as u16),
            rect.width.saturating_sub(4),
            1,
        );
        f.render_widget(
            Paragraph::new(bar).style(Style::default().fg(kind_color).bg(colors.bg)),
            bar_area,
        );
        // Small close button on the top border, just left of the corner.
        if let Some((cx, cy)) = toast_close_cell(layout) {
            let close = Paragraph::new(app.icon_registry.get_command_icon("Close Tab"))
                .style(Style::default().fg(kind_color).bg(colors.bg));
            f.render_widget(close, Rect::new(cx, cy, 1, 1));
        }
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    let mode_text = if app.is_welcome {
        " WELCOME "
    } else if app.is_fuzzy {
        " FUZZY "
    } else if app.live_script_mode && app.focus == Focus::Editor {
        " LIVE SCRIPT "
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
            let raw_file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "[No Name]".to_string());
            let trunc_file_name = truncate_with_ellipsis(&raw_file_name, 25);

            let path_str = if components.len() > 1 {
                let parent_parts: Vec<_> = components[..components.len() - 1].iter().collect();
                let parent_str = if parent_parts.len() > 2 {
                    let last_dirs: Vec<_> = parent_parts.iter().rev().take(2).rev().collect();
                    let mut p = String::new();
                    for part in last_dirs {
                        p.push_str(&part.as_os_str().to_string_lossy());
                        p.push('/');
                    }
                    format!(".../{p}")
                } else {
                    let mut p = String::new();
                    for part in parent_parts {
                        p.push_str(&part.as_os_str().to_string_lossy());
                        p.push('/');
                    }
                    p
                };
                let combined = format!("{parent_str}{trunc_file_name}");
                truncate_with_ellipsis(&combined, 38)
            } else {
                trunc_file_name
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
    } else if app.live_script_mode && app.focus == Focus::Editor {
        vec![
            (app.config.get_keybind("run_live_script").to_uppercase(), "Run"),
            ("Ctrl+Alt+←".to_string(), "Prev Tab"),
            ("Ctrl+Alt+→".to_string(), "Next Tab"),
            (
                app.config.get_keybind("command_palette").to_uppercase(),
                "Palette",
            ),
        ]
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

/// Split the fuzzy query into spans, highlighting the active selection.
/// Byte ranges come from char indices, so slicing is always on boundaries.
fn fuzzy_query_spans<'a>(app: &'a App, colors: &UIColors) -> Vec<Span<'a>> {
    match app.fuzzy_selection_bytes() {
        Some((s, e)) if s < e => vec![
            Span::raw(&app.fuzzy_query[..s]),
            Span::styled(
                &app.fuzzy_query[s..e],
                Style::default().bg(colors.sel).fg(colors.accent),
            ),
            Span::raw(&app.fuzzy_query[e..]),
        ],
        _ => vec![Span::raw(app.fuzzy_query.as_str())],
    }
}

/// Terminal cursor position for the fuzzy input field, if the mode shows one.
/// DeleteConfirm/UnsavedChanges/ExternalChange have no text field.
fn fuzzy_cursor_position(app: &App, input_area: Rect, prefix_width: usize) -> Option<(u16, u16)> {
    if !app.fuzzy_has_editable_input() {
        return None;
    }
    let cursor_byte = app.fuzzy_byte_idx(app.fuzzy_cursor);
    let query_width = UnicodeWidthStr::width(&app.fuzzy_query[..cursor_byte]);
    let x = input_area
        .x
        .saturating_add(prefix_width.saturating_add(query_width) as u16);
    let max_x = input_area
        .x
        .saturating_add(input_area.width.saturating_sub(1));
    Some((x.min(max_x), input_area.y))
}

fn centered_rect_fixed(w: u16, h: u16, r: Rect) -> Rect {
    let w = w.min(r.width.saturating_sub(2)).max(1);
    let h = h.min(r.height.saturating_sub(2)).max(1);
    let x = r.x + (r.width.saturating_sub(w)) / 2;
    let y = r.y + (r.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

fn draw_confirmation_modal(f: &mut Frame, app: &mut App, colors: &UIColors) {
    let area = centered_rect_fixed(66, 9, f.area());
    f.render_widget(Clear, area);

    let is_delete = app.fuzzy_mode == FuzzyMode::DeleteConfirm;
    let border_color = if is_delete { colors.error } else { colors.accent };

    let title_text = match app.fuzzy_mode {
        FuzzyMode::DeleteConfirm => format!(" 󰆴  {} ", app.i18n.t("delete_confirm")),
        FuzzyMode::UnsavedChanges => {
            if app.pending_action == Some(crate::app::types::PendingAction::Quit) {
                " 󰆓  Quit NEdit ".to_string()
            } else if app.live_script_mode && app.pending_buffer_idx == app.live_script_buffer_idx {
                " 󰆓  Close Live Script ".to_string()
            } else {
                format!(" 󰆓  {} ", app.i18n.t("unsaved_changes"))
            }
        }
        FuzzyMode::ExternalChange => format!(" 󰑐  {} ", app.i18n.t("external_change")),
        _ => " Dialog ".to_string(),
    };

    let block = Block::default()
        .title(
            Line::from(title_text).style(
                Style::default()
                    .fg(border_color)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .bg(colors.bg);

    f.render_widget(block, area);

    let inner_y = area.y + 1;
    let inner_x = area.x + 2;
    let inner_w = area.width.saturating_sub(4);

    let (prompt_line, target_line, subtitle_line, hint_line) = match app.fuzzy_mode {
        FuzzyMode::DeleteConfirm => {
            let path_str = app
                .pending_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let trunc_path = truncate_with_ellipsis(&path_str, inner_w.saturating_sub(6) as usize);
            let prompt = format!(" {}", app.i18n.t("delete_prompt"));
            let warning = format!("  {}", app.i18n.t("delete_warning"));
            (
                Line::from(vec![
                    Span::styled(
                        prompt,
                        Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  󰈔 ", Style::default().fg(colors.error)),
                    Span::styled(
                        trunc_path,
                        Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        warning,
                        Style::default().fg(colors.text_muted()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [←/→] Select   [Enter] Confirm   [Esc] Cancel",
                        Style::default().fg(colors.text_muted()),
                    ),
                ]),
            )
        }
        FuzzyMode::UnsavedChanges => {
            let is_script = app.live_script_mode
                && app.pending_buffer_idx == app.live_script_buffer_idx;
            let is_quit = app.pending_action == Some(crate::app::types::PendingAction::Quit);

            let filename = if is_script {
                "Live Script".to_string()
            } else {
                app.pending_buffer_idx
                    .and_then(|idx| app.buffers.get(idx))
                    .and_then(|buf| buf.path.as_ref())
                    .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                    .unwrap_or_else(|| app.i18n.t("no_name").to_string())
            };
            let trunc_name = truncate_with_ellipsis(&filename, inner_w.saturating_sub(6) as usize);

            let (prompt_key, hints) = if is_script {
                (
                    "unsaved_script_prompt",
                    " [←/→] Select   [Enter] Confirm   [Esc] Cancel",
                )
            } else if is_quit {
                (
                    "unsaved_quit_prompt",
                    " [←/→] Select   [Enter] Confirm   [S] Save   [D] Discard   [Esc] Cancel",
                )
            } else {
                (
                    "unsaved_prompt",
                    " [←/→] Select   [Enter] Confirm   [S] Save   [D] Discard   [Esc] Cancel",
                )
            };
            let prompt = format!(" {}", app.i18n.t(prompt_key));
            let sub = format!("  {}", app.i18n.t("unsaved_warning"));

            (
                Line::from(vec![
                    Span::styled(
                        prompt,
                        Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  󰈔 ", Style::default().fg(colors.accent)),
                    Span::styled(
                        trunc_name,
                        Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(sub, Style::default().fg(colors.text_muted())),
                ]),
                Line::from(vec![
                    Span::styled(format!(" {hints}"), Style::default().fg(colors.text_muted())),
                ]),
            )
        }
        FuzzyMode::ExternalChange => {
            let filename = app
                .pending_buffer_idx
                .and_then(|idx| app.buffers.get(idx))
                .and_then(|buf| buf.path.as_ref())
                .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
                .unwrap_or_else(|| app.i18n.t("no_name").to_string());
            let trunc_name = truncate_with_ellipsis(&filename, inner_w.saturating_sub(6) as usize);

            let prompt = format!(" {}", app.i18n.t("external_change_prompt"));
            let warning = format!("  {}", app.i18n.t("external_change_warning"));
            (
                Line::from(vec![
                    Span::styled(
                        prompt,
                        Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled("  󰈔 ", Style::default().fg(colors.accent)),
                    Span::styled(
                        trunc_name,
                        Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        warning,
                        Style::default().fg(colors.text_muted()),
                    ),
                ]),
                Line::from(vec![
                    Span::styled(
                        " [←/→] Select   [Enter] Confirm   [R] Reload   [K] Keep   [Esc] Cancel",
                        Style::default().fg(colors.text_muted()),
                    ),
                ]),
            )
        }
        _ => return,
    };

    f.render_widget(Paragraph::new(prompt_line), Rect::new(inner_x, inner_y, inner_w, 1));
    f.render_widget(Paragraph::new(target_line), Rect::new(inner_x, inner_y + 1, inner_w, 1));
    f.render_widget(Paragraph::new(subtitle_line), Rect::new(inner_x, inner_y + 2, inner_w, 1));

    let buttons = app.modal_buttons();
    let selected_idx = app.modal_button_idx.unwrap_or(0);
    let btn_row_y = inner_y + 4;
    let mut btn_cur_x = inner_x + 1;

    for (i, (base_label, action, is_danger)) in buttons.into_iter().enumerate() {
        let is_selected = selected_idx == i;
        let display_label = if is_selected {
            format!(" ▸ {base_label} ")
        } else {
            format!("   {base_label} ")
        };
        let label_len = UnicodeWidthStr::width(display_label.as_str()) as u16;
        let is_hovered = app
            .mouse_pos
            .map(|(mx, my)| my == btn_row_y && mx >= btn_cur_x && mx < btn_cur_x + label_len)
            .unwrap_or(false);

        let btn_style = if is_danger {
            if is_selected || is_hovered {
                Style::default()
                    .bg(colors.error)
                    .fg(Color::Rgb(0, 0, 0))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .bg(colors.button_danger_bg())
                    .fg(Color::Rgb(255, 255, 255))
                    .add_modifier(Modifier::BOLD)
            }
        } else if is_selected || is_hovered {
            Style::default()
                .bg(colors.accent)
                .fg(Color::Rgb(0, 0, 0))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(colors.button_bg())
                .fg(colors.fg)
                .add_modifier(Modifier::BOLD)
        };

        f.render_widget(
            Paragraph::new(Span::styled(display_label, btn_style)),
            Rect::new(btn_cur_x, btn_row_y, label_len, 1),
        );

        app.modal_button_hitboxes.push(ModalButtonHitbox {
            x: btn_cur_x,
            y: btn_row_y,
            width: label_len,
            height: 1,
            action,
        });

        btn_cur_x += label_len + 2;
    }

    f.render_widget(Paragraph::new(hint_line), Rect::new(inner_x, inner_y + 6, inner_w, 1));
}

fn draw_input_modal(f: &mut Frame, app: &mut App, colors: &UIColors) {
    let area = centered_rect_fixed(66, 8, f.area());
    f.render_widget(Clear, area);

    let (title, prompt, icon) = match app.fuzzy_mode {
        FuzzyMode::Create => (
            format!(" 󰉋  {} ", app.i18n.t("new_file_folder_title")),
            format!(" {}", app.i18n.t("new_file_folder_prompt")),
            "󰉋 ",
        ),
        FuzzyMode::Rename => (
            format!(" 󰏫  {} ", app.i18n.t("rename_item_title")),
            format!(" {}", app.i18n.t("rename_item_prompt")),
            "󰏫 ",
        ),
        FuzzyMode::SaveAs => (
            format!(" 󰆓  {} ", app.i18n.t("save_buffer_as_title")),
            format!(" {}", app.i18n.t("save_buffer_as_prompt")),
            "󰆓 ",
        ),
        _ => return,
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(colors.accent))
        .bg(colors.bg);

    f.render_widget(block, area);

    let inner_y = area.y + 1;
    let inner_x = area.x + 2;
    let inner_w = area.width.saturating_sub(4);

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!(" {prompt}"),
            Style::default().fg(colors.fg).add_modifier(Modifier::BOLD),
        )])),
        Rect::new(inner_x, inner_y, inner_w, 1),
    );

    let query_spans = fuzzy_query_spans(app, colors);
    let mut spans = vec![Span::styled(format!(" {icon} "), Style::default().fg(colors.accent))];
    let prefix_width = UnicodeWidthStr::width(format!(" {icon} ").as_str());
    spans.extend(query_spans);

    let input_area = Rect::new(inner_x + 1, inner_y + 2, inner_w.saturating_sub(2), 1);
    f.render_widget(Paragraph::new(Line::from(spans)).bg(colors.button_bg()), input_area);

    if let Some((cursor_x, cursor_y)) = fuzzy_cursor_position(app, input_area, prefix_width) {
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let btn_row_y = inner_y + 4;
    let buttons = app.modal_buttons();
    let mut btn_cur_x = inner_x + 1;
    for (i, (base_label, action, _)) in buttons.into_iter().enumerate() {
        let is_selected = app.modal_button_idx == Some(i);
        let display_label = if is_selected {
            format!(" ▸ {base_label} ")
        } else {
            format!("   {base_label} ")
        };
        let label_len = UnicodeWidthStr::width(display_label.as_str()) as u16;
        let is_hovered = app
            .mouse_pos
            .map(|(mx, my)| my == btn_row_y && mx >= btn_cur_x && mx < btn_cur_x + label_len)
            .unwrap_or(false);

        let btn_style = if is_selected || is_hovered {
            Style::default()
                .bg(colors.accent)
                .fg(Color::Rgb(0, 0, 0))
                .add_modifier(Modifier::BOLD)
        } else if action == ModalAction::ConfirmInput {
            Style::default()
                .bg(colors.button_bg())
                .fg(colors.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .bg(colors.button_bg())
                .fg(colors.fg)
                .add_modifier(Modifier::BOLD)
        };

        f.render_widget(
            Paragraph::new(Span::styled(display_label, btn_style)),
            Rect::new(btn_cur_x, btn_row_y, label_len, 1),
        );

        app.modal_button_hitboxes.push(ModalButtonHitbox {
            x: btn_cur_x,
            y: btn_row_y,
            width: label_len,
            height: 1,
            action,
        });

        btn_cur_x += label_len + 2;
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            " [Tab/↓] Buttons   [Enter] Confirm   [Esc] Cancel",
            Style::default().fg(colors.text_muted()),
        )])),
        Rect::new(inner_x, inner_y + 6, inner_w, 1),
    );
}

fn draw_list_search_modal(f: &mut Frame, app: &mut App, colors: &UIColors) {
    let w: u16 = 74.min(f.area().width.saturating_sub(4));
    let h: u16 = ((f.area().height as f32 * 0.6) as u16)
        .max(16)
        .min(f.area().height.saturating_sub(2));
    let area = centered_rect_fixed(w, h, f.area());

    f.render_widget(Clear, area);

    let title = match app.fuzzy_mode {
        FuzzyMode::Content => format!("   {} ", app.i18n.t("global_search_content")),
        FuzzyMode::Local => format!("   {} ", app.i18n.t("local_search_file")),
        FuzzyMode::Files => format!("   {} ", app.i18n.t("fuzzy_finder_files")),
        FuzzyMode::Themes => format!(" 󰏘  {} ", app.i18n.t("select_color_theme")),
        FuzzyMode::FileOptions => format!(" 󰘳  {} ", app.i18n.t("file_options")),
        FuzzyMode::CommandPalette => format!(" 󰘳  {} ", app.i18n.t("command_palette")),
        FuzzyMode::Move => format!(" 󰏫  {} ", app.i18n.t("move_file")),
        FuzzyMode::DocSelect => format!(" 󰈔  {} ", app.i18n.t("select_documentation")),
        _ => " Search ".to_string(),
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
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(colors.accent))
        .bg(colors.bg);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let query_spans = fuzzy_query_spans(app, colors);
    let (input, prefix_width) = if app.fuzzy_mode == FuzzyMode::Move {
        let dir_str = app
            .move_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let prefix = format!(" 󰉋 {} > ", dir_str);
        let prefix_width = UnicodeWidthStr::width(prefix.as_str());
        let mut spans = vec![Span::styled(prefix, Style::default().fg(colors.accent))];
        spans.extend(query_spans);
        spans.push(Span::styled(
            " (Tab: Move here, Enter: Open)",
            Style::default().fg(colors.text_muted()),
        ));
        (Paragraph::new(Line::from(spans)), prefix_width)
    } else {
        let mut spans = vec![Span::styled("   ", Style::default().fg(colors.accent))];
        spans.extend(query_spans);
        (
            Paragraph::new(Line::from(spans)),
            UnicodeWidthStr::width("   "),
        )
    };

    f.render_widget(input, chunks[0]);

    if let Some((cursor_x, cursor_y)) = fuzzy_cursor_position(app, chunks[0], prefix_width) {
        f.set_cursor_position((cursor_x, cursor_y));
    }

    let total_matches = match app.fuzzy_mode {
        FuzzyMode::Local => app.fuzzy_lines.len(),
        FuzzyMode::Content => {
            if !app.fuzzy_results.is_empty() {
                app.fuzzy_results.len()
            } else {
                app.fuzzy_global_results.len()
            }
        }
        FuzzyMode::Themes => app.fuzzy_themes.len(),
        FuzzyMode::Files => {
            if !app.fuzzy_file_results.is_empty() {
                app.fuzzy_file_results.len()
            } else {
                app.fuzzy_results.len()
            }
        }
        FuzzyMode::CommandPalette
        | FuzzyMode::FileOptions
        | FuzzyMode::Move
        | FuzzyMode::DocSelect => app.fuzzy_results.len(),
        _ => 0,
    };

    if total_matches > 0 && chunks[0].width > 25 {
        let counter_text = format!(" [{} matches] ", total_matches);
        let counter_w = UnicodeWidthStr::width(counter_text.as_str()) as u16;
        if counter_w < chunks[0].width {
            let counter_x = chunks[0].x + chunks[0].width.saturating_sub(counter_w);
            f.render_widget(
                Paragraph::new(Span::styled(
                    counter_text,
                    Style::default().fg(colors.text_muted()),
                )),
                Rect::new(counter_x, chunks[0].y, counter_w, 1),
            );
        }
    }

    f.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(colors.indent_guide)),
        chunks[1],
    );

    let list_height = chunks[2].height as usize;
    let start_idx = app.fuzzy_idx.saturating_sub(list_height / 2);

    app.modal_list_area = Some(chunks[2]);
    app.modal_list_start_idx = start_idx;

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
                    ListItem::new(Line::from(vec![
                        Span::styled(format!(" {:4}: ", line_num + 1), Style::default().fg(colors.accent)),
                        Span::styled(text.trim(), style),
                    ]))
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
                        ListItem::new(Line::from(vec![
                            Span::styled(format!(" {name}"), style),
                            Span::styled(format!(" (L{}): ", line_num + 1), Style::default().fg(colors.text_muted())),
                            Span::styled(text.trim(), style),
                        ]))
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
        FuzzyMode::CommandPalette | FuzzyMode::FileOptions | FuzzyMode::DocSelect
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
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let style = if i == app.fuzzy_idx {
                        Style::default()
                            .bg(colors.sel)
                            .fg(colors.accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    let icon = match app.fuzzy_mode {
                        FuzzyMode::CommandPalette | FuzzyMode::FileOptions => {
                            app.icon_registry.get_command_icon(&name)
                        }
                        FuzzyMode::DocSelect => app.icon_registry.get_icon(path, false, false),
                        _ => "  ",
                    };
                    ListItem::new(format!(" {} {}", icon, name)).style(style)
                })
                .collect()
        }
    } else if app.fuzzy_mode == FuzzyMode::Files {
        if app.fuzzy_file_results.is_empty() {
            vec![]
        } else {
            let safe_start = start_idx.min(app.fuzzy_file_results.len().saturating_sub(1));
            let end_idx = (safe_start + list_height).min(app.fuzzy_file_results.len());
            app.fuzzy_file_results[safe_start..end_idx]
                .iter()
                .enumerate()
                .map(|(idx, fr)| {
                    let i = safe_start + idx;
                    let is_selected = i == app.fuzzy_idx;

                    let icon = app.icon_registry.get_icon(&fr.full_path, fr.full_path.is_dir(), false);

                    let match_set: std::collections::HashSet<usize> =
                        fr.match_positions.iter().copied().collect();
                    let mut spans = Vec::new();
                    spans.push(Span::raw(format!(" {} ", icon)));

                    for (ci, ch) in fr.relative_path.char_indices() {
                        let style = if is_selected {
                            if match_set.contains(&ci) {
                                Style::default()
                                    .bg(colors.sel)
                                    .fg(colors.accent)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().bg(colors.sel).fg(colors.accent)
                            }
                        } else if match_set.contains(&ci) {
                            Style::default().fg(colors.accent).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(colors.fg)
                        };
                        spans.push(Span::styled(ch.to_string(), style));
                    }

                    let line_style = if is_selected {
                        Style::default().bg(colors.sel).fg(colors.accent).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(colors.fg)
                    };
                    ListItem::new(Line::from(spans)).style(line_style)
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
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {} {} ", icon, name), style),
                    Span::styled(format!("({})", rel_path), Style::default().fg(colors.text_muted())),
                ]))
            })
            .collect()
    };

    f.render_widget(List::new(items), chunks[2]);
    f.render_widget(block, area);
}

fn draw_fuzzy_finder(f: &mut Frame, app: &mut App, colors: &UIColors) {
    app.modal_button_hitboxes.clear();
    app.modal_list_area = None;

    match app.fuzzy_mode {
        FuzzyMode::DeleteConfirm | FuzzyMode::UnsavedChanges | FuzzyMode::ExternalChange => {
            draw_confirmation_modal(f, app, colors);
        }
        FuzzyMode::Create | FuzzyMode::Rename | FuzzyMode::SaveAs => {
            draw_input_modal(f, app, colors);
        }
        _ => {
            draw_list_search_modal(f, app, colors);
        }
    }
}

#[cfg(test)]
mod syntax_resolution_tests {
    use super::resolve_buffer_syntax;
    use crate::buffer::EditorBuffer;
    use std::path::PathBuf;
    use syntect::parsing::SyntaxSet;

    #[test]
    fn override_wins_path_wins_plain_text() {
        let syntax_set = SyntaxSet::load_defaults_nonewlines();

        // Pathless buffer with override (live script pane) -> Lua.
        let mut script = EditorBuffer::new();
        script.syntax_override = Some("lua".to_string());
        assert_eq!(resolve_buffer_syntax(&syntax_set, &script).name, "Lua");

        // Regular file keeps extension-based detection.
        let mut rust_file = EditorBuffer::new();
        rust_file.path = Some(PathBuf::from("main.rs"));
        assert_eq!(resolve_buffer_syntax(&syntax_set, &rust_file).name, "Rust");

        // Override beats a conflicting path, unknown override falls back.
        let mut conflicted = EditorBuffer::new();
        conflicted.path = Some(PathBuf::from("main.rs"));
        conflicted.syntax_override = Some("lua".to_string());
        assert_eq!(resolve_buffer_syntax(&syntax_set, &conflicted).name, "Lua");
        let mut unknown = EditorBuffer::new();
        unknown.path = Some(PathBuf::from("script.py"));
        unknown.syntax_override = Some("no-such-lang".to_string());
        assert_eq!(resolve_buffer_syntax(&syntax_set, &unknown).name, "Python");

        // Neither override nor path -> plain text.
        let plain = EditorBuffer::new();
        assert_eq!(
            resolve_buffer_syntax(&syntax_set, &plain).name,
            "Plain Text"
        );
    }
}

#[cfg(test)]
mod large_file_render_tests {
    use super::render;
    use crate::app::App;
    use crate::buffer::EditorBuffer;
    use ratatui::{backend::TestBackend, Terminal};
    use ropey::Rope;
    use std::path::PathBuf;

    #[test]
    fn rendering_large_file_keeps_line_caches_empty() {
        let mut app = App::new(&[]);
        let content = Rope::from_str("alpha\nbeta\ngamma\n");
        app.current_buffer_idx = app.push_buffer(EditorBuffer::from_loaded_large_file(
            PathBuf::from("large.rs"),
            content,
        ));
        app.is_welcome = false;

        let backend = TestBackend::new(80, 12);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();

        let buffer = &app.buffers[app.current_buffer_idx];
        assert!(buffer.syntax_states.is_empty());
        assert!(buffer.rendered_spans.is_empty());
        assert_eq!(buffer.max_visual_width, None);
    }

    #[test]
    fn loading_buffer_renders_centered_indicator() {
        let mut app = App::new(&[]);
        let path = PathBuf::from("big_file.txt");
        app.current_buffer_idx = app.push_buffer(EditorBuffer::loading(path.clone(), 42));
        app.is_welcome = false;

        let width = 80;
        let height = 12;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let buf = terminal.backend().buffer().clone();

        let all_text = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        let found_loading = all_text.iter().any(|line| line.contains("[Loading]"));
        assert!(found_loading, "Expected [Loading] in rendered output, got:\n{}", all_text.join("\n"));

        // Header status bar should NOT be rendered while loading
        let found_header = all_text.iter().any(|line| line.contains("Lines:"));
        assert!(!found_header, "Header status bar shouldn't render while loading");

        // Line numbers shouldn't render while loading
        let found_line_num = all_text.iter().any(|line| line.starts_with("  1 "));
        assert!(!found_line_num, "Line numbers shouldn't render while loading");

        // Center row contains [Loading]
        let center_row = &all_text[5];
        let center_row2 = &all_text[6];
        assert!(
            center_row.contains("[Loading]") || center_row2.contains("[Loading]"),
            "Expected [Loading] centered vertically, rows:\n5: {}\n6: {}",
            center_row,
            center_row2
        );

        // When load finishes, content and header are shown, [Loading] is gone
        app.apply_large_file_load(crate::app::LargeFileLoadResult {
            request_id: 42,
            path: path.clone(),
            result: Ok(Rope::from_str("hello large world\nsecond line\n")),
        });

        terminal.draw(|frame| render(frame, &mut app)).unwrap();
        let buf_after = terminal.backend().buffer().clone();
        let all_text_after = (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf_after[(x, y)].symbol().to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();

        assert!(
            !all_text_after.iter().any(|line| line.contains("[Loading]")),
            "[Loading] should disappear after load completes"
        );
        assert!(
            all_text_after.iter().any(|line| line.contains("hello large world")),
            "File content should be displayed after load completes"
        );
        assert!(
            all_text_after.iter().any(|line| line.contains("Lines: 3")),
            "Header metrics should be displayed after load completes"
        );
    }
}

#[cfg(test)]
mod explorer_search_render_tests {
    use super::{draw_explorer, get_colors};
    use crate::app::App;
    use crate::explorer::FileItem;
    use ratatui::{backend::TestBackend, Terminal};

    fn row_text(buf: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
        (0..width)
            .map(|x| buf[(x, y)].symbol().to_string())
            .collect::<String>()
            .trim_end()
            .to_string()
    }

    #[test]
    fn search_bar_renders_under_title_with_borders_and_filters_rows() {
        let mut app = App::new(&[]);
        app.focus = crate::app::Focus::Explorer;
        for name in ["alpha.txt", "beta.txt"] {
            app.explorer.items.push(FileItem {
                path: std::path::PathBuf::from(format!("/root/{name}")),
                is_dir: false,
                name: name.to_string(),
                depth: 0,
                expanded: false,
            });
        }
        app.explorer.search_corpus = app
            .explorer
            .items
            .iter()
            .map(|i| (i.path.clone(), i.is_dir))
            .collect();
        app.explorer.search_input.set_text("alp".to_string());
        app.explorer.update_search_results(true);
        assert_eq!(app.explorer.search_results.len(), 1);

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_explorer(f, &app, f.area(), &colors);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        // Row 0 carries the Explorer title and match counter...
        let row0 = row_text(&buf, 0, 40);
        assert!(row0.contains("Explorer"), "got: {row0}");
        assert!(row0.contains("(1)"), "got: {row0}");
        // Row 1 is the top border of the search box
        let row1 = row_text(&buf, 1, 40);
        assert!(row1.contains('╭'), "got: {row1}");
        // Row 2 is the search box content with icon and query
        let row2 = row_text(&buf, 2, 40);
        assert!(row2.contains("󰍉"), "got: {row2}");
        assert!(row2.contains("alp"), "got: {row2}");
        // Row 3 is the bottom border of the search box
        let row3 = row_text(&buf, 3, 40);
        assert!(row3.contains('╰'), "got: {row3}");
        // Row 4 lists the matching item
        let row4 = row_text(&buf, 4, 40);
        assert!(row4.contains("alpha.txt"), "got: {row4}");
        for y in 4..10 {
            assert!(!row_text(&buf, y, 40).contains("beta.txt"));
        }
    }

    #[test]
    fn long_search_query_scrolls_horizontally_without_spilling_or_breaking_borders() {
        let mut app = App::new(&[]);
        app.focus = crate::app::Focus::Explorer;
        // Total explorer width 25 columns
        let width = 25;
        let height = 10;
        let long_query = "this_is_a_very_long_search_query_that_exceeds_explorer_width";
        app.explorer.search_input.set_text(long_query.to_string());
        app.explorer.update_search_results(true);

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_explorer(f, &app, f.area(), &colors);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();

        // Rightmost column of explorer is always the separator border │
        for y in 0..height {
            assert_eq!(
                buf[(width - 1, y)].symbol(),
                "│",
                "right border broken at row {y}"
            );
        }

        // Search box borders are intact on rows 1, 2, 3
        let row1 = row_text(&buf, 1, width);
        assert!(row1.contains('╭'), "got: {row1}");
        assert!(row1.contains('╮'), "got: {row1}");

        let row2 = row_text(&buf, 2, width);
        // Search icon is present
        assert!(row2.contains("󰍉"), "got: {row2}");
        // Search box left and right borders are intact
        assert!(row2.contains('│'), "got: {row2}");
        // The tail of the long query is visible because of horizontal scrolling
        assert!(row2.contains("width"), "got: {row2}");
        // The start of the query has scrolled off
        assert!(!row2.contains("this_is_a_very"), "got: {row2}");

        let row3 = row_text(&buf, 3, width);
        assert!(row3.contains('╰'), "got: {row3}");
        assert!(row3.contains('╯'), "got: {row3}");
    }

    #[test]
    fn search_bar_and_match_counter_maintain_accent_color_when_unfocused() {
        let mut app = App::new(&[]);
        app.focus = crate::app::Focus::Editor;
        app.explorer.items.push(crate::explorer::FileItem {
            path: std::path::PathBuf::from("/root/alpha.txt"),
            is_dir: false,
            name: "alpha.txt".to_string(),
            depth: 0,
            expanded: false,
        });
        app.explorer.search_corpus = vec![(std::path::PathBuf::from("/root/alpha.txt"), false)];
        app.explorer.search_input.set_text("alp".to_string());
        app.explorer.update_search_results(true);

        let backend = TestBackend::new(40, 10);
        let mut terminal = Terminal::new(backend).unwrap();
        let colors = get_colors(&app);
        terminal
            .draw(|f| {
                draw_explorer(f, &app, f.area(), &colors);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();

        let row0_text = row_text(&buf, 0, 40);
        let paren_col = row0_text.find('(').expect("should have ( in row 0");
        assert_eq!(buf[(paren_col as u16, 0)].fg, colors.accent);

        let row1_text = row_text(&buf, 1, 40);
        let corner_col = row1_text.find('╭').expect("should have ╭ in row 1");
        assert_eq!(buf[(corner_col as u16, 1)].fg, colors.accent);
    }
}

#[cfg(test)]
mod toast_render_tests {
    use super::{draw_toasts, get_colors};
    use crate::app::{App, NotificationType};
    use crate::config::NotificationPosition;
    use ratatui::{backend::TestBackend, Terminal};

    fn render_toasts(app: &App, width: u16, height: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let colors = get_colors(app);
                draw_toasts(f, app, &colors);
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn app_with_toast(msg: &str) -> App {
        let mut app = App::new(&[]);
        app.show_notification(msg.to_string(), NotificationType::Info);
        app
    }

    #[test]
    fn toast_hugs_bottom_right_by_default() {
        let app = app_with_toast("Hi");
        assert_eq!(
            app.config.notification_position,
            NotificationPosition::BottomRight
        );
        // 1-line message -> width clamps to min 20, height 4 (border+line+bar+border).
        let buf = render_toasts(&app, 80, 24);
        assert_eq!(buf[(58, 19)].symbol(), "╭");
        assert_eq!(buf[(77, 19)].symbol(), "╮");
        assert_eq!(buf[(58, 22)].symbol(), "╰");
        assert_eq!(buf[(77, 22)].symbol(), "╯");
        // No filled card and no clearing halo: the interior blends with the
        // editor background, only border/text/progress carry color.
        let colors = get_colors(&app);
        assert_eq!(buf[(60, 20)].bg, colors.bg);
        assert_eq!(buf[(60, 20)].symbol(), "H");
        // The close button replaces the top border just left of the corner.
        assert_ne!(buf[(76, 19)].symbol(), "─");
    }

    #[test]
    fn toast_follows_configured_corner() {
        let mut app = app_with_toast("Hi");
        app.config.notification_position = NotificationPosition::TopLeft;
        let buf = render_toasts(&app, 80, 24);
        assert_eq!(buf[(2, 1)].symbol(), "╭");
        assert_eq!(buf[(2, 4)].symbol(), "╰");
    }

    #[test]
    fn long_message_wraps_and_stays_on_screen() {
        let app = app_with_toast(&"word ".repeat(30));
        let buf = render_toasts(&app, 30, 10);
        // Width clamps to the 30-col screen (1-col margins): right edge at col 28.
        // 4 capped message lines -> height 7, bottom row at 10-1-1 = 8.
        assert_eq!(buf[(28, 2)].symbol(), "╮");
        assert_eq!(buf[(28, 8)].symbol(), "╯");
    }

    #[test]
    fn expired_toasts_render_nothing() {
        let mut app = app_with_toast("Hi");
        // Age the toast past its duration.
        app.notifications[0].created -= std::time::Duration::from_secs(60);
        assert!(app.notifications[0].is_expired());
        let buf = render_toasts(&app, 80, 24);
        assert_eq!(buf[(58, 19)].symbol(), " ");
    }
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

#[cfg(test)]
mod explorer_marquee_and_truncation_tests {
    use super::*;
    use crate::app::App;
    use crate::explorer::FileItem;
    use ratatui::{backend::TestBackend, Terminal};
    use std::path::PathBuf;
    use std::time::Duration;

    #[test]
    fn test_truncate_with_ellipsis() {
        assert_eq!(truncate_with_ellipsis("short.rs", 10), "short.rs");
        assert_eq!(truncate_with_ellipsis("exact10len", 10), "exact10len");
        assert_eq!(
            truncate_with_ellipsis("very_long_filename_here.txt", 15),
            "very_long_fi..."
        );
        assert_eq!(truncate_with_ellipsis("ab", 2), "ab");
        assert_eq!(truncate_with_ellipsis("abcdef", 2), "ab");
        assert_eq!(truncate_with_ellipsis("abcdef", 3), "abc");
        assert_eq!(truncate_with_ellipsis("abcdef", 4), "a...");
        // Multibyte unicode characters
        assert_eq!(truncate_with_ellipsis("ação_coração", 8), "ação_...");
    }

    #[test]
    fn test_calculate_marquee_offset() {
        // Fits within width: no scroll
        let (offset, animating) = calculate_marquee_offset(10, 15, Duration::from_secs(5));
        assert_eq!(offset, 0);
        assert!(!animating);

        // Long name: 30 chars, avail_width: 10 -> max_offset = 20
        // pause_start: 1000ms, step: 220ms, pause_end: 1200ms
        // scroll_duration: 20 * 220 = 4400ms. total_cycle = 1000 + 4400 + 1200 = 6600ms
        let (offset_start, anim) = calculate_marquee_offset(30, 10, Duration::from_millis(500));
        assert_eq!(offset_start, 0);
        assert!(anim);

        // 1000ms + 220ms * 2 = 1440ms -> offset 2
        let (offset_mid, _) = calculate_marquee_offset(30, 10, Duration::from_millis(1440));
        assert_eq!(offset_mid, 2);

        // During end pause: 1000 + 4400 + 500 = 5900ms -> max_offset (20)
        let (offset_end, _) = calculate_marquee_offset(30, 10, Duration::from_millis(5900));
        assert_eq!(offset_end, 20);

        // After full cycle (6600ms + 100ms) -> restarts from beginning (offset 0)
        let (offset_restarted, _) = calculate_marquee_offset(30, 10, Duration::from_millis(6700));
        assert_eq!(offset_restarted, 0);
    }

    #[test]
    fn test_format_explorer_name_selected_vs_unselected() {
        let long_name = "pi-session-2026-08-05T17-41-53-338Z_019fd304-7839-7357-8568-aaaef680af";
        let avail = 15;

        // Unselected item is truncated with ellipsis '…'
        let (unselected, anim) = format_explorer_name(long_name, avail, false, Duration::ZERO);
        assert_eq!(unselected.chars().count(), avail);
        assert!(unselected.ends_with('…'));
        assert!(!anim);

        // Selected item starts with marquee at offset 0
        let (selected_start, anim) = format_explorer_name(long_name, avail, true, Duration::ZERO);
        assert_eq!(selected_start.chars().count(), avail);
        assert_eq!(selected_start, &long_name[..avail]);
        assert!(anim);

        // Selected item after scroll time advances
        let (selected_later, _) = format_explorer_name(
            long_name,
            avail,
            true,
            Duration::from_millis(1000 + 220 * 5),
        );
        assert_eq!(selected_later.chars().count(), avail);
        let expected_slice: String = long_name.chars().skip(5).take(avail).collect();
        assert_eq!(selected_later, expected_slice);
    }

    #[test]
    fn test_explorer_renders_with_standard_width_and_no_overflow() {
        let mut app = App::new(&[]);
        app.show_explorer = true;
        app.focus = Focus::Explorer;
        let long_name =
            "pi-session-2026-08-05T17-41-53-338Z_019fd304-7839-7357-8568-aaaef680af.json";
        app.explorer.items.push(FileItem {
            path: PathBuf::from(format!("/root/{long_name}")),
            is_dir: false,
            name: long_name.to_string(),
            depth: 0,
            expanded: false,
        });
        app.explorer.selected_idx = 0;

        let width = 120;
        let height = 20;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                render(f, &mut app);
            })
            .unwrap();
        let buf = terminal.backend().buffer().clone();

        // Standard explorer width is 20% of 120 = 24 cols.
        // Therefore, column 23 must be the explorer right border '│'.
        assert_eq!(buf[(23, 0)].symbol(), "│");
        assert_eq!(buf[(23, 5)].symbol(), "│");
        assert_eq!(buf[(23, 10)].symbol(), "│");

        // The active marquee flag should be set since the filename exceeds 20 cols
        assert!(app.explorer.has_active_marquee.get());
    }

    #[test]
    fn test_tab_bar_truncates_long_filename() {
        let mut app = App::new(&[]);
        let long_name =
            "pi-session-2026-08-05T17-41-53-338Z_019fd304-7839-7357-8568-aaaef680af.json";
        let mut buf = crate::buffer::EditorBuffer::new();
        buf.path = Some(PathBuf::from(format!("/tmp/{long_name}")));
        app.buffers.push(buf);
        app.current_buffer_idx = 0;
        app.is_welcome = false;

        let width = 100;
        let height = 10;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_tab_bar(f, &mut app, Rect::new(0, 0, width, 1), &colors);
            })
            .unwrap();
        let rendered_buffer = terminal.backend().buffer().clone();
        let row0: String = (0..width)
            .map(|x| rendered_buffer[(x, 0)].symbol().to_string())
            .collect();

        // Row should contain truncated tab name with '...' and NOT full 75-char name
        assert!(row0.contains("..."));
        assert!(!row0.contains(long_name));
    }

    #[test]
    fn test_status_bar_truncates_long_filename() {
        let mut app = App::new(&[]);
        let long_name =
            "pi-session-2026-08-05T17-41-53-338Z_019fd304-7839-7357-8568-aaaef680af.json";
        let mut buf = crate::buffer::EditorBuffer::new();
        buf.path = Some(PathBuf::from(format!("/tmp/{long_name}")));
        app.buffers.push(buf);
        app.current_buffer_idx = 0;
        app.is_welcome = false;

        let width = 120;
        let height = 5;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_status_bar(f, &app, Rect::new(0, 4, width, 1), &colors);
            })
            .unwrap();
        let rendered_buffer = terminal.backend().buffer().clone();
        let row4: String = (0..width)
            .map(|x| rendered_buffer[(x, 4)].symbol().to_string())
            .collect();

        // Status bar should contain truncated name with '...' and keep shortcuts
        assert!(row4.contains("..."));
        assert!(!row4.contains(long_name));
        assert!(row4.contains("Save"));
    }

    #[test]
    fn test_tab_bar_hover_shows_close_button_and_hitboxes() {
        let mut app = App::new(&[]);
        let mut buf1 = crate::buffer::EditorBuffer::new();
        buf1.path = Some(PathBuf::from("/tmp/first.rs"));
        let mut buf2 = crate::buffer::EditorBuffer::new();
        buf2.path = Some(PathBuf::from("/tmp/second.rs"));
        app.buffers.push(buf1);
        app.buffers.push(buf2);
        app.current_buffer_idx = 0;
        app.is_welcome = false;

        let width = 100;
        let height = 5;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        // 1. Without mouse hover: no close button rendered, hitboxes have close_start_x == None
        app.mouse_pos = None;
        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_tab_bar(f, &mut app, Rect::new(0, 0, width, 1), &colors);
            })
            .unwrap();

        let rendered = terminal.backend().buffer().clone();
        let row0_unhovered: String = (0..width).map(|x| rendered[(x, 0)].symbol().to_string()).collect();
        assert!(!row0_unhovered.contains("󰅖"));
        assert_eq!(app.tab_hitboxes.len(), 2);
        assert!(app.tab_hitboxes[0].close_start_x.is_none());
        assert!(app.tab_hitboxes[1].close_start_x.is_none());

        // 2. With mouse hover over tab 0: tab 0 shows close button and records close hitbox
        app.mouse_pos = Some((5, 0));
        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_tab_bar(f, &mut app, Rect::new(0, 0, width, 1), &colors);
            })
            .unwrap();

        let rendered = terminal.backend().buffer().clone();
        let row0_hovered: String = (0..width).map(|x| rendered[(x, 0)].symbol().to_string()).collect();
        assert!(row0_hovered.contains("󰅖"));
        assert!(app.tab_hitboxes[0].close_start_x.is_some());
        assert!(app.tab_hitboxes[0].close_end_x.is_some());
        assert!(app.tab_hitboxes[1].close_start_x.is_none());
    }

    #[test]
    fn test_confirmation_modal_renders_buttons_and_hitboxes() {
        let mut app = App::new(&[]);
        app.is_fuzzy = true;
        app.fuzzy_mode = FuzzyMode::DeleteConfirm;
        app.pending_path = Some(PathBuf::from("/tmp/important_file.rs"));

        let width = 100;
        let height = 30;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_fuzzy_finder(f, &mut app, &colors);
            })
            .unwrap();

        assert_eq!(app.modal_button_hitboxes.len(), 2);
        assert_eq!(app.modal_button_hitboxes[0].action, ModalAction::ConfirmDelete);
        assert_eq!(app.modal_button_hitboxes[1].action, ModalAction::Cancel);

        let rendered = terminal.backend().buffer().clone();
        let all_text: String = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| rendered[(x, y)].symbol().to_string())
            .collect();
        assert!(all_text.contains("Delete"));
        assert!(all_text.contains("Cancel"));
        assert!(all_text.contains("important_file.rs"));
    }

    #[test]
    fn test_input_modal_renders_buttons_and_hitboxes() {
        let mut app = App::new(&[]);
        app.is_fuzzy = true;
        app.fuzzy_mode = FuzzyMode::Create;

        let width = 100;
        let height = 30;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                let colors = get_colors(&app);
                draw_fuzzy_finder(f, &mut app, &colors);
            })
            .unwrap();

        assert_eq!(app.modal_button_hitboxes.len(), 2);
        assert_eq!(app.modal_button_hitboxes[0].action, ModalAction::ConfirmInput);
        assert_eq!(app.modal_button_hitboxes[1].action, ModalAction::Cancel);
    }
}
