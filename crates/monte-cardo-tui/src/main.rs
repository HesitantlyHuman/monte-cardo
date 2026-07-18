use std::io::{self, stdout};

use crossterm::{
    event::{self, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use ratatui::prelude::CrosstermBackend;

use crossterm::event::{Event, KeyCode};
use monte_cardo_tui::{App, AppKey};

fn map_crossterm_key(event: KeyEvent) -> Option<AppKey> {
    Some(match event.code {
        KeyCode::Esc => AppKey::Esc,
        KeyCode::Enter => AppKey::Enter,
        KeyCode::Left => AppKey::Left,
        KeyCode::Right => AppKey::Right,
        KeyCode::Up => AppKey::Up,
        KeyCode::Down => AppKey::Down,
        KeyCode::Tab => AppKey::Tab,
        KeyCode::Backspace => AppKey::Backspace,
        KeyCode::Delete => AppKey::Delete,
        KeyCode::Home => AppKey::Home,
        KeyCode::End => AppKey::End,
        KeyCode::PageUp => AppKey::PageUp,
        KeyCode::PageDown => AppKey::PageDown,
        KeyCode::Char(c) => {
            if event.modifiers == KeyModifiers::CONTROL {
                match c {
                    'c' => AppKey::ControlC,
                    'q' => AppKey::ControlQ,
                    _ => AppKey::Char(c),
                }
            } else {
                AppKey::Char(c)
            }
        }
        _ => return None,
    })
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = App::new();

    while !app.should_quit() {
        terminal.draw(|frame| {
            app.render(frame);
        })?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                if let Some(key) = map_crossterm_key(key_event) {
                    app.handle_event(key)?;
                }
            }
        }

        app.tick();
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
