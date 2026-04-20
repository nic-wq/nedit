use std::io;
use crossterm::{
    execute,
    event::{EnableMouseCapture, DisableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod buffer;
mod explorer;
mod input;
mod ui;
mod config;
mod i18n;
pub mod lua;
pub mod clipboard;

use crate::app::App;

fn main() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let args: Vec<String> = std::env::args().collect();
    let mut app = App::new(&args[1..]);

    // Main loop
    let mut tick_counter: u8 = 0;
    loop {
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}
