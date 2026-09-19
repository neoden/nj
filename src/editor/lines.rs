//! Edits that work on lines rather than on cursors: a new line with the
//! indent of the old, indenting and outdenting, commenting out. Each is a
//! function from the text and the selections to a transaction, so the editor
//! only has to apply what it is handed.

use std::ops::Range;

use crate::buffer::language::Indent;
use crate::text::movement::{Direction, Motion};
use crate::text::selection::{Selection, SelectionSet};
use crate::text::transaction::{self, Change, CursorAfter, Transaction};
use crate::text::Text;

/// A new line starts where the current one starts. Nothing language-aware
/// happens here: opening a block deepens the indent only once there is a
/// parser to say so.
pub fn newline(text: &Text, selections: &SelectionSet) -> Transaction {
    let mut leading = selections
        .first_on_each_line(text, Selection::min)
        .into_iter();

    Transaction::edit(selections, CursorAfter::EndOfChange, |selection| {
        let line = text.line_of(selection.min());
        let start = text.line_start(line);
        let content = text.line_str(line);
        let local = selection.min() - start;

        let indent: String = content[..local]
            .chars()
            .take_while(|character| character.is_whitespace())
            .collect();

        // Taken for every selection, or the flags fall out of step with
        // the cursors they belong to.
        let first = leading.next().unwrap_or(false);
        // A line holding nothing but this indent is left behind empty
        // rather than as a row of spaces.
        let abandoned =
            first && indent.len() == local && content[local..].is_empty() && selection.is_empty();

        let range = match abandoned {
            true => start..selection.max(),
            false => selection.range(),
        };
        (range, format!("\n{indent}"))
    })
}

/// What `Tab` inserts at a bare cursor: enough to reach the next stop, which
/// is not the same as a whole level when the cursor sits mid-line.
pub fn to_tab_stop(indent: Indent, column: usize) -> String {
    match indent {
        Indent::Tabs => "\t".to_owned(),
        Indent::Spaces(width) => " ".repeat(width - column % width),
    }
}

/// One level in front of every touched line.
pub fn indent(text: &Text, selections: &SelectionSet, unit: Indent) -> Transaction {
    let unit = unit.unit();
    let changes = selections
        .touched_lines(text)
        .into_iter()
        .map(|line| {
            let start = text.line_start(line);
            Change {
                range: start..start,
                text: unit.clone(),
            }
        })
        .collect();
    over_lines(text, selections, changes)
}

/// One level, or what there is of one, off the front of every touched line.
/// `None` when no line had anything to lose.
pub fn outdent(text: &Text, selections: &SelectionSet, unit: Indent) -> Option<Transaction> {
    let changes: Vec<Change> = selections
        .touched_lines(text)
        .into_iter()
        .filter_map(|line| {
            let start = text.line_start(line);
            let content = text.line_str(line);
            let removed = match unit {
                Indent::Tabs => content.starts_with('\t').then_some(1),
                Indent::Spaces(width) => match content.starts_with('\t') {
                    true => Some(1),
                    false => match content.chars().take_while(|&c| c == ' ').count() {
                        0 => None,
                        spaces => Some(spaces.min(width)),
                    },
                },
            }?;
            Some(Change {
                range: start..start + removed,
                text: String::new(),
            })
        })
        .collect();

    (!changes.is_empty()).then(|| over_lines(text, selections, changes))
}

/// Comments the touched lines, or uncomments them when every one of them is
/// already commented. Blank lines are left out either way; `None` when there
/// is nothing but blank lines.
pub fn toggle_comment(text: &Text, selections: &SelectionSet, token: &str) -> Option<Transaction> {
    let lines: Vec<usize> = selections
        .touched_lines(text)
        .into_iter()
        .filter(|&line| !text.line_str(line).trim().is_empty())
        .collect();
    if lines.is_empty() {
        return None;
    }

    let commented = lines
        .iter()
        .all(|&line| text.line_str(line).trim_start().starts_with(token));

    let changes = match commented {
        true => lines
            .iter()
            .map(|&line| {
                let start = text.line_start(line);
                let content = text.line_str(line);
                let at = content.len() - content.trim_start().len();
                let rest = &content[at + token.len()..];
                let width = token.len() + usize::from(rest.starts_with(' '));
                Change {
                    range: start + at..start + at + width,
                    text: String::new(),
                }
            })
            .collect(),
        false => {
            // All the markers line up with the shallowest line, which is
            // what keeps a commented-out block readable.
            let column = lines
                .iter()
                .map(|&line| {
                    let content = text.line_str(line);
                    content.len() - content.trim_start().len()
                })
                .min()
                .unwrap_or(0);

            lines
                .iter()
                .map(|&line| {
                    let at = text.line_start(line) + column;
                    Change {
                        range: at..at,
                        text: format!("{token} "),
                    }
                })
                .collect()
        }
    };

    Some(over_lines(text, selections, changes))
}

/// A selection that took part in a line edit ends up covering those lines
/// whole, so the indentation it just gained is inside it rather than hanging
/// off its left edge.
fn over_lines(text: &Text, selections: &SelectionSet, changes: Vec<Change>) -> Transaction {
    let after = selections.mapped(|selection| {
        if selection.is_empty() {
            let head = transaction::displace(selection.head, &changes);
            return Selection::cursor(head);
        }

        let start = text.line_start(text.line_of(selection.min()));
        let end = transaction::displace(selection.max(), &changes);
        match selection.is_reversed() {
            true => Selection {
                anchor: end,
                head: start,
                goal_column: None,
            },
            false => Selection {
                anchor: start,
                head: end,
                goal_column: None,
            },
        }
    });

    Transaction::over_lines(selections, changes, after)
}

/// Backspace inside a run of leading spaces removes back to the previous tab
/// stop, mirroring what Tab does forwards. Anywhere else it is an ordinary
/// deletion, or alignment inside a line would be eaten a level at a time.
pub fn deletion_to_tab_stop(
    text: &Text,
    indent: Indent,
    motion: Motion,
    head: usize,
) -> Option<Range<usize>> {
    let Indent::Spaces(width) = indent else {
        return None;
    };
    if motion != Motion::Grapheme(Direction::Backward) {
        return None;
    }

    let line = text.line_of(head);
    let start = text.line_start(line);
    let local = head - start;
    let content = text.line_str(line);
    let prefix = content.get(..local)?;
    if prefix.is_empty() || !prefix.chars().all(|character| character == ' ') {
        return None;
    }

    let stop = (local - 1) / width * width;
    Some(start + stop..head)
}
