use std::ops::Range;

use super::Text;

/// A cursor with a tail. `anchor` is where selecting started, `head` is where
/// the cursor is now; a collapsed selection has them equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
    /// Column the cursor wants to be in while moving vertically, so that
    /// crossing a short line does not lose its horizontal position.
    pub goal_column: Option<usize>,
}

impl Selection {
    pub fn cursor(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
            goal_column: None,
        }
    }

    pub fn range(&self) -> Range<usize> {
        self.min()..self.max()
    }

    pub fn min(&self) -> usize {
        self.anchor.min(self.head)
    }

    pub fn max(&self) -> usize {
        self.anchor.max(self.head)
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    pub fn is_reversed(&self) -> bool {
        self.head < self.anchor
    }

    pub fn contains(&self, offset: usize) -> bool {
        (self.min()..=self.max()).contains(&offset)
    }
}

/// Every cursor in one view. Always non-empty, always sorted, never
/// overlapping: commands may produce any set they like and normalization
/// restores those invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionSet {
    selections: Vec<Selection>,
    primary: usize,
}

impl From<Selection> for SelectionSet {
    fn from(selection: Selection) -> Self {
        Self {
            selections: vec![selection],
            primary: 0,
        }
    }
}

impl SelectionSet {
    pub fn single(offset: usize) -> Self {
        Selection::cursor(offset).into()
    }

    pub fn from_selections(selections: Vec<Selection>, primary: usize) -> Self {
        if selections.is_empty() {
            return Self::single(0);
        }
        let mut set = Self {
            primary: primary.min(selections.len() - 1),
            selections,
        };
        set.normalize();
        set
    }

    /// Never zero: a set always holds at least one selection, so there is
    /// no `is_empty` to go with this.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.selections.len()
    }

    /// Takes one selection out — never the primary, which is the caller's
    /// to keep: the column of cursors shrinks towards it, not past it.
    pub fn remove(&mut self, index: usize) {
        debug_assert!(index != self.primary, "the primary is not removed");
        self.selections.remove(index);
        if index < self.primary {
            self.primary -= 1;
        }
    }

    pub fn primary_index(&self) -> usize {
        self.primary
    }

    pub fn primary(&self) -> Selection {
        self.selections[self.primary]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Selection> {
        self.selections.iter()
    }

    /// The selections touching a byte range, found rather than scanned for:
    /// the renderer asks once per drawn line.
    pub fn touching(&self, range: Range<usize>) -> &[Selection] {
        let first = self
            .selections
            .partition_point(|selection| selection.max() < range.start);
        let rest = &self.selections[first..];
        let len = rest.partition_point(|selection| selection.min() <= range.end);
        &rest[..len]
    }

    /// Adds a selection beside the others. The primary stays what it was:
    /// it is the cursor the person placed, and what a command adds around
    /// it — the next occurrence, a cursor on the row below — does not take
    /// that over, so `Esc` always comes back to it.
    pub fn push(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.normalize();
    }

    /// Moves every cursor at once, which is what every motion command does.
    /// Returns a new set rather than mutating, so the caller may read whatever
    /// the closure captures.
    pub fn mapped(&self, f: impl FnMut(Selection) -> Selection) -> Self {
        let mut moved = Self {
            selections: self.selections.iter().copied().map(f).collect(),
            primary: self.primary,
        };
        moved.normalize();
        moved
    }

    pub fn collapse_to_primary(&mut self) {
        let primary = self.primary();
        self.selections = vec![primary];
        self.primary = 0;
    }

    pub fn collapse_to_cursors(&mut self) {
        *self = self.mapped(|selection| Selection::cursor(selection.head));
    }

    /// Lines under any cursor, each named once however many cursors sit on
    /// it. A selection ending exactly on a line start does not reach that
    /// line — it only swallowed the break before it.
    pub fn touched_lines(&self, text: &Text) -> Vec<usize> {
        let mut lines: Vec<usize> = self
            .selections
            .iter()
            .flat_map(|selection| {
                let first = text.line_of(selection.min());
                let mut last = text.line_of(selection.max());
                if last > first && text.line_start(last) == selection.max() {
                    last -= 1;
                }
                first..=last
            })
            .collect();
        lines.sort_unstable();
        lines.dedup();
        lines
    }

    /// Whether each selection is the first one on its line, taking the line
    /// from the offset `by` picks, so that a line edit happens once however
    /// many cursors sit on it.
    pub fn first_on_each_line(&self, text: &Text, by: impl Fn(&Selection) -> usize) -> Vec<bool> {
        let mut previous = None;
        self.selections
            .iter()
            .map(|selection| {
                let line = text.line_of(by(selection));
                let first = previous != Some(line);
                previous = Some(line);
                first
            })
            .collect()
    }

    /// The primary is marked before sorting and the mark travels through
    /// the merge, since looking it up again by its head would be ambiguous
    /// where two selections meet.
    fn normalize(&mut self) {
        let mut tagged: Vec<(Selection, bool)> = self
            .selections
            .drain(..)
            .enumerate()
            .map(|(index, selection)| (selection, index == self.primary))
            .collect();
        tagged.sort_by_key(|(s, _)| (s.min(), s.max()));

        let mut merged: Vec<(Selection, bool)> = Vec::with_capacity(tagged.len());
        for (selection, primary) in tagged {
            match merged.last_mut() {
                Some((last, was_primary)) if overlap(last, &selection) => {
                    *last = union(last, &selection);
                    *was_primary |= primary;
                }
                _ => merged.push((selection, primary)),
            }
        }

        self.primary = merged
            .iter()
            .position(|(_, primary)| *primary)
            .expect("the primary is one of the selections");
        self.selections = merged.into_iter().map(|(s, _)| s).collect();
    }
}

/// Selections that merely touch stay apart, because two cursors at a shared
/// boundary are two editing points. A collapsed cursor sitting on the edge of
/// a selection is redundant, though, so that one is absorbed.
fn overlap(a: &Selection, b: &Selection) -> bool {
    b.min() < a.max() || (b.min() == a.max() && (a.is_empty() || b.is_empty()))
}

fn union(a: &Selection, b: &Selection) -> Selection {
    let (min, max) = (a.min().min(b.min()), a.max().max(b.max()));
    let reversed = if a.is_empty() {
        b.is_reversed()
    } else {
        a.is_reversed()
    };
    Selection {
        anchor: if reversed { max } else { min },
        head: if reversed { min } else { max },
        goal_column: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(anchor: usize, head: usize) -> Selection {
        Selection {
            anchor,
            head,
            goal_column: None,
        }
    }

    fn set(selections: &[Selection]) -> SelectionSet {
        let mut set = SelectionSet::single(selections[0].head);
        set.selections = selections.to_vec();
        set.normalize();
        set
    }

    #[test]
    fn a_reversed_selection_still_knows_its_range() {
        let backwards = selection(10, 4);
        assert_eq!(backwards.range(), 4..10);
        assert!(backwards.is_reversed());
    }

    #[test]
    fn selections_are_kept_sorted() {
        let set = set(&[selection(20, 25), selection(0, 5)]);
        assert_eq!(set.len(), 2);
        assert_eq!(set.iter().next(), Some(&selection(0, 5)));
    }

    #[test]
    fn overlapping_selections_merge() {
        let set = set(&[selection(0, 6), selection(4, 10)]);
        assert_eq!(set.len(), 1);
        assert_eq!(set.primary(), selection(0, 10));
    }

    #[test]
    fn touching_selections_stay_apart() {
        let set = set(&[selection(0, 5), selection(5, 10)]);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn a_cursor_on_the_edge_of_a_selection_is_absorbed() {
        assert_eq!(set(&[selection(0, 5), selection(5, 5)]).len(), 1);
        assert_eq!(set(&[selection(3, 3), selection(3, 8)]).len(), 1);
    }

    #[test]
    fn duplicate_cursors_collapse() {
        assert_eq!(set(&[selection(7, 7), selection(7, 7)]).len(), 1);
    }

    #[test]
    fn merging_keeps_the_direction_of_the_selection() {
        let set = set(&[selection(10, 4), selection(8, 14)]);
        assert_eq!(set.len(), 1);
        assert_eq!(set.primary(), selection(14, 4));
    }

    #[test]
    fn pushing_adds_a_selection_without_making_it_primary() {
        let mut set = SelectionSet::single(30);
        set.push(selection(20, 25));
        assert_eq!(set.primary(), selection(30, 30));

        set.push(selection(10, 12));
        assert_eq!(set.primary(), selection(30, 30));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn the_primary_follows_its_cursor_through_normalization() {
        let set = set(&[selection(20, 25), selection(0, 5)]);
        assert_eq!(set.primary(), selection(20, 25));
        assert_eq!(set.iter().next(), Some(&selection(0, 5)));
    }

    /// Two selections meeting at one offset both contain it, so a head on
    /// the seam must not be how the primary is found.
    #[test]
    fn the_primary_survives_a_shared_boundary() {
        let mut set = SelectionSet::single(5);
        set.push(selection(3, 3));
        set = set.mapped(|s| selection(s.anchor, s.head - 2));
        assert_eq!(set.primary(), selection(5, 3));

        let mut set = SelectionSet::from_selections(vec![selection(4, 2), selection(0, 2)], 1);
        set.push(selection(0, 0));
        assert_eq!(set.primary(), selection(0, 2));
    }

    #[test]
    fn removing_secondary_cursors_keeps_the_primary_selection() {
        let mut set: SelectionSet = selection(20, 25).into();
        set.push(selection(0, 5));
        set.collapse_to_primary();

        assert_eq!(set.len(), 1);
        assert_eq!(set.primary(), selection(20, 25));
    }

    #[test]
    fn cancelling_a_selection_leaves_a_cursor_at_its_head() {
        let mut set = set(&[selection(4, 10), selection(20, 15)]);
        set.collapse_to_cursors();

        let heads: Vec<_> = set.iter().map(|selection| selection.head).collect();
        assert_eq!(heads, [10, 15]);
        assert!(set.iter().all(Selection::is_empty));
    }
}
