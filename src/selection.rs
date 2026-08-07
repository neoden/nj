use std::ops::Range;

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

    pub fn len(&self) -> usize {
        self.selections.len()
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

    pub fn push(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.primary = self.selections.len() - 1;
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

    fn normalize(&mut self) {
        let primary_head = self.selections[self.primary].head;
        self.selections.sort_by_key(|s| (s.min(), s.max()));

        let mut merged: Vec<Selection> = Vec::with_capacity(self.selections.len());
        for selection in self.selections.drain(..) {
            match merged.last_mut() {
                Some(last) if overlap(last, &selection) => *last = union(last, &selection),
                _ => merged.push(selection),
            }
        }

        self.primary = merged
            .iter()
            .position(|s| s.contains(primary_head))
            .unwrap_or(0);
        self.selections = merged;
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
    let reversed = if a.is_empty() { b.is_reversed() } else { a.is_reversed() };
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
    fn the_primary_follows_its_cursor_through_normalization() {
        let mut set = SelectionSet::single(0);
        set.push(selection(20, 25));
        assert_eq!(set.primary(), selection(20, 25));

        set.push(selection(10, 12));
        assert_eq!(set.primary(), selection(10, 12));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn escape_leaves_one_cursor() {
        let mut set = SelectionSet::single(0);
        set.push(selection(20, 25));
        set.collapse_to_primary();
        assert_eq!(set.len(), 1);
        assert_eq!(set.primary(), selection(20, 25));
    }
}
