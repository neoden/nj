//! Everything the editor can be asked to do, as values (ADR 16). The keymap
//! produces them, the palette will list them, a socket will send them; the
//! editor executes them and nothing else reaches it.
//!
//! Nested by subsystem so the outer match routes and the modules do the
//! work. Nothing here borrows: a command is plain data that can be stored,
//! sent and later parsed from a configuration file.

use crate::movement::{Direction, Motion};
use crate::picker::Pick;
use crate::prompt::Purpose;
use crate::selection::SelectionSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Goes to the field of an open panel when there is one, to the text
    /// otherwise — like `Edit::Insert`, `Edit::Delete` and `Select::All`.
    Move { motion: Motion, extend: bool },
    /// Rows; the cursors stay where they are.
    Scroll(isize),
    /// The next hit of the open query, or of the last search once the
    /// prompt has closed.
    FindNext(Direction),
    Edit(Edit),
    Select(Select),
    File(File),
    Panel(PanelCommand),
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    Insert(String),
    Newline,
    Delete(Motion),
    Indent,
    Outdent,
    ToggleComment,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Select {
    All,
    Line,
    /// Collapses every selection to its cursor.
    Cancel,
    /// Keeps the primary cursor only.
    RemoveSecondary,
    AddCursor(Direction),
    NextOccurrence,
    /// Every hit of the open query while a panel is up; otherwise every
    /// whole-word occurrence of what is selected.
    AllOccurrences,
    /// Replaces the selections outright: what a socket sends to open a file
    /// at a line, and what a test uses to start from a known place.
    Set(SelectionSet),
    /// In screen cells. `add` keeps the existing cursors.
    Click { row: usize, column: usize, add: bool },
    Drag { row: usize, column: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum File {
    Save,
    /// By name, since a language is a table entry rather than a value.
    SetLanguage(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelCommand {
    Prompt(Purpose),
    Picker(Pick),
    Close,
    /// Backwards for `Shift+Enter` while searching.
    Submit { backwards: bool },
    /// Takes the picker's highlighted candidate.
    Pick,
    /// A single-key answer to a question.
    Answer(char),
    NextField,
    /// Moves the picker's highlight; positive is away from the query.
    MoveChoice(isize),
}
