//! How a cell is drawn, as the editor sees it. Two independent channels:
//! what the text is, which decides its colour, and what state lies under
//! it. Kept apart so that a keyword stays a keyword inside a selection and
//! on the current line, without a variant for every combination. The
//! terminal layer that paints these does not care what they mean.

/// What the text in a cell is. Syntax tokens will join this list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Ink {
    #[default]
    Text,
    Gutter,
    Rule,
    Status,
}

/// What lies under the text: a state of the editor, not of the text. A cell
/// under several of them takes one, the topmost; the order is the view's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mark {
    #[default]
    None,
    CurrentLine,
    Matched,
    Selected,
    Cursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellStyle {
    pub ink: Ink,
    pub mark: Mark,
}

impl CellStyle {
    /// Text of `ink` with nothing under it.
    pub const fn of(ink: Ink) -> Self {
        Self {
            ink,
            mark: Mark::None,
        }
    }

    /// Ordinary text with `mark` under it.
    pub const fn marked(mark: Mark) -> Self {
        Self {
            ink: Ink::Text,
            mark,
        }
    }
}
