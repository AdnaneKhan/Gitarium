//! Keyboard/paste events fed into the App by the browser host. (Mouse and
//! wheel are handled in pixel space by the px view, which resolves them to
//! semantic clicks/scrolls itself.)

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Esc,
    Tab,
    BackTab,
    Backspace,
    Delete,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl Mods {
    pub const NONE: Mods = Mods { ctrl: false, alt: false, shift: false };
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Key(Key, Mods),
    Paste(String),
}
