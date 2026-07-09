mod app;
mod cards;
mod game;
mod hand;
mod live_widgets;
mod main_menu_widgets;
mod players;
mod rank_count_editor;
mod settings;
mod settings_widgets;
mod solver_worker;
mod table;
mod view_model;

use std::io::{self, stdout};

use crossterm::{
    event,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use ratatui::prelude::CrosstermBackend;

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = app::App::new();

    while !app.should_quit() {
        terminal.draw(|frame| {
            app.render(frame);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            let event = event::read()?;
            app.handle_event(event)?;
        }

        app.tick();
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
