use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
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

    let open_start = Instant::now();
    app.open_file(std::path::PathBuf::from("test.rs"));
    print_step(
        "open_file_smoke",
        open_start.elapsed(),
        format!("buffers={}", app.buffers.len()),
    );

    let syntax_start = Instant::now();
    app.ensure_syntax_set_loaded();
    print_step("syntax_load_sync", syntax_start.elapsed(), "");
    if let Some(syntax_set) = &app.syntax_set {
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
