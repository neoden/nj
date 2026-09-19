//! What was copied, and how it goes back in. Kept as one piece per cursor so
//! that a multi-cursor copy can be pasted into the same number of cursors;
//! taken as whole lines when nothing was selected, and put back as such.

use std::ops::Range;

use crate::text::selection::{Selection, SelectionSet};
use crate::text::transaction::{self, Change, CursorAfter, Transaction};
use crate::text::Text;

/// The clipboard outside the editor, whatever reaches it. The editor writes
/// every copy there and, before a paste, asks whether someone else has
/// copied since (S5: a test can stand in for the desktop).
pub trait Desktop {
    fn read(&mut self) -> Option<String>;
    fn write(&mut self, text: &str);
}

#[derive(Debug, Default)]
pub struct Clipboard {
    pieces: Vec<String>,
    /// Copied with nothing selected, meaning whole lines were taken and whole
    /// lines should be put back.
    lines: bool,
}

impl Clipboard {
    pub fn set(&mut self, pieces: Vec<String>, lines: bool) {
        self.pieces = pieces;
        self.lines = lines;
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    pub fn holds_lines(&self) -> bool {
        self.lines
    }

    pub fn text(&self) -> String {
        match self.lines {
            true => self.pieces.concat(),
            false => self.pieces.join("\n"),
        }
    }

    /// Cursor for cursor when the counts agree; otherwise every cursor gets
    /// the whole thing.
    pub fn pieces_for(&self, cursors: usize) -> Vec<String> {
        match self.pieces.len() == cursors {
            true => self.pieces.clone(),
            false => vec![self.text(); cursors],
        }
    }

    /// Takes what the cursors cover. With nothing selected, whole lines are
    /// taken, which is what every editor with this binding does; a line under
    /// several cursors is taken once, however many of them sit on it.
    pub fn copy_from(&mut self, text: &Text, selections: &SelectionSet) {
        let lines = selections.iter().all(Selection::is_empty);
        let pieces = match lines {
            true => selections
                .touched_lines(text)
                .into_iter()
                .map(|line| {
                    let mut piece = text.slice(line_range(text, line)).to_string();
                    if !piece.ends_with('\n') {
                        piece.push('\n');
                    }
                    piece
                })
                .collect(),
            false => selections
                .iter()
                .map(|selection| text.slice(selection.range()).to_string())
                .collect(),
        };
        self.set(pieces, lines);
    }

    /// Removes what `copy_from` just took. Lines go one change per line
    /// rather than per cursor: a line removed twice would take the line after
    /// it as well. `None` when there was nothing to remove.
    pub fn cut_from(&self, text: &Text, selections: &SelectionSet) -> Option<Transaction> {
        if !self.lines {
            let transaction = Transaction::edit(selections, CursorAfter::EndOfChange, |selection| {
                (selection.range(), String::new())
            });
            return (!transaction.is_empty()).then_some(transaction);
        }

        let changes: Vec<Change> = selections
            .touched_lines(text)
            .into_iter()
            .map(|line| Change {
                range: line_range(text, line),
                text: String::new(),
            })
            .collect();
        let after = selections.mapped(|selection| {
            Selection::cursor(transaction::displace(selection.head, &changes))
        });
        Some(Transaction::over_lines(selections, changes, after))
    }

    /// Puts the pieces back. Whole lines land above the line a cursor is on,
    /// so cursors sharing a line share the one insertion, and only that one
    /// takes a piece.
    pub fn paste_into(&self, text: &Text, selections: &SelectionSet) -> Transaction {
        let leading = match self.lines {
            true => selections.first_on_each_line(text, |selection| selection.head),
            false => vec![true; selections.len()],
        };
        let mut pieces = self
            .pieces_for(leading.iter().filter(|&&first| first).count())
            .into_iter();
        let mut leading = leading.into_iter();

        let cursor = match self.lines {
            true => CursorAfter::Displaced,
            false => CursorAfter::EndOfChange,
        };
        Transaction::edit(selections, cursor, |selection| {
            if !leading.next().unwrap_or(true) {
                return (selection.head..selection.head, String::new());
            }
            let range = match self.lines {
                true => {
                    let start = text.line_start(text.line_of(selection.head));
                    start..start
                }
                false => selection.range(),
            };
            (range, pieces.next().unwrap_or_default())
        })
    }
}

/// A whole line with the break after it, which is what a copy takes when
/// nothing is selected.
fn line_range(text: &Text, line: usize) -> Range<usize> {
    let start = text.line_start(line);
    start..(start + text.line(line).len() + 1).min(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clipboard(pieces: &[&str], lines: bool) -> Clipboard {
        let mut clipboard = Clipboard::default();
        clipboard.set(pieces.iter().map(|&piece| piece.to_owned()).collect(), lines);
        clipboard
    }

    #[test]
    fn matching_cursor_counts_get_a_piece_each() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(2), ["one", "two"]);
    }

    #[test]
    fn a_lone_cursor_gets_everything() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(1), ["one\ntwo"]);
    }

    #[test]
    fn mismatched_counts_get_everything_each() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(3), ["one\ntwo", "one\ntwo", "one\ntwo"]);
    }

    #[test]
    fn copied_lines_already_carry_their_breaks() {
        let clipboard = clipboard(&["one\n", "two\n"], true);
        assert_eq!(clipboard.text(), "one\ntwo\n");
    }
}
