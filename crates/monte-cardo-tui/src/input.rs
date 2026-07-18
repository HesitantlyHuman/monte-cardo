#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppKey {
    Esc,
    Enter,
    Left,
    Right,
    Up,
    Down,
    Tab,
    Backspace,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    ControlC,
    ControlQ,
    Char(char),
}
