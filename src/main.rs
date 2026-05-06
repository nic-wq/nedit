use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Instant;

use nedit::{input, lua, ui};
use nedit::app::App;

fn run_diagnostics() -> anyhow::Result<()> {
    let start = Instant::now();
    println!("--- NEdit Diagnostic Report ---");

    // 1. Startup Performance
    print!("[1/7] Checking startup performance... ");
    let args: Vec<String> = Vec::new();
    let mut app = App::new(&args);
    let startup_duration = start.elapsed();
    println!("OK ({:?})", startup_duration);

    // 2. Configuration & i18n
    print!("[2/7] Checking configuration & i18n... ");
    if app.config.theme.is_empty() {
        println!("FAILED (Theme empty)");
        anyhow::bail!("Config loading failed");
    }
    if app.i18n.defaults.is_empty() {
        println!("FAILED (i18n empty)");
        anyhow::bail!("i18n loading failed");
    }
    println!("OK");

    // 3. Active Workspace Test
    print!("[3/7] Testing workspace persistence... ");
    let test_ws_name = "__debug_test_workspace__".to_string();
    let current_dir = std::env::current_dir()?;
    app.create_workspace(test_ws_name.clone(), current_dir).map_err(|e| anyhow::anyhow!(e))?;
    app.save_workspaces();
    
    // Reload and check
    let mut app2 = App::new(&args);
    app2.load_workspaces();
    if !app2.workspaces.iter().any(|w| w.name == test_ws_name) {
        println!("FAILED (Workspace not persisted)");
        anyhow::bail!("Workspace persistence failed");
    }
    // Cleanup
    app2.workspaces.retain(|w| w.name != test_ws_name);
    app2.save_workspaces();
    println!("OK");

    // 4. Directory Initialization
    print!("[4/7] Testing directory initialization... ");
    let temp_dir = std::env::temp_dir().canonicalize()?;
    let app_with_dir = App::new(&[temp_dir.to_string_lossy().to_string()]);
    if app_with_dir.explorer.root.canonicalize()? != temp_dir {
        println!("FAILED (Explorer root mismatch)");
        anyhow::bail!("Directory initialization failed");
    }
    println!("OK");

    // 5. Lua Integration
    print!("[5/7] Testing Lua engine interaction... ");
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
            if actions.len() == 1 {
                println!("OK");
            } else {
                println!("FAILED (Actions mismatch)");
                anyhow::bail!("Lua engine failed to return actions");
            }
        }
        Err(e) => {
            println!("FAILED ({})", e);
            anyhow::bail!("Lua engine error: {}", e);
        }
    }

    // 6. Buffer & Syntax Validation
    print!("[6/7] Testing buffer & syntax detection... ");
    app.open_file(std::path::PathBuf::from("test.rs"));
    app.ensure_syntax_set_loaded();
    if let Some(syntax_set) = &app.syntax_set {
        let syntax = syntax_set.find_syntax_by_extension("rs");
        if syntax.is_none() {
            println!("FAILED (Rust syntax not found)");
            anyhow::bail!("Syntax set incomplete");
        }
    } else {
        println!("FAILED (Syntax set not loaded)");
        anyhow::bail!("Syntax set failed to load");
    }
    println!("OK");

    // 7. Search engine warmup
    print!("[7/7] Warming up search engine... ");
    // Ensure file list is triggered
    app.ensure_all_files_collected(); 
    println!("OK");

    println!("-------------------------------");
    println!("Integrity check COMPLETED in {:?}", start.elapsed());
    
    Ok(())
}

fn main() -> anyhow::Result<()> {
    // Process arguments
    let args: Vec<String> = std::env::args().collect();
    
    if args.contains(&"--debug".to_string()) {
        return run_diagnostics();
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
