use std::{cell::RefCell, collections::VecDeque, io, rc::Rc};

use monte_cardo_tui::{App, AppKey};
use ratzilla::{
    backend::canvas::CanvasBackendOptions, ratatui::Terminal, CanvasBackend, WebRenderer,
};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::KeyboardEvent;

const TERMINAL_COLUMNS: u32 = 200;
const TERMINAL_ROWS: u32 = 55;

const CELL_WIDTH_PX: u32 = 10;
const CELL_HEIGHT_PX: u32 = 19;

// CanvasBackend reports one fewer row and column than its internal buffer.
const CANVAS_WIDTH_PX: u32 = (TERMINAL_COLUMNS + 1) * CELL_WIDTH_PX;
const CANVAS_HEIGHT_PX: u32 = (TERMINAL_ROWS + 1) * CELL_HEIGHT_PX;

fn main() -> io::Result<()> {
    console_error_panic_hook::set_once();

    let app = Rc::new(RefCell::new(App::new()));
    let pending_keys = Rc::new(RefCell::new(VecDeque::<AppKey>::new()));

    let backend = CanvasBackend::new_with_options(
        CanvasBackendOptions::new()
            .grid_id("monte-cardo-root")
            .size((CANVAS_WIDTH_PX, CANVAS_HEIGHT_PX)),
    )?;

    let terminal = Terminal::new(backend)?;

    install_keyboard_listener(Rc::clone(&pending_keys))?;

    terminal.draw_web({
        let app = Rc::clone(&app);
        let pending_keys = Rc::clone(&pending_keys);

        move |frame| {
            // Remove the queued keys while releasing the queue borrow
            // before mutating the application.
            let keys = {
                let mut pending_keys = pending_keys.borrow_mut();
                std::mem::take(&mut *pending_keys)
            };

            let mut app = app.borrow_mut();

            for key in keys {
                app.handle_event(key).unwrap();
            }

            app.tick();
            app.render(frame);
        }
    });

    Ok(())
}

fn install_keyboard_listener(pending_keys: Rc<RefCell<VecDeque<AppKey>>>) -> io::Result<()> {
    let listener = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event| {
        let Some(key) = map_browser_key(&event) else {
            return;
        };

        event.prevent_default();

        pending_keys.borrow_mut().push_back(key);
    });

    let window =
        web_sys::window().ok_or_else(|| io::Error::other("browser window is unavailable"))?;

    window
        .add_event_listener_with_callback("keydown", listener.as_ref().unchecked_ref())
        .map_err(|error| {
            io::Error::other(format!("failed to register keyboard listener: {error:?}"))
        })?;

    listener.forget();

    Ok(())
}
fn map_browser_key(event: &KeyboardEvent) -> Option<AppKey> {
    // Browser build does not use Ctrl+C or Ctrl+Q as application shortcuts.
    if event.ctrl_key() || event.meta_key() || event.alt_key() {
        return None;
    }

    let key = event.key();

    Some(match key.as_str() {
        "Escape" => AppKey::Esc,
        "Enter" => AppKey::Enter,
        "ArrowLeft" => AppKey::Left,
        "ArrowRight" => AppKey::Right,
        "ArrowUp" => AppKey::Up,
        "ArrowDown" => AppKey::Down,
        "Tab" => AppKey::Tab,
        "Backspace" => AppKey::Backspace,
        "Delete" => AppKey::Delete,
        "Home" => AppKey::Home,
        "End" => AppKey::End,
        "PageUp" => AppKey::PageUp,
        "PageDown" => AppKey::PageDown,
        " " => AppKey::Char(' '),

        value => {
            let mut characters = value.chars();
            let character = characters.next()?;

            if characters.next().is_some() {
                return None;
            }

            AppKey::Char(character)
        }
    })
}
