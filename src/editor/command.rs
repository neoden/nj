//! Everything the editor can be asked to do, as values (ADR 16). The keymap
//! produces them, the palette will list them, a socket will send them; the
//! editor executes them and nothing else reaches it.
//!
//! Nested by subsystem so the outer match routes and the modules do the
//! work. Nothing here borrows: a command is plain data that can be stored,
//! sent and later parsed from a configuration file.

use std::path::PathBuf;

use super::panel::picker::Pick;
use super::panel::prompt::Purpose;
use super::panel::After;
use crate::text::movement::{Direction, Motion};
use crate::text::selection::SelectionSet;

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
    /// Under this name from now on. A pipeline buffer writes a copy instead.
    SaveAs(PathBuf),
    /// In place of the buffer in focus, which asks first if it has unsaved
    /// changes. A path with no file behind it opens empty, like `ned newfile`.
    Open(PathBuf),
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

/// What the palette lists (F2): every command worth asking for by name.
/// Movement is not — nobody opens a palette to go left — and neither is
/// anything that only makes sense inside a panel.
pub const PALETTE: &[(&str, Command)] = &[
    ("Open file", Command::Panel(PanelCommand::Prompt(Purpose::Open))),
    ("Save", Command::File(File::Save)),
    ("Save as", Command::Panel(PanelCommand::Prompt(Purpose::SaveAs(After::Stay)))),
    ("Quit", Command::Quit),
    ("Undo", Command::Edit(Edit::Undo)),
    ("Redo", Command::Edit(Edit::Redo)),
    ("Cut", Command::Edit(Edit::Cut)),
    ("Copy", Command::Edit(Edit::Copy)),
    ("Paste", Command::Edit(Edit::Paste)),
    ("Indent", Command::Edit(Edit::Indent)),
    ("Outdent", Command::Edit(Edit::Outdent)),
    ("Toggle comment", Command::Edit(Edit::ToggleComment)),
    ("Find", Command::Panel(PanelCommand::Prompt(Purpose::Search))),
    ("Replace", Command::Panel(PanelCommand::Prompt(Purpose::Replace))),
    ("Find next", Command::FindNext(Direction::Forward)),
    ("Find previous", Command::FindNext(Direction::Backward)),
    ("Go to line", Command::Panel(PanelCommand::Prompt(Purpose::GotoLine))),
    ("Select all", Command::Select(Select::All)),
    ("Select line", Command::Select(Select::Line)),
    ("Select next occurrence", Command::Select(Select::NextOccurrence)),
    ("Select all occurrences", Command::Select(Select::AllOccurrences)),
    ("Add cursor above", Command::Select(Select::AddCursor(Direction::Backward))),
    ("Add cursor below", Command::Select(Select::AddCursor(Direction::Forward))),
    ("Set language", Command::Panel(PanelCommand::Picker(Pick::Language))),
];
