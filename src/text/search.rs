//! What is searched for and how a search in progress keeps its hits. The
//! query is the editor's policy — word boundaries now, case and patterns
//! later (F13) — laid over the buffer's literal `find`, which knows nothing
//! of it. One engine serves the prompt, `Ctrl+D` and `F3`; they differ only
//! in the query they build (S2).

use std::ops::Range;

use super::Text;
use super::movement::{Class, Direction};
use super::selection::{Selection, SelectionSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub needle: String,
    /// Only hits with no word character on either side: renaming `i`
    /// leaves `if` alone.
    pub whole_word: bool,
}

impl Query {
    pub fn literal(needle: impl Into<String>) -> Self {
        Self {
            needle: needle.into(),
            whole_word: false,
        }
    }

    pub fn word(needle: impl Into<String>) -> Self {
        Self {
            needle: needle.into(),
            whole_word: true,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.needle.is_empty()
    }

    /// Every hit, left to right.
    pub fn all<'a>(&'a self, text: &'a Text) -> impl Iterator<Item = Range<usize>> + 'a {
        text.matches(&self.needle)
            .filter(move |hit| self.accepts(text, hit))
    }

    /// The nearest hit in `direction`, wrapping around the text: forward
    /// looks at or after `from`, backward strictly before it.
    pub fn find(&self, text: &Text, from: usize, direction: Direction) -> Option<Range<usize>> {
        if self.is_empty() {
            return None;
        }
        let wrapped = match direction {
            Direction::Forward => 0,
            Direction::Backward => text.len(),
        };
        self.scan(text, from, direction)
            .or_else(|| self.scan(text, wrapped, direction))
    }

    fn scan(&self, text: &Text, from: usize, direction: Direction) -> Option<Range<usize>> {
        let mut at = from;
        loop {
            let start = match direction {
                Direction::Forward => text.find(&self.needle, at)?,
                Direction::Backward => text.rfind(&self.needle, at)?,
            };
            let hit = start..start + self.needle.len();
            if self.accepts(text, &hit) {
                return Some(hit);
            }
            at = match direction {
                Direction::Forward => text.next_grapheme(start),
                Direction::Backward => start,
            };
        }
    }

    /// Whether a hit of this query stands at `at`: what narrowing checks.
    pub fn hits_at(&self, text: &Text, at: usize) -> bool {
        text.matches_at(at, &self.needle) && self.accepts(text, &(at..at + self.needle.len()))
    }

    fn accepts(&self, text: &Text, hit: &Range<usize>) -> bool {
        if !self.whole_word {
            return true;
        }
        let word = |at: usize| text.char_at(at).map(Class::of) == Some(Class::Word);
        let before = text.previous_grapheme(hit.start);
        !(before != hit.start && word(before)) && !word(hit.end)
    }

    pub fn selection(hit: Range<usize>) -> Selection {
        Selection {
            anchor: hit.start,
            head: hit.end,
            goal_column: None,
        }
    }
}

/// A search being typed: the query so far and every hit of it, kept so that
/// a longer query sifts them rather than reading the text again, and so that
/// walking and counting need no second pass.
#[derive(Debug)]
pub struct Session {
    query: Query,
    /// Sorted and disjoint, like the matches they came from.
    hits: Vec<Range<usize>>,
    /// Where the cursors stood when the search opened, so that typing starts
    /// over from there and `Esc` can put them back.
    pub origin: SelectionSet,
}

impl Session {
    pub fn new(origin: SelectionSet) -> Self {
        Self {
            query: Query::literal(""),
            hits: Vec::new(),
            origin,
        }
    }

    pub fn query(&self) -> &Query {
        &self.query
    }

    pub fn hits(&self) -> &[Range<usize>] {
        &self.hits
    }

    /// Collects the hits of `query`. Hits of a longer needle are a subset of
    /// the shorter one's, so extending the query sifts what is there.
    pub fn retarget(&mut self, query: Query, text: &Text) {
        let narrows = !self.query.is_empty()
            && query.whole_word == self.query.whole_word
            && query.needle.starts_with(&self.query.needle);

        match narrows {
            true => {
                // Grown hits may reach into their neighbours; a scan would
                // have skipped those, so sifting skips them too.
                let mut taken = 0;
                self.hits.retain_mut(|hit| {
                    let stands = hit.start >= taken && query.hits_at(text, hit.start);
                    hit.end = hit.start + query.needle.len();
                    if stands {
                        taken = hit.end;
                    }
                    stands
                });
            }
            false => self.hits = query.all(text).collect(),
        }
        self.query = query;
    }

    /// The first hit at or after `from`, or the first of all: where an
    /// incremental search lands so that the match never drifts as the query
    /// grows.
    pub fn landing(&self, from: usize) -> Option<Range<usize>> {
        self.hits
            .iter()
            .find(|hit| hit.start >= from)
            .or_else(|| self.hits.first())
            .cloned()
    }

    /// The next hit in `direction` from `from`, wrapping: forward at or
    /// after, backward strictly before — the same rule as `Query::find`, over
    /// the hits already in hand.
    pub fn next_hit(&self, from: usize, direction: Direction) -> Option<Range<usize>> {
        match direction {
            Direction::Forward => self
                .hits
                .iter()
                .find(|hit| hit.start >= from)
                .or_else(|| self.hits.first()),
            Direction::Backward => self
                .hits
                .iter()
                .rev()
                .find(|hit| hit.start < from)
                .or_else(|| self.hits.last()),
        }
        .cloned()
    }

    /// Every hit as a selection, with the one at `at` primary.
    pub fn selections(&self, at: usize) -> SelectionSet {
        let primary = self
            .hits
            .iter()
            .position(|hit| hit.start == at)
            .unwrap_or(0);
        let selections = self.hits.iter().cloned().map(Query::selection).collect();
        SelectionSet::from_selections(selections, primary)
    }

    /// `Found: 3/17`, counting the hit at `at` as the current one.
    pub fn status(&self, at: usize) -> String {
        if self.hits.is_empty() {
            return "Found: none".to_owned();
        }
        let current = self
            .hits
            .iter()
            .position(|hit| hit.start == at)
            .map_or(0, |index| index + 1);
        format!("Found: {current}/{}", self.hits.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(content: &str) -> Text {
        Text::from_str(content)
    }

    #[test]
    fn a_literal_query_finds_every_occurrence() {
        let text = text("a ab a");
        let hits: Vec<_> = Query::literal("a").all(&text).collect();
        assert_eq!(hits, [0..1, 2..3, 5..6]);
    }

    #[test]
    fn a_word_query_leaves_longer_words_alone() {
        let text = text("a ab a ba");
        let hits: Vec<_> = Query::word("a").all(&text).collect();
        assert_eq!(hits, [0..1, 5..6]);
    }

    #[test]
    fn finding_wraps_in_both_directions() {
        let text = text("x . x . x");
        let query = Query::literal("x");

        assert_eq!(query.find(&text, 1, Direction::Forward), Some(4..5));
        assert_eq!(query.find(&text, 9, Direction::Forward), Some(0..1));
        assert_eq!(query.find(&text, 4, Direction::Backward), Some(0..1));
        assert_eq!(query.find(&text, 0, Direction::Backward), Some(8..9));
    }

    #[test]
    fn a_word_query_skips_over_partial_hits_while_scanning() {
        let text = text("ab a ab");
        let query = Query::word("a");
        assert_eq!(query.find(&text, 0, Direction::Forward), Some(3..4));
        assert_eq!(query.find(&text, 7, Direction::Backward), Some(3..4));
        assert_eq!(
            Query::word("ab").find(&text, 1, Direction::Forward),
            Some(5..7)
        );
    }

    #[test]
    fn an_empty_query_finds_nothing() {
        let text = text("anything");
        assert_eq!(Query::literal("").find(&text, 0, Direction::Forward), None);
        assert_eq!(Query::literal("").all(&text).count(), 0);
    }

    #[test]
    fn a_session_narrows_as_the_query_grows() {
        let text = text("ab abc ab abcd");
        let mut session = Session::new(SelectionSet::single(0));

        session.retarget(Query::literal("ab"), &text);
        assert_eq!(session.hits(), [0..2, 3..5, 7..9, 10..12]);

        session.retarget(Query::literal("abc"), &text);
        assert_eq!(session.hits(), [3..6, 10..13]);

        session.retarget(Query::literal("abcd"), &text);
        assert_eq!(session.hits(), [10..14]);

        // Backspace cannot narrow, so this one has to read the text again.
        session.retarget(Query::literal("abc"), &text);
        assert_eq!(session.hits(), [3..6, 10..13]);
    }

    /// Sifting must end with the same hits a fresh scan would find, which
    /// for a needle that overlaps itself means dropping every other one.
    #[test]
    fn narrowing_keeps_the_hits_disjoint() {
        let text = text("aaaaa");
        let mut session = Session::new(SelectionSet::single(0));
        session.retarget(Query::literal("a"), &text);
        session.retarget(Query::literal("aa"), &text);

        let scanned: Vec<_> = Query::literal("aa").all(&text).collect();
        assert_eq!(session.hits(), [0..2, 2..4]);
        assert_eq!(session.hits(), scanned.as_slice());
    }

    #[test]
    fn a_session_walks_its_hits_and_counts_them() {
        let text = text("a a a");
        let mut session = Session::new(SelectionSet::single(0));
        session.retarget(Query::literal("a"), &text);

        assert_eq!(session.landing(1), Some(2..3));
        assert_eq!(session.landing(5), Some(0..1));
        assert_eq!(session.next_hit(3, Direction::Forward), Some(4..5));
        assert_eq!(session.next_hit(5, Direction::Forward), Some(0..1));
        assert_eq!(session.next_hit(2, Direction::Backward), Some(0..1));
        assert_eq!(session.next_hit(0, Direction::Backward), Some(4..5));

        assert_eq!(session.status(2), "Found: 2/3");
        assert_eq!(session.status(1), "Found: 0/3");
        assert_eq!(session.selections(2).primary().range(), 2..3);
        assert_eq!(session.selections(2).len(), 3);

        session.retarget(Query::literal("zz"), &text);
        assert_eq!(session.status(0), "Found: none");
    }
}
