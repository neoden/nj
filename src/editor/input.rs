//! What the editor takes from the outside: keystrokes, the mouse, pasted
//! text — in its own terms, so that where they come from is the terminal's
//! business alone. Turning any of it into a command is decided here, with
//! the editor's state at hand.

use super::command::{Command, Edit, Select};
use super::key::{Keystroke, Mods};
use super::{Editor, keymap};

const MOUSE_SCROLL_LINES: isize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Key(Keystroke),
    Mouse(Mouse),
    /// Bracketed paste: the whole text at once instead of a replay of
    /// keystrokes.
    Paste(String),
}

/// The mouse speaks in screen cells; the editor knows which view they fall in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mouse {
    pub kind: MouseKind,
    pub mods: Mods,
    pub row: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseKind {
    ScrollUp,
    ScrollDown,
    /// The left button going down.
    Press,
    /// The pointer moving with the left button held.
    Drag,
}

impl Input {
    /// The command this means right now, if it means one.
    pub fn into_command(self, editor: &Editor) -> Option<Command> {
        match self {
            Input::Key(stroke) => keymap::resolve(stroke, editor),
            Input::Mouse(mouse) => Some(mouse.command()),
            Input::Paste(text) => Some(Command::Edit(Edit::Insert(text))),
        }
    }
}

impl Mouse {
    fn command(self) -> Command {
        let Mouse { row, column, .. } = self;
        match self.kind {
            MouseKind::ScrollUp => Command::Scroll(-MOUSE_SCROLL_LINES),
            MouseKind::ScrollDown => Command::Scroll(MOUSE_SCROLL_LINES),
            MouseKind::Press => Command::Select(Select::Click {
                row,
                column,
                add: self.mods.contains(Mods::ALT),
            }),
            MouseKind::Drag => Command::Select(Select::Drag { row, column }),
        }
    }
}
