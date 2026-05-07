fn draw_fuzzy_finder(f: &mut Frame, app: &App) {
    let area = centered_rect(70, 50, f.area());
    f.render_widget(Clear, area);

    let title = if app.fuzzy_mode == crate::app::FuzzyMode::Content {
        "   Global Search (Content) "
    } else if app.fuzzy_mode == crate::app::FuzzyMode::Local {
        "   Local Search (Current File) "
    } else {
        "   Fuzzy Finder (Files) "
    };

    let block = Block::default()
        .title(Line::from(title).style(Style::default().fg(MOCHA_BLUE).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MOCHA_BLUE))
        .bg(MOCHA_BASE);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Input
            Constraint::Length(1), // Divider
            Constraint::Min(0),    // Results
        ])
        .split(area);

    let input_style = Style::default().fg(MOCHA_MAUVE).add_modifier(Modifier::BOLD);
    let input = Paragraph::new(Line::from(vec![
        Span::styled(" 󰍉 ", input_style),
        Span::raw(&app.fuzzy_query),
    ]));
    f.render_widget(input, chunks[0]);

    f.render_widget(Block::default().borders(Borders::TOP).border_style(Style::default().fg(MOCHA_SURFACE)), chunks[1]);

    let list_height = chunks[2].height as usize;
    let start_idx = app.fuzzy_idx.saturating_sub(list_height / 2);

    let items: Vec<ListItem> = if app.fuzzy_mode == crate::app::FuzzyMode::Local {
        let end_idx = (start_idx + list_height).min(app.fuzzy_lines.len());
        app.fuzzy_lines[start_idx..end_idx].iter().enumerate().map(|(idx, (line_num, text))| {
            let i = start_idx + idx;
            let style = if i == app.fuzzy_idx {
                Style::default().bg(MOCHA_SELECTED).fg(MOCHA_BLUE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MOCHA_TEXT)
            };
            ListItem::new(format!(" {}: {}", line_num + 1, text.trim())).style(style)
        }).collect()
    } else {
        let end_idx = (start_idx + list_height).min(app.fuzzy_results.len());
        app.fuzzy_results[start_idx..end_idx].iter().enumerate().map(|(idx, path)| {
            let i = start_idx + idx;
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let rel_path = path.strip_prefix(&app.explorer.root).unwrap_or(path).to_string_lossy();

            let style = if i == app.fuzzy_idx {
                Style::default().bg(MOCHA_SELECTED).fg(MOCHA_BLUE).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MOCHA_TEXT)
            };

            let icon = if path.is_dir() { "󰉋 " } else { "󰈔 " };
            ListItem::new(format!(" {} {} ({})", icon, name, rel_path)).style(style)
        }).collect()
    };

    let list = List::new(items);
    f.render_widget(list, chunks[2]);
    f.render_widget(block, area);
}
