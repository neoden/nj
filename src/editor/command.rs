//! Everything the editor can be asked to do, as values. The keymap
//! produces them, the palette will list them, a socket will send them; the
//! editor executes them and nothing else reaches it.
//!
//! Nested by subsystem so the outer match routes and the modules do the
//! work. Nothing here borrows: a command is plain data that can be stored,
//! sent and later parsed from a configuration file.

use std::path::PathBuf;

use super::panel::After;
use super::panel::picker::Pick;
use crate::text::movement::{Direction, Motion};
use crate::text::selection::SelectionSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Goes to the field of an open panel when there is one, to the text
    /// otherwise — like `Edit::Insert`, `Edit::Delete` and `Select::All`.
    Move {
        motion: Motion,
        extend: bool,
    },
    /// Rows; the cursors stay where they are.
    Scroll(isize),
    /// The next hit of the search going on, or of the last one, brought
    /// back. A command of the text: an open panel does not take it.
    FindNext(Direction),
    /// Puts out the highlighted hits, until `FindNext` brings them back.
    /// The selections stay where they are.
    EndSearch,
    Edit(Edit),
    Select(Select),
    File(File),
    Panel(PanelCommand),
    Quit,
    /// Leaves with a failure code and writes nothing, so that whatever is
    /// waiting on the editor — a commit, a pipeline — does not go on.
    Abort,
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
    /// Every hit of the search while one is going on; otherwise every
    /// whole-word occurrence of what is selected.
    AllOccurrences,
    /// Replaces the selections outright: what a socket sends to open a file
    /// at a line, and what a test uses to start from a known place.
    Set(SelectionSet),
    /// In screen cells.
    Click {
        row: usize,
        column: usize,
        kind: ClickKind,
    },
    Drag {
        row: usize,
        column: usize,
    },
}

/// What a click does with the cursors, one of three: the modifiers held
/// decide which, and nothing else about the mouse is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickKind {
    /// A cursor where the click landed, and no other.
    Place,
    /// One more cursor, the others kept — Alt.
    Add,
    /// The selection's head to the click, its anchor kept — Shift.
    Extend,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum File {
    Save,
    /// Under this name from now on. A pipeline buffer writes a copy instead.
    SaveAs(PathBuf),
    /// In place of the buffer in focus, which asks first if it has unsaved
    /// changes. A path with no file behind it opens empty, like `nj newfile`.
    Open(PathBuf),
    /// By name, since a language is a table entry rather than a value.
    SetLanguage(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelCommand {
    /// Opens the search field.
    Find,
    /// `Enter` in the search field: starts the search and goes to its first
    /// hit from the cursor in this direction.
    Search(Direction),
    /// Opens the replace fields.
    Replace,
    /// `Enter` in the replace fields: starts the search and asks at its
    /// first hit.
    StartReplacing,
    /// Opens the line number field.
    GotoLine,
    /// `Enter` in the line number field: goes to the line typed there.
    Jump,
    /// Opens the field a path to open is typed into.
    OpenFile,
    /// `Enter` in that field: opens the file at the path typed there.
    OpenPath,
    /// Opens the field a name to save under is typed into; carries what the
    /// save is for.
    SaveAs(After),
    /// `Enter` in that field: saves under the name typed there.
    SavePath,
    Picker(Pick),
    Close,
    /// Takes the picker's highlighted candidate.
    Pick,
    /// A single-key answer to a question.
    Answer(char),
    /// While a replacement asks at each hit: replaces this one and every
    /// one not answered for yet, which at the first question is all of them.
    ReplaceAll,
    /// While a replacement asks at each hit: replaces this one and goes on.
    ReplaceHit,
    /// While a replacement asks at each hit: leaves this one and goes on.
    SkipHit,
    NextField,
    /// Moves the picker's highlight; positive is away from the query.
    MoveChoice(isize),
}

/// What the palette lists (F2): every command worth asking for by name.
/// Movement is not — nobody opens a palette to go left — and neither is
/// anything that only makes sense inside a panel.
pub const PALETTE: &[(&str, Command)] = &[
    ("Open file", Command::Panel(PanelCommand::OpenFile)),
    ("Save", Command::File(File::Save)),
    ("Save as", Command::Panel(PanelCommand::SaveAs(After::Stay))),
    ("Quit", Command::Quit),
    ("Abort", Command::Abort),
    ("Undo", Command::Edit(Edit::Undo)),
    ("Redo", Command::Edit(Edit::Redo)),
    ("Cut", Command::Edit(Edit::Cut)),
    ("Copy", Command::Edit(Edit::Copy)),
    ("Paste", Command::Edit(Edit::Paste)),
    ("Indent", Command::Edit(Edit::Indent)),
    ("Outdent", Command::Edit(Edit::Outdent)),
    ("Toggle comment", Command::Edit(Edit::ToggleComment)),
    ("Find", Command::Panel(PanelCommand::Find)),
    ("Replace", Command::Panel(PanelCommand::Replace)),
    ("Find next", Command::FindNext(Direction::Forward)),
    ("Find previous", Command::FindNext(Direction::Backward)),
    ("Go to line", Command::Panel(PanelCommand::GotoLine)),
    ("Select all", Command::Select(Select::All)),
    ("Select line", Command::Select(Select::Line)),
    (
        "Select next occurrence",
        Command::Select(Select::NextOccurrence),
    ),
    (
        "Select all occurrences",
        Command::Select(Select::AllOccurrences),
    ),
    (
        "Add cursor above",
        Command::Select(Select::AddCursor(Direction::Backward)),
    ),
    (
        "Add cursor below",
        Command::Select(Select::AddCursor(Direction::Forward)),
    ),
    (
        "Set language",
        Command::Panel(PanelCommand::Picker(Pick::Language)),
    ),
];
