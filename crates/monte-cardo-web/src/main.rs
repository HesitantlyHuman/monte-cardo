use std::{cell::RefCell, io, rc::Rc};

use monte_cardo_tui::{App, AppKey};
use ratatui::Terminal;
use ratzilla::{event::KeyCode, DomBackend, WebRenderer};

fn main() -> io::Result<()> {
    console_error_panic_hook::set_once();

    let app = Rc::new(RefCell::new(App::new()));

    let backend = DomBackend::new()?;
    let mut terminal = Terminal::new(backend)?;

    terminal.on_key_event({
        let app = Rc::clone(&app);

        move |event| {
            if let Some(key) = map_web_key(event.code) {
                app.borrow_mut().handle_event(key).unwrap();
            }
        }
    })?;

    terminal.draw_web(move |frame| {
        let mut app = app.borrow_mut();

        app.tick();
        app.render(frame);
    });

    Ok(())
}

fn map_web_key(code: KeyCode) -> Option<AppKey> {
    Some(match code {
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
        KeyCode::Char(c) => AppKey::Char(c),
        KeyCode::F(_) | KeyCode::Unidentified => return None,
    })
}
