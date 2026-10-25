//! Edits that work on lines rather than on cursors: a new line with the
//! indent of the old, indenting and outdenting, commenting out. Each is a
//! function from the text and the selections to a transaction, so the editor
//! only has to apply what it is handed.

use std::ops::Range;

use crate::buffer::language::Indent;
use crate::text::Text;
use crate::text::layout::{self, indentation};
use crate::text::movement::{Direction, Motion};
use crate::text::selection::{Selection, SelectionSet};
use crate::text::transaction::{self, Change, CursorAfter, Transaction};

/// A new line starts where the current one starts. Nothing language-aware
/// happens here: opening a block deepens the indent only once there is a
/// parser to say so.
pub fn newline(text: &Text, selections: &SelectionSet) -> Transaction {
    let leading = selections.first_on_each_line(text, Selection::min);

    let changes = selections.iter().zip(leading).map(|(selection, first)| {
        let line = text.line_of(selection.min());
        let start = text.line_start(line);
        let content = text.line_str(line);
        let local = selection.min() - start;

        // Only as much of the indentation as lies before the cursor; the
        // rest stays with the text that follows.
        let lead = indentation(&content);
        let indent = &lead[..local.min(lead.len())];

        // What would stay behind on the old line is dropped when it is
        // nothing but indent, so no line is left holding a row of spaces —
        // whether Enter came at the end of such a line or inside the indent
        // of a longer one. By the first cursor on the line only.
        let abandoned = first && indent.len() == local && selection.is_empty();

        let range = match abandoned {
            true => start..selection.max(),
            false => selection.range(),
        };
        (range, format!("\n{indent}"))
    });
    Transaction::over(selections, CursorAfter::EndOfChange, changes)
}

/// What `Tab` inserts at a bare cursor: enough to reach the next stop, which
/// is not the same as a whole level when the cursor sits mid-line.
fn to_tab_stop(indent: Indent, column: usize) -> String {
    match indent {
        Indent::Tabs => "\t".to_owned(),
        Indent::Spaces(width) => " ".repeat(width - column % width),
    }
}

/// `Tab` at bare cursors: every cursor moves to the next tab stop. The new
/// line is printed in the head, cursor to cursor, since a cursor's column
/// depends on what the cursors before it on the line have inserted.
pub fn tab_stops(
    text: &Text,
    selections: &SelectionSet,
    indent: Indent,
    tab_width: usize,
) -> Transaction {
    let first = selections.first_on_each_line(text, |selection| selection.head);

    // Where printing has got to: the offset in the old text, and the column
    // in the new one.
    let mut printed = 0;
    let mut column = 0;
    let mut changes = Vec::with_capacity(selections.len());

    for (selection, first) in selections.iter().zip(first) {
        if first {
            printed = text.line_start(text.line_of(selection.head));
            column = 0;
        }
        column = layout::advance(column, &text.substring(printed..selection.head), tab_width);
        let fill = to_tab_stop(indent, column);
        column = layout::advance(column, &fill, tab_width);
        printed = selection.head;

        changes.push((selection.range(), fill));
    }
    Transaction::over(selections, CursorAfter::EndOfChange, changes)
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
            let lead = indentation(&content);
            let removed = match unit {
                Indent::Tabs => lead.starts_with('\t').then_some(1),
                Indent::Spaces(width) => match lead.starts_with('\t') {
                    true => Some(1),
                    false => match lead.bytes().take_while(|&c| c == b' ').count() {
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
pub fn toggle_comment(
    text: &Text,
    selections: &SelectionSet,
    token: &str,
    tab_width: usize,
) -> Option<Transaction> {
    let lines: Vec<usize> = selections
        .touched_lines(text)
        .into_iter()
        .filter(|&line| !text.line_str(line).trim().is_empty())
        .collect();
    if lines.is_empty() {
        return None;
    }

    let commented = lines.iter().all(|&line| {
        let content = text.line_str(line);
        content[indentation(&content).len()..].starts_with(token)
    });

    let changes = match commented {
        true => lines
            .iter()
            .map(|&line| {
                let start = text.line_start(line);
                let content = text.line_str(line);
                let at = indentation(&content).len();
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
            // what keeps a commented-out block readable. Lined up in display
            // columns: a tab and two spaces are not the same depth, and a
            // marker must not land inside another line's indentation.
            let column = lines
                .iter()
                .map(|&line| layout::line_width(indentation(&text.line_str(line)), tab_width))
                .min()
                .unwrap_or(0);

            lines
                .iter()
                .map(|&line| {
                    let content = text.line_str(line);
                    let at = text.line_start(line)
                        + layout::byte_at_column(indentation(&content), column, tab_width);
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
