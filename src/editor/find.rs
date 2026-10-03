//! Finding things in the text: the search a prompt types, walking its hits,
//! repeating it once the prompt is gone, and the selection-driven kind —
//! `Ctrl+D` and `Ctrl+Shift+L` — that starts from the word under the cursor.
//! One engine (`text::search`) serves them all; what differs is the query
//! and where the hits go.

use std::ops::Range;

use super::Editor;
use crate::buffer::EditKind;
use crate::text::movement::{self, Direction};
use crate::text::search::{Query, Session};
use crate::text::selection::{Selection, SelectionSet};

/// More cursors than this and every keystroke is an edit of that many
/// places. A command that would put a cursor on every hit refuses past it,
/// rather than quietly taking some of the hits (P10).
pub(super) const MAX_CURSORS: usize = 10_000;

fn too_many_hits() -> String {
    format!("More than {MAX_CURSORS} occurrences; select fewer")
}

/// The cursors a run of `Ctrl+D` left behind, so the next press can tell
/// whether it is the same run.
pub(super) struct Occurrences {
    selections: SelectionSet,
    whole_word: bool,
}

impl Editor {
    /// Incremental search: every keystroke retargets the session and lands
    /// on the first hit at or after where the cursor stood when the prompt
    /// opened, so the match never drifts forward as the query grows.
    pub(super) fn prompt_changed(&mut self) {
        // The search is there with any prompt open, and only a search prompt
        // has a query to give it.
        let Some(prompt) = self.prompt().filter(|prompt| prompt.purpose.searches()) else {
            return;
        };
        let needle = prompt.contents(0);
        let origin = prompt.origin.clone();
        let Some(search) = &mut self.search else {
            return;
        };

        search.retarget(Query::literal(needle), self.workspace.focus().1.text());

        match search.landing(origin.primary().min()) {
            Some(hit) => self.select_hit(hit),
            None => self.view_mut().selections = origin,
        }
    }

    /// Turns the hits into cursors and hands the keyboard back to the text,
    /// which is the only reason anyone wants a cursor on every hit. The
    /// search is over with that: its hits are what is selected now.
    pub(super) fn select_all_matches(&mut self) {
        let Some(search) = &self.search else {
            return;
        };
        if search.hits().is_empty() {
            return;
        }
        if search.hits().len() > MAX_CURSORS {
            self.message = Some(too_many_hits());
            return;
        }

        let selections = search.selections(self.view().selections.primary().min());
        let (view, buffer) = self.workspace.focus_mut();
        view.selections = selections;
        view.scroll_to_cursor(buffer);
        self.close_panel();
        self.end_search();
    }

    /// Walks the hits the prompt has in hand.
    pub(super) fn go_to_match(&mut self, direction: Direction) {
        let from = self.from(direction);
        let hit = self
            .search
            .as_ref()
            .and_then(|search| search.next_hit(from, direction));
        match hit {
            Some(hit) => self.select_hit(hit),
            None => self.message = Some("Not found".to_owned()),
        }
    }

    /// `F3` with the prompt gone: walks the search in progress, bringing
    /// back the one `Esc` ended if there is none.
    pub(super) fn search_next(&mut self, direction: Direction) {
        if self.search.is_none()
            && let Some(query) = self.last.take()
        {
            let mut search = Session::default();
            search.retarget(query, self.buffer().text());
            self.search = Some(search);
        }
        if self.has_search() {
            self.go_to_match(direction);
        }
    }

    /// The query is kept for `F3` to bring back. The hits are not, so that
    /// a search nobody is looking at costs nothing to keep. A search for
    /// nothing has no query worth keeping over the one before it.
    pub(super) fn end_search(&mut self) {
        if let Some(search) = self.search.take()
            && !search.query().is_empty()
        {
            self.last = Some(search.query().clone());
        }
    }

    /// Where the next hit is looked for: past the primary selection going
    /// forward, before it going back — so the one selected is not found again.
    fn from(&self, direction: Direction) -> usize {
        let primary = self.view().selections.primary();
        match direction {
            Direction::Forward => primary.max(),
            Direction::Backward => primary.min(),
        }
    }

    fn select_hit(&mut self, hit: Range<usize>) {
        let (view, buffer) = self.workspace.focus_mut();
        view.selections = Query::selection(hit).into();
        view.scroll_to_cursor(buffer);
    }

    /// Every hit of the session becomes a cursor, and one insertion does the
    /// rest — so a replacement is one undo step. The search is over before
    /// the insertion, or it would be left looking for what is no longer there.
    pub(super) fn replace_all(&mut self, replacement: &str) {
        let Some(search) = &self.search else {
            return;
        };
        let hits = search.hits().len();
        if hits == 0 {
            self.message = Some("Not found".to_owned());
            return;
        }

        let selections = search.selections(0);
        self.view_mut().selections = selections;
        self.end_search();
        self.insert(replacement, EditKind::Other);
        self.message = Some(format!("Replaced {hits}"));
    }

    /// First press selects the word under each cursor; every press after that
    /// adds the next occurrence of it, wrapping around the document.
    pub(super) fn select_next_occurrence(&mut self) {
        let Some(query) = self.take_query() else {
            return;
        };

        let (view, buffer) = self.workspace.focus_mut();
        let text = buffer.text();
        let selected = |hit: &Range<usize>| view.selections.iter().any(|s| s.range() == *hit);

        // Hits already under a cursor are passed over, not stopped at: the
        // primary need not be the last one added. Coming back round to the
        // first hit found means there is nothing left to add.
        let from = view.selections.primary().max();
        let Some(first) = query.find(text, from, Direction::Forward) else {
            return;
        };
        let mut hit = first.clone();
        while selected(&hit) {
            let Some(next) = query.find(text, hit.end, Direction::Forward) else {
                return;
            };
            if next == first {
                return;
            }
            hit = next;
        }

        view.selections.push(Query::selection(hit));
        view.scroll_to_cursor(buffer);
        self.remember_occurrences(query.whole_word);
    }

    pub(super) fn select_all_occurrences(&mut self) {
        let Some(query) = self.take_query() else {
            return;
        };

        // One past the limit is all it takes to know there are too many.
        let selections: Vec<_> = query
            .all(self.buffer().text())
            .take(MAX_CURSORS + 1)
            .map(Query::selection)
            .collect();
        if selections.len() > MAX_CURSORS {
            self.message = Some(too_many_hits());
            return;
        }

        let (view, buffer) = self.workspace.focus_mut();
        view.selections = SelectionSet::from_selections(selections, 0);
        view.scroll_to_cursor(buffer);
        self.remember_occurrences(query.whole_word);
    }

    /// Bare cursors first expand to the word they touch; only once something
    /// is selected is there anything to search for.
    fn take_query(&mut self) -> Option<Query> {
        let (view, buffer) = self.workspace.focus_mut();
        let text = buffer.text();
        if view.selections.iter().any(Selection::is_empty) {
            view.selections = view.selections.mapped(|selection| {
                if !selection.is_empty() {
                    return selection;
                }
                let word = movement::word_around(text, selection.head);
                Selection {
                    anchor: word.start,
                    head: word.end,
                    goal_column: None,
                }
            });
            self.remember_occurrences(true);
            return None;
        }

        // Anything else moving the cursors means the user chose this text.
        let whole_word = self
            .occurrences
            .as_ref()
            .is_some_and(|hunt| hunt.whole_word && hunt.selections == self.view().selections);

        let range = self.view().selections.primary().range();
        Some(Query {
            needle: self.buffer().text().substring(range),
            whole_word,
        })
    }

    fn remember_occurrences(&mut self, whole_word: bool) {
        self.occurrences = Some(Occurrences {
            selections: self.view().selections.clone(),
            whole_word,
        });
    }
}
