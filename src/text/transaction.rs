use std::ops::Range;

use super::anchor::{self, Bias, Splice};
use super::selection::{Selection, SelectionSet};
use super::Text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub range: Range<usize>,
    pub text: String,
}

/// One atomic edit: every cursor's change, plus where the cursors stood before
/// and after it. Typing, pasting and multi-cursor edits are all this.
#[derive(Debug, Clone)]
pub struct Transaction {
    changes: Vec<Change>,
    before: SelectionSet,
    after: SelectionSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorAfter {
    /// Where the change ends, which is where typing leaves the cursor.
    EndOfChange,
    /// Where the cursor already was, displaced by the change. Only meaningful
    /// for changes that insert entirely before it, such as pasting a line.
    Displaced,
}

impl Transaction {
    /// Builds one change per cursor. Selections are sorted and disjoint, so
    /// the running delta gives each cursor its position in the new text.
    pub fn edit(
        selections: &SelectionSet,
        cursor: CursorAfter,
        mut change: impl FnMut(&Selection) -> (Range<usize>, String),
    ) -> Self {
        let mut changes = Vec::with_capacity(selections.len());
        let mut heads = Vec::with_capacity(selections.len());
        let mut delta = 0isize;

        for selection in selections.iter() {
            let (range, text) = change(selection);
            let own = text.len() as isize - range.len() as isize;
            let head = match cursor {
                CursorAfter::EndOfChange => range.start.saturating_add_signed(delta) + text.len(),
                CursorAfter::Displaced => selection.head.saturating_add_signed(delta + own),
            };
            delta += own;

            heads.push(Selection::cursor(head));
            changes.push(Change { range, text });
        }

        Self {
            changes,
            before: selections.clone(),
            after: SelectionSet::from_selections(heads, selections.primary_index()),
        }
    }

    /// For edits that are about lines rather than cursors, such as indenting
    /// a selection. Where the cursors end up is the caller's decision, since
    /// only it knows what the lines were supposed to become.
    pub fn over_lines(before: &SelectionSet, changes: Vec<Change>, after: SelectionSet) -> Self {
        Self {
            changes,
            before: before.clone(),
            after,
        }
    }

    pub fn selections_before(&self) -> &SelectionSet {
        &self.before
    }

    pub fn selections_after(&self) -> &SelectionSet {
        &self.after
    }

    pub fn is_empty(&self) -> bool {
        self.changes.iter().all(|change| change.range.is_empty() && change.text.is_empty())
    }

    /// The geometry of the edit, which is what the buffer's journal keeps.
    pub fn splices(&self) -> Vec<Splice> {
        self.changes
            .iter()
            .map(|change| Splice {
                range: change.range.clone(),
                len: change.text.len(),
            })
            .collect()
    }

    /// Applies the edit and returns the transaction that undoes it.
    pub fn apply(self, text: &mut Text) -> Self {
        // Back to front, so that offsets ahead of each change stay valid.
        let mut removed = Vec::with_capacity(self.changes.len());
        for change in self.changes.iter().rev() {
            removed.push(text.substring(change.range.clone()));
            text.remove(change.range.clone());
            text.insert(change.range.start, &change.text);
        }
        removed.reverse();

        // The inverse addresses the new text, where every change is displaced
        // by what the changes before it inserted or removed.
        let mut delta = 0isize;
        let changes = self
            .changes
            .iter()
            .zip(removed)
            .map(|(change, removed)| {
                let start = change.range.start.saturating_add_signed(delta);
                delta += change.text.len() as isize - change.range.len() as isize;
                Change {
                    range: start..start + change.text.len(),
                    text: removed,
                }
            })
            .collect();

        Self {
            changes,
            before: self.after,
            after: self.before,
        }
    }
}

/// Where a cursor ends up once these changes have been applied: an insertion
/// at its position lands before it, and a range replaced around it collapses
/// it to the end of what took its place.
pub fn displace(position: usize, changes: &[Change]) -> usize {
    let splices = changes.iter().map(|change| Splice {
        range: change.range.clone(),
        len: change.text.len(),
    });
    anchor::displace(position, splices, Bias::After)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert(selections: &SelectionSet, inserted: &str) -> Transaction {
        Transaction::edit(selections, CursorAfter::EndOfChange, |selection| {
            (selection.range(), inserted.to_owned())
        })
    }

    #[test]
    fn inserting_moves_the_cursor_past_what_was_typed() {
        let mut text = Text::from_str("ac");
        let transaction = insert(&SelectionSet::single(1), "b");

        let selections = transaction.selections_after().clone();
        transaction.apply(&mut text);

        assert_eq!(text.to_string(), "abc");
        assert_eq!(selections.primary().head, 2);
    }

    #[test]
    fn every_cursor_edits_and_later_ones_shift() {
        let mut text = Text::from_str("a a a");
        let mut selections = SelectionSet::single(0);
        selections.push(Selection::cursor(2));
        selections.push(Selection::cursor(4));

        let transaction = insert(&selections, "xy");
        let after = transaction.selections_after().clone();
        transaction.apply(&mut text);

        assert_eq!(text.to_string(), "xya xya xya");
        let heads: Vec<_> = after.iter().map(|selection| selection.head).collect();
        assert_eq!(heads, [2, 6, 10]);
    }

    #[test]
    fn typing_over_a_selection_replaces_it() {
        let mut text = Text::from_str("hello world");
        let selections = SelectionSet::from_selections(
            vec![Selection {
                anchor: 0,
                head: 5,
                goal_column: None,
            }],
            0,
        );

        let transaction = insert(&selections, "bye");
        let after = transaction.selections_after().clone();
        transaction.apply(&mut text);

        assert_eq!(text.to_string(), "bye world");
        assert_eq!(after.primary().head, 3);
    }

    #[test]
    fn undoing_restores_the_text_and_the_cursors() {
        let mut text = Text::from_str("a a");
        let mut selections = SelectionSet::single(1);
        selections.push(Selection::cursor(3));

        let transaction = insert(&selections, "!!");
        let undo = transaction.apply(&mut text);
        assert_eq!(text.to_string(), "a!! a!!");

        let restored = undo.selections_after().clone();
        let redo = undo.apply(&mut text);
        assert_eq!(text.to_string(), "a a");
        assert_eq!(
            restored.iter().copied().collect::<Vec<_>>(),
            selections.iter().copied().collect::<Vec<_>>()
        );

        redo.apply(&mut text);
        assert_eq!(text.to_string(), "a!! a!!");
    }

    #[test]
    fn deleting_reports_what_it_removed() {
        let mut text = Text::from_str("abcd");
        let transaction = Transaction::edit(&SelectionSet::single(0), CursorAfter::EndOfChange, |_| {
            (1..3, String::new())
        });

        let undo = transaction.apply(&mut text);
        assert_eq!(text.to_string(), "ad");

        undo.apply(&mut text);
        assert_eq!(text.to_string(), "abcd");
    }
}
