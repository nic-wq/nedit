use ratatui::prelude::Stylize;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::path::Path;
use syntect::highlighting::{HighlightIterator, HighlightState, Highlighter};
use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxReference, SyntaxSet};
use unicode_width::UnicodeWidthStr;

use crate::app::{App, Focus, FuzzyMode};
use crate::buffer::{column::TAB_WIDTH, EditorBuffer};

use super::welcome::draw_welcome_screen;
use super::{centered_rect, get_colors, UIColors};

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

pub fn render(f: &mut Frame, app: &mut App) {
    let colors = get_colors(app);

    // Remember the full frame for overlay hit-testing (toast close button).
    app.screen_area = f.area();

    f.render_widget(Block::default().bg(colors.bg), f.area());

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

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
        let current_path = app
            .buffers
            .get(app.current_buffer_idx)
            .and_then(|b| b.path.clone());
        app.ensure_syntax_for_path_loading(current_path.as_deref());

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

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    if app.buffers.is_empty() {
        return;
    }

    let mut spans = Vec::new();
    for (i, buffer) in app.buffers.iter().enumerate() {
        let is_live_script = Some(i) == app.live_script_buffer_idx;
        let is_preview = buffer.is_preview;
        if is_live_script || is_preview {
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

/// Always-visible explorer search bar (top row of the panel).
/// Typing with the explorer focused lands here; the cursor and the Shift
/// selection render exactly like the fuzzy finder input.
fn draw_explorer_search_bar(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    const PREFIX_WIDTH: usize = 3; // " " + icon + " "
    let mut spans = vec![Span::styled("  ", Style::default().fg(colors.accent))];
    match app.explorer.search_input.selection_bytes() {
        Some((s, e)) if s < e => {
            let query = &app.explorer.search_input.text;
            spans.push(Span::raw(query[..s].to_string()));
            spans.push(Span::styled(
                query[s..e].to_string(),
                Style::default().bg(colors.sel).fg(colors.accent),
            ));
            spans.push(Span::raw(query[e..].to_string()));
        }
        _ => spans.push(Span::raw(app.explorer.search_input.text.clone())),
    }
    f.render_widget(Paragraph::new(Line::from(spans)).bg(colors.bg), area);
    if app.focus == Focus::Explorer {
        let cursor_byte = app
            .explorer
            .search_input
            .byte_idx(app.explorer.search_input.cursor);
        let query_width = UnicodeWidthStr::width(&app.explorer.search_input.text[..cursor_byte]);
        let cursor_x = area
            .x
            .saturating_add(PREFIX_WIDTH.saturating_add(query_width) as u16);
        let max_x = area.x.saturating_add(area.width.saturating_sub(1));
        f.set_cursor_position((cursor_x.min(max_x), area.y));
    }
}

fn draw_explorer(f: &mut Frame, app: &App, area: Rect, colors: &UIColors) {
    let border_color = if app.focus == Focus::Explorer {
        colors.accent
    } else {
        colors.surface
    };
    // The title row doubles as the match counter while filtering, so the
    // row it occupies always earns its space.
    let title = if app.explorer.is_searching() {
        format!(
            " {} ({}) ",
            app.i18n.t("explorer"),
            app.explorer.search_results.len()
        )
    } else {
        format!(" {} ", app.i18n.t("explorer"))
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(border_color));

    if area.height == 0 {
        return;
    }
    // Row 0 is always the search bar; the list below shows the filtered
    // flat view while searching, else the tree.
    let bar_area = Rect::new(area.x, area.y, area.width, 1);
    draw_explorer_search_bar(f, app, bar_area, colors);

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

    let mut items: Vec<ListItem> = Vec::new();
    if searching && len == 0 {
        items.push(ListItem::new(" No matches").style(Style::default().fg(colors.surface)));
    }
    for actual_idx in scroll..end {
        let item = if searching {
            &app.explorer.search_results[actual_idx]
        } else {
            &app.explorer.items[actual_idx]
        };
        let style = if actual_idx == selected {
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
            let mut spans = vec![Span::raw(format!("{}{}", icon, item.name))];
            if !parent.is_empty() {
                spans.push(Span::styled(
                    format!("  {parent}"),
                    Style::default().fg(colors.surface),
                ));
            }
            items.push(ListItem::new(Line::from(spans)).style(style));
        } else {
            let indent = "  ".repeat(item.depth);
            let icon = app
                .icon_registry
                .get_icon(&item.path, item.is_dir, item.expanded);
            items.push(
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
                .style(style),
            );
        }
    }

    let list_area = Rect::new(
        area.x,
        area.y.saturating_add(1),
        area.width,
        area.height.saturating_sub(1),
    );
    f.render_widget(List::new(items).block(block).bg(colors.bg), list_area);
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
    if buffer.max_visual_width.is_none() {
        buffer.update_max_visual_width();
    }
    let lines = buffer.content.len_lines();
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

    let matching_bracket = if app.config.highlight_matching_bracket {
        buffer.find_matching_bracket()
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
        .filter(|_| !is_markdown && buffer.content.len_bytes() <= 5_242_880)
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

    let md_highlight = if is_markdown {
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
    let (active_indent_level, active_scope_start, active_scope_end) = if is_focused {
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

fn draw_fuzzy_finder(f: &mut Frame, app: &App, colors: &UIColors) {
    let is_small = matches!(
        app.fuzzy_mode,
        FuzzyMode::SaveAs
            | FuzzyMode::Rename
            | FuzzyMode::DeleteConfirm
            | FuzzyMode::Create
            | FuzzyMode::UnsavedChanges
            | FuzzyMode::ExternalChange
    );
    let is_two_line_popup = matches!(
        app.fuzzy_mode,
        FuzzyMode::UnsavedChanges | FuzzyMode::ExternalChange | FuzzyMode::DeleteConfirm
    );

    let area = if is_small {
        let (w, h) = if is_two_line_popup {
            (70, 6)
        } else {
            (70, 3)
        };
        let centered_y = (f.area().height.saturating_sub(h)) / 2;
        let centered_x = (f.area().width.saturating_sub(w)) / 2;
        Rect::new(centered_x, centered_y, w.min(f.area().width), h)
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
        FuzzyMode::DocSelect => " 󰈔  Select Documentation ".to_string(),
        FuzzyMode::Create => " 󰉋  New Name (trailing / = folder) ".to_string(),
        FuzzyMode::UnsavedChanges => format!(" 󰆓  {} ", app.i18n.t("unsaved_changes")),
        FuzzyMode::ExternalChange => format!(" 󰆓  {} ", app.i18n.t("external_change")),
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
        if is_two_line_popup {
            vec![Constraint::Min(1)]
        } else {
            vec![Constraint::Length(1)]
        }
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
            " (Tab: Move here, Enter: Open folder)",
            Style::default().fg(colors.surface),
        ));
        (Paragraph::new(Line::from(spans)), prefix_width)
    } else if app.fuzzy_mode == FuzzyMode::DeleteConfirm {
        let path_str = app
            .pending_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        (
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(" 󰆴 ", Style::default().fg(colors.error)),
                    Span::styled(
                        "Confirm Delete:",
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(Span::raw(path_str)),
                Line::from(vec![Span::styled(
                    format!("(Enter: Confirm  Esc: {})", app.i18n.t("cancel")),
                    Style::default().fg(colors.surface),
                )]),
            ])
            .wrap(Wrap { trim: false }),
            0,
        )
    } else if app.fuzzy_mode == FuzzyMode::UnsavedChanges {
        let filename = app.pending_buffer_idx
            .and_then(|idx| app.buffers.get(idx))
            .and_then(|buf| buf.path.as_ref())
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .unwrap_or_else(|| app.i18n.t("no_name").to_string());
        (
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(" 󰆓 ", Style::default().fg(colors.accent)),
                    Span::styled(
                        format!("Save changes to {}?", filename),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![Span::styled(
                    format!("(S: Save  D: Discard  Esc: {})", app.i18n.t("cancel")),
                    Style::default().fg(colors.accent),
                )]),
            ])
            .wrap(Wrap { trim: true }),
            0,
        )
    } else if app.fuzzy_mode == FuzzyMode::ExternalChange {
        let filename = app
            .pending_buffer_idx
            .and_then(|idx| app.buffers.get(idx))
            .and_then(|buf| buf.path.as_ref())
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .unwrap_or_else(|| app.i18n.t("no_name").to_string());
        // Short, two-line layout to avoid truncation on narrow terminals.
        (
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(" 󰆓 ", Style::default().fg(colors.accent)),
                    Span::styled(
                        format!("{} — {}", filename, app.i18n.t("file_changed")),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![Span::styled(
                    format!(
                        "({}: R  {}: K  Esc: {})",
                        app.i18n.t("reload"),
                        app.i18n.t("keep"),
                        app.i18n.t("cancel")
                    ),
                    Style::default().fg(colors.accent),
                )]),
            ])
            .wrap(Wrap { trim: true }),
            0,
        )
    } else {
        let mut spans = vec![Span::styled(" 󰍉 ", Style::default().fg(colors.accent))];
        spans.extend(query_spans);
        (
            Paragraph::new(Line::from(spans)),
            UnicodeWidthStr::width(" 󰍉 "),
        )
    };
    f.render_widget(input, chunks[0]);
    if let Some((cursor_x, cursor_y)) = fuzzy_cursor_position(app, chunks[0], prefix_width) {
        f.set_cursor_position((cursor_x, cursor_y));
    }

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
                            FuzzyMode::CommandPalette
                            | FuzzyMode::FileOptions => app.icon_registry.get_command_icon(&name),
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

                        // Build highlighted spans for the relative path.
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
                            } else {
                                if match_set.contains(&ci) {
                                    Style::default().fg(colors.accent).add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(colors.fg)
                                }
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
                    ListItem::new(format!(" {} {} ({})", icon, name, rel_path)).style(style)
                })
                .collect()
        };

        f.render_widget(List::new(items), chunks[2]);
    }

    f.render_widget(block, area);
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
    fn search_bar_renders_on_top_and_filters_rows() {
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
        // Row 0 is always the search bar showing the query...
        assert!(row_text(&buf, 0, 40).contains("alp"));
        // ...row 1 carries the match counter...
        assert!(row_text(&buf, 1, 40).contains("(1)"));
        // ...and only the match is listed below.
        let row2 = row_text(&buf, 2, 40);
        assert!(row2.contains("alpha.txt"), "got: {row2}");
        for y in 2..10 {
            assert!(!row_text(&buf, y, 40).contains("beta.txt"));
        }
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
