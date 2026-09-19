//! Positions that survive edits (ADR 18). A byte offset addresses one
//! version of the text; an anchor is that offset together with the version it
//! came from, and resolving it means running it forward through everything
//! that happened since. Nothing here knows about text — only about what was
//! replaced by how much.

use std::ops::Range;

/// A version of a buffer's text; goes up by one per applied transaction.
pub type Version = u64;

/// One replacement, in the coordinates of the text it was applied to: the
/// range that went, and the length of what took its place. What the journal
/// holds instead of the text itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Splice {
    pub range: Range<usize>,
    pub len: usize,
}

impl Splice {
    fn shift(&self) -> isize {
        self.len as isize - self.range.len() as isize
    }
}

/// What happens to an anchor that an edit lands exactly on: an insertion at
/// its position leaves it before the new text or after it, and a replacement
/// covering it collapses it to the start of the change or the end. A cursor
/// wants `After`; the start of an excerpt wants `Before`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bias {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub version: Version,
    pub offset: usize,
    pub bias: Bias,
}

/// Where `position` ends up once `splices` have been applied, in order.
/// Each splice is in the coordinates of the text before it, as a transaction's
/// changes are, so a running shift carries the position across them.
pub fn displace(position: usize, splices: impl IntoIterator<Item = Splice>, bias: Bias) -> usize {
    let mut shift = 0isize;
    for splice in splices {
        let range = &splice.range;
        if range.start > position {
            break;
        }
        // Past the edit: carried along with everything after it. An empty
        // range at the position is an insertion right here, which is not
        // "past" — it is the case the bias decides.
        if range.end < position || (range.end == position && !range.is_empty()) {
            shift += splice.shift();
            continue;
        }
        // On or inside the edit.
        let start = range.start.saturating_add_signed(shift);
        return match bias {
            Bias::Before => start,
            Bias::After => start + splice.len,
        };
    }
    position.saturating_add_signed(shift)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn splice(range: Range<usize>, len: usize) -> Splice {
        Splice { range, len }
    }

    fn after(position: usize, splices: &[Splice]) -> usize {
        displace(position, splices.iter().cloned(), Bias::After)
    }

    fn before(position: usize, splices: &[Splice]) -> usize {
        displace(position, splices.iter().cloned(), Bias::Before)
    }

    #[test]
    fn an_edit_ahead_leaves_the_position_alone() {
        let edits = [splice(5..7, 3)];
        assert_eq!(after(2, &edits), 2);
        assert_eq!(before(2, &edits), 2);
    }

    #[test]
    fn an_edit_behind_carries_the_position_along() {
        assert_eq!(after(10, &[splice(2..4, 5)]), 13);
        assert_eq!(after(10, &[splice(2..4, 0)]), 8);
        assert_eq!(before(10, &[splice(2..4, 0)]), 8);
    }

    #[test]
    fn an_insertion_at_the_position_is_what_the_bias_decides() {
        let insertion = [splice(4..4, 3)];
        assert_eq!(before(4, &insertion), 4);
        assert_eq!(after(4, &insertion), 7);
    }

    #[test]
    fn a_position_inside_a_removed_range_collapses_to_its_edge() {
        let deletion = [splice(2..6, 0)];
        assert_eq!(before(4, &deletion), 2);
        assert_eq!(after(4, &deletion), 2);

        // Replaced rather than removed: the edges are those of the new text.
        let replacement = [splice(2..6, 3)];
        assert_eq!(before(4, &replacement), 2);
        assert_eq!(after(4, &replacement), 5);
        assert_eq!(before(2, &replacement), 2);
        assert_eq!(after(2, &replacement), 5);
    }

    #[test]
    fn the_end_of_a_removed_range_is_past_the_edit() {
        let replacement = [splice(2..6, 3)];
        assert_eq!(before(6, &replacement), 5);
        assert_eq!(after(6, &replacement), 5);
    }

    /// The splices of one transaction all address the text before it, and
    /// are sorted and disjoint, as its changes are.
    #[test]
    fn the_edits_of_one_transaction_accumulate_their_shifts() {
        // "abcdef": insert "Y" at 0, "XX" at 1, and remove the "d".
        let edits = [splice(0..0, 1), splice(1..1, 2), splice(3..4, 0)];
        // 'f' sat at 5 and is past all three: +1, +2, -1.
        assert_eq!(after(5, &edits), 7);
        // 'd' collapses to where it was, carried by the two before it.
        assert_eq!(after(3, &edits), 6);
        assert_eq!(before(3, &edits), 6);
    }
}
