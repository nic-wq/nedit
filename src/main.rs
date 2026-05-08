use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{CrosstermBackend, TestBackend},
    Terminal,
};
use std::collections::HashMap;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use nedit::app::{App, FuzzyMode};
use nedit::{input, lua, ui};

fn format_duration(duration: Duration) -> String {
    if duration.as_secs() > 0 {
        format!("{:.3}s", duration.as_secs_f64())
    } else {
        format!("{:.3}ms", duration.as_secs_f64() * 1000.0)
    }
}

fn print_step(name: &str, duration: Duration, detail: impl AsRef<str>) {
    let detail = detail.as_ref();
    if detail.is_empty() {
        println!("{:<34} {}", name, format_duration(duration));
    } else {
        println!("{:<34} {}  {}", name, format_duration(duration), detail);
    }
}

fn print_check(name: &str, ok: bool, detail: impl AsRef<str>) {
    let status = if ok { "ok" } else { "failed" };
    let detail = detail.as_ref();
    if detail.is_empty() {
        println!("{:<34} {}", name, status);
    } else {
        println!("{:<34} {}  {}", name, status, detail);
    }
}

fn count_files_in_dir(path: &Path) -> usize {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .count()
        })
        .unwrap_or(0)
}

fn command_exists(command: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path).any(|dir| dir.join(command).is_file())
}

fn temp_debug_dir() -> PathBuf {
    std::env::temp_dir().join(format!(
        "nedit-debug-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ))
}

fn diagnose_environment() -> anyhow::Result<()> {
    println!("== Environment ==");
    print_check(
        "stdin_tty",
        io::stdin().is_terminal(),
        "interactive terminal input",
    );
    print_check(
        "stdout_tty",
        io::stdout().is_terminal(),
        "interactive terminal output",
    );
    println!(
        "{:<34} TERM={} COLORTERM={} SHELL={}",
        "terminal_env",
        std::env::var("TERM").unwrap_or_else(|_| "unset".to_string()),
        std::env::var("COLORTERM").unwrap_or_else(|_| "unset".to_string()),
        std::env::var("SHELL").unwrap_or_else(|_| "unset".to_string())
    );
    println!(
        "{:<34} WAYLAND_DISPLAY={} DISPLAY={}",
        "display_env",
        std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "unset".to_string()),
        std::env::var("DISPLAY").unwrap_or_else(|_| "unset".to_string())
    );

    let exe = std::env::current_exe()?;
    let metadata = fs::metadata(&exe)?;
    println!(
        "{:<34} profile={} exe={} size={} bytes",
        "binary",
        if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        exe.display(),
        metadata.len()
    );

    print_check(
        "clipboard_wl_copy",
        command_exists("wl-copy"),
        "external Wayland copy helper",
    );
    print_check(
        "clipboard_wl_paste",
        command_exists("wl-paste"),
        "external Wayland paste helper",
    );
    print_check(
        "clipboard_xclip",
        command_exists("xclip"),
        "external X11 clipboard helper",
    );
    println!();
    Ok(())
}

fn diagnose_config_files(app: &App) -> anyhow::Result<()> {
    println!("== Config ==");
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("nedit");
    let config_file = config_dir.join("config.toml");
    let theme_file = config_dir.join("theme.txt");
    let language_file = config_dir.join("language.toml");
    let workspace_file = config_dir.join("workspaces.toml");
    let syntax_dir = config_dir.join("syntax");
    let themes_dir = config_dir.join("themes");

    print_check(
        "config_dir_exists",
        config_dir.is_dir(),
        config_dir.display().to_string(),
    );
    print_check(
        "config_file",
        config_file.is_file(),
        config_file.display().to_string(),
    );
    print_check(
        "theme_file",
        theme_file.is_file(),
        theme_file.display().to_string(),
    );
    print_check(
        "language_file",
        language_file.is_file(),
        language_file.display().to_string(),
    );
    print_check(
        "workspace_file",
        workspace_file.is_file(),
        workspace_file.display().to_string(),
    );
    println!(
        "{:<34} syntax_files={} theme_files={}",
        "custom_assets",
        count_files_in_dir(&syntax_dir),
        count_files_in_dir(&themes_dir)
    );

    let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();
    for (action, key) in &app.config.keybinds {
        reverse
            .entry(key.as_str())
            .or_default()
            .push(action.as_str());
    }
    let collisions: Vec<String> = reverse
        .into_iter()
        .filter(|(_, actions)| actions.len() > 1)
        .map(|(key, actions)| format!("{}={}", key, actions.join(",")))
        .collect();
    print_check(
        "keybind_collisions",
        collisions.is_empty(),
        if collisions.is_empty() {
            "none".to_string()
        } else {
            collisions.join("; ")
        },
    );
    println!();
    Ok(())
}

fn diagnose_filesystem() -> anyhow::Result<()> {
    println!("== Filesystem ==");
    let dir = temp_debug_dir();
    let file = dir.join("sample.txt");
    let renamed = dir.join("sample-renamed.txt");
    let start = Instant::now();
    fs::create_dir_all(&dir)?;
    fs::write(&file, "nedit diagnostics\n")?;
    let read_back = fs::read_to_string(&file)?;
    fs::rename(&file, &renamed)?;
    fs::remove_file(&renamed)?;
    fs::remove_dir_all(&dir)?;
    print_step(
        "tmp_create_write_read_rename_rm",
        start.elapsed(),
        format!("ok={}", read_back == "nedit diagnostics\n"),
    );
    if read_back != "nedit diagnostics\n" {
        anyhow::bail!("Temporary filesystem roundtrip failed");
    }
    println!();
    Ok(())
}

fn diagnose_watcher() -> anyhow::Result<()> {
    println!("== Watcher ==");
    let dir = temp_debug_dir();
    fs::create_dir_all(&dir)?;

    let start = Instant::now();
    let mut app = App::new(&[dir.to_string_lossy().to_string()]);
    wait_for_background_tasks(&mut app);
    fs::write(dir.join("watch-created.txt"), "watch\n")?;

    let mut saw_refresh = false;
    for _ in 0..50 {
        app.handle_fs_events();
        if app.explorer_refresh_receiver.is_some() {
            saw_refresh = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    wait_for_background_tasks(&mut app);
    fs::remove_dir_all(&dir)?;

    print_check(
        "notify_create_event",
        saw_refresh,
        format!("elapsed={}", format_duration(start.elapsed())),
    );
    if !saw_refresh {
        anyhow::bail!("Filesystem watcher did not observe a create event");
    }
    println!();
    Ok(())
}

fn diagnose_buffer_core() -> anyhow::Result<()> {
    println!("== Buffer ==");
    let start = Instant::now();
    let mut buffer = nedit::buffer::EditorBuffer::new();
    for ch in "hello".chars() {
        buffer.insert_char(ch);
    }
    buffer.insert_char('\n');
    for ch in "world".chars() {
        buffer.insert_char(ch);
    }
    buffer.move_cursor(-1, 0, 80);
    buffer.move_to_line_end();
    buffer.delete_backspace();
    buffer.undo();
    buffer.redo();
    let words = buffer.collect_all_words();
    let ok = buffer.content.len_lines() >= 1 && words.contains_key("world");
    print_step(
        "edit_cursor_history_words",
        start.elapsed(),
        format!(
            "ok={} lines={} words={}",
            ok,
            buffer.content.len_lines(),
            words.len()
        ),
    );
    if !ok {
        anyhow::bail!("Buffer editing smoke test failed");
    }
    println!();
    Ok(())
}

fn diagnose_render(app: &mut App) -> anyhow::Result<()> {
    println!("== Render ==");
    let start = Instant::now();
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend)?;
    terminal.draw(|f| ui::render(f, app))?;
    print_step("ratatui_test_render", start.elapsed(), "100x30");
    let background_wait = wait_for_background_tasks(app);
    print_step(
        "render_background_wait",
        background_wait,
        format!("syntax_loaded={}", app.syntax_set.is_some()),
    );
    println!();
    Ok(())
}

fn diagnose_lua_filesystem() -> anyhow::Result<()> {
    println!("== Lua ==");
    let dir = temp_debug_dir();
    fs::create_dir_all(&dir)?;
    fs::write(dir.join("input.txt"), "lua input")?;
    let script = r#"
        local content = nedit.read_file("input.txt")
        nedit.create_file("created.txt", content .. " ok")
        nedit.write_selection(content)
    "#;
    let ctx = crate::lua::LuaContext {
        current_file: String::new(),
        current_content: String::new(),
        current_selection: String::new(),
        current_dir: dir.clone(),
        is_live_script: false,
    };

    let start = Instant::now();
    let actions = lua::run_script(script, ctx, &None).map_err(|e| anyhow::anyhow!(e))?;
    fs::remove_dir_all(&dir)?;
    let ok = actions.len() == 2;
    print_step(
        "lua_file_api",
        start.elapsed(),
        format!("ok={} actions={}", ok, actions.len()),
    );
    if !ok {
        anyhow::bail!("Lua filesystem API smoke test failed");
    }
    println!();
    Ok(())
}

fn wait_for_background_tasks(app: &mut App) -> Duration {
    let start = Instant::now();
    while app.explorer_refresh_receiver.is_some()
        || app.indexed_files_receiver.is_some()
        || app.content_search_receiver.is_some()
        || app.syntax_set_receiver.is_some()
    {
        app.poll_background_tasks();
        std::thread::sleep(Duration::from_millis(1));
    }
    start.elapsed()
}

fn run_diagnostics(args: &[String]) -> anyhow::Result<()> {
    let start = Instant::now();
    println!("--- NEdit Debug Report ---");
    println!("version: {}", env!("CARGO_PKG_VERSION"));
    println!("cwd: {}", std::env::current_dir()?.display());
    println!(
        "config_dir: {}",
        dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("nedit")
            .display()
    );
    println!(
        "args: {}",
        if args.is_empty() {
            "none".to_string()
        } else {
            args.join(" ")
        }
    );
    println!();

    diagnose_environment()?;

    println!("== App Startup ==");
    let app_start = Instant::now();
    let mut app = App::new(args);
    print_step(
        "app_new",
        app_start.elapsed(),
        format!(
            "root={} buffers={} workspace={}",
            app.explorer.root.display(),
            app.buffers.len(),
            app.current_workspace.as_deref().unwrap_or("none")
        ),
    );

    let explorer_wait = wait_for_background_tasks(&mut app);
    print_step(
        "background_after_startup",
        explorer_wait,
        format!("explorer_items={}", app.explorer.items.len()),
    );

    let config_ok = !app.config.theme.is_empty();
    let i18n_ok = !app.i18n.defaults.is_empty();
    println!(
        "{:<34} config={} i18n_defaults={} theme={}",
        "config_i18n",
        if config_ok { "ok" } else { "failed" },
        app.i18n.defaults.len(),
        app.current_theme
    );
    if !config_ok || !i18n_ok {
        anyhow::bail!("Config/i18n loading failed");
    }

    println!(
        "{:<34} total={} active={}",
        "workspaces",
        app.workspaces.len(),
        app.current_workspace.as_deref().unwrap_or("none")
    );
    println!();

    diagnose_config_files(&app)?;
    diagnose_filesystem()?;
    diagnose_watcher()?;
    diagnose_buffer_core()?;
    diagnose_render(&mut app)?;

    println!("== Path Handling ==");
    let dir_start = Instant::now();
    let temp_dir = std::env::temp_dir().canonicalize()?;
    let app_with_dir = App::new(&[temp_dir.to_string_lossy().to_string()]);
    print_step(
        "app_new_with_directory_arg",
        dir_start.elapsed(),
        format!("root={}", app_with_dir.explorer.root.display()),
    );
    if app_with_dir.explorer.root.canonicalize()? != temp_dir {
        anyhow::bail!("Directory initialization failed");
    }
    println!();

    println!("== Lua Basic ==");
    let lua_start = Instant::now();
    let test_script = "nedit.write_selection('diagnostics')";
    let lua_ctx = crate::lua::LuaContext {
        current_file: String::new(),
        current_content: String::new(),
        current_selection: String::new(),
        current_dir: std::env::current_dir()?,
        is_live_script: false,
    };
    match lua::run_script(test_script, lua_ctx, &None) {
        Ok(actions) => {
            print_step(
                "lua_smoke",
                lua_start.elapsed(),
                format!("actions={}", actions.len()),
            );
            if actions.len() != 1 {
                anyhow::bail!("Lua engine failed to return actions");
            }
        }
        Err(e) => {
            anyhow::bail!("Lua engine error: {}", e);
        }
    }
    println!();
    diagnose_lua_filesystem()?;

    println!("== File Open ==");
    let open_start = Instant::now();
    app.open_file(std::path::PathBuf::from("test.rs"));
    print_step(
        "open_file_smoke",
        open_start.elapsed(),
        format!("buffers={}", app.buffers.len()),
    );
    println!();

    println!("== Syntax And Themes ==");
    let mut syntax_app = App::new(args);
    let syntax_start = Instant::now();
    syntax_app.ensure_syntax_set_loaded();
    print_step("syntax_load_sync", syntax_start.elapsed(), "");
    if let Some(syntax_set) = &syntax_app.syntax_set {
        let syntax = syntax_set.find_syntax_by_extension("rs");
        if syntax.is_none() {
            anyhow::bail!("Syntax set incomplete");
        }
    } else {
        anyhow::bail!("Syntax set failed to load");
    }

    let theme_start = Instant::now();
    app.ensure_all_themes_loaded();
    print_step(
        "theme_load_all",
        theme_start.elapsed(),
        format!("themes={}", app.theme_set.themes.len()),
    );
    println!();

    println!("== Search ==");
    let index_start = Instant::now();
    app.toggle_fuzzy(FuzzyMode::Files);
    let kick_index_duration = index_start.elapsed();
    let index_wait = wait_for_background_tasks(&mut app);
    print_step(
        "file_index_kick",
        kick_index_duration,
        format!("ready={}", app.all_files_ready),
    );
    print_step(
        "file_index_wait",
        index_wait,
        format!("files={}", app.all_files.len()),
    );

    let content_start = Instant::now();
    app.fuzzy_mode = FuzzyMode::Content;
    app.fuzzy_query = "fn".to_string();
    app.update_fuzzy();
    let content_kick = content_start.elapsed();
    let content_wait = wait_for_background_tasks(&mut app);
    print_step("content_search_kick", content_kick, "");
    print_step(
        "content_search_wait",
        content_wait,
        format!("results={}", app.fuzzy_global_results.len()),
    );

    println!("-------------------------------");
    println!("completed in {}", format_duration(start.elapsed()));

    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Process arguments
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--debug".to_string()) {
        let diagnostic_args: Vec<String> = args
            .iter()
            .skip(1)
            .filter(|arg| arg.as_str() != "--debug")
            .cloned()
            .collect();
        return run_diagnostics(&diagnostic_args);
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(&args[1..]);

    // Main loop
    let mut tick_counter: u8 = 0;
    loop {
        app.handle_fs_events();
        app.poll_background_tasks();
        terminal.draw(|f| ui::render(f, &mut app))?;

        if let Err(e) = input::handle_events(&mut app) {
            eprintln!("Error handling events: {}", e);
        }

        tick_counter = tick_counter.wrapping_add(1);
        if tick_counter == 0 {
            app.tick_notification();
        }

        if app.should_quit {
            app.save_workspaces();
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
