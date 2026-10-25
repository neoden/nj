//! Finding things in the text: the search the search field starts, walking
//! its hits, replacing them one at a time, and the selection-driven kind —
//! `Ctrl+D` and `Ctrl+Shift+L` — that starts from the word under the cursor.
//! One engine (`text::search`) serves them all; what differs is the query
//! and where the hits go.

use std::ops::Range;

use super::Editor;
use super::panel::Panel;
use super::panel::replacing::Replacing;
use crate::buffer::EditKind;
use crate::text::Text;
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

/// A run of `Ctrl+D`: the cursors it left behind, so the next press can
/// tell whether it is the same run, and what it is hunting — settled when
/// the run began, so no press after that has to look at the text again.
/// `None` when the cursors took different words, which leaves nothing to
/// hunt.
pub(super) struct Occurrences {
    selections: SelectionSet,
    query: Option<Query>,
}

impl Editor {
    /// Turns the hits into cursors. The search is over with that: its hits
    /// are what is selected now.
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
        self.end_search();
    }

    /// `Enter` in a search field: what is in it becomes the search, and the
    /// selection goes to its first hit from the cursor. A field with nothing
    /// in it starts no search.
    pub(super) fn start_search(&mut self, needle: String, direction: Direction) {
        let Some(search) = Session::new(Query::literal(needle), self.buffer().text()) else {
            return;
        };
        self.search = Some(search);
        self.go_to_match(direction);
    }

    /// `Enter` in the replace fields: the search starts, and with a hit to
    /// stand on the question is asked. The first hit is looked for from
    /// where the selection begins, not past its end as `F3` looks: a hit
    /// that is selected is the one to ask about first, not last.
    pub(super) fn start_replacing(&mut self, needle: String, replacement: String) {
        let Some(search) = Session::new(Query::literal(needle), self.buffer().text()) else {
            return;
        };
        let left = search.hits().len();
        let from = self.view().selections.primary().min();
        let first = search.next_hit(from, Direction::Forward);
        self.search = Some(search);
        self.last_replacement = Some(replacement.clone());
        match first {
            Some(hit) => {
                self.select_hit(hit);
                self.panel = Some(Panel::Replacing(Replacing {
                    replacement,
                    left,
                    replaced: 0,
                }));
            }
            None => self.message = Some("Not found".to_owned()),
        }
    }

    /// `y` while replacing: the selected hit becomes the replacement, as an
    /// undo step of its own. The next hit is looked for after what was put
    /// in, so a replacement that contains the query is not offered to itself.
    pub(super) fn replace_hit(&mut self) {
        let Some(Panel::Replacing(replacing)) = &mut self.panel else {
            return;
        };
        replacing.replaced += 1;
        let replacement = replacing.replacement.clone();
        self.insert(&replacement, EditKind::Other);
        // From here it is the same as leaving it: one fewer to ask about.
        self.skip_hit();
    }

    /// `n` while replacing: the selected hit stays as it is. With none left
    /// to ask about, the walk is over.
    pub(super) fn skip_hit(&mut self) {
        let Some(Panel::Replacing(replacing)) = &mut self.panel else {
            return;
        };
        replacing.left -= 1;
        match replacing.left {
            0 => self.close_panel(),
            _ => self.go_to_match(Direction::Forward),
        }
    }

    /// Walks the hits of the search going on.
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

    /// `F3`: walks the search in progress, bringing back the one `Esc` ended
    /// if there is none.
    pub(super) fn search_next(&mut self, direction: Direction) {
        if self.search.is_none()
            && let Some(query) = self.last.take()
        {
            self.search = Session::new(query, self.buffer().text());
        }
        if self.has_search() {
            self.go_to_match(direction);
        }
    }

    /// The query is kept for `F3` to bring back. The hits are not, so that
    /// a search nobody is looking at costs nothing to keep.
    pub(super) fn end_search(&mut self) {
        if let Some(search) = self.search.take() {
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

    /// `a` while replacing: every hit not answered for yet becomes the
    /// replacement — the selected one and those the walk had still to come
    /// to. They become cursors and one insertion does the rest, so it is one
    /// undo step. Closing the question ends the search, and that comes
    /// before the insertion, or the search would be left looking for what is
    /// no longer there.
    pub(super) fn replace_rest(&mut self) {
        let Some(Panel::Replacing(replacing)) = &mut self.panel else {
            return;
        };
        replacing.replaced += replacing.left;
        let (replacement, left) = (replacing.replacement.clone(), replacing.left);
        let Some(search) = &self.search else {
            return;
        };

        let selections = search.rest(self.view().selections.primary().min(), left);
        self.view_mut().selections = selections;
        self.close_panel();
        self.insert(&replacement, EditKind::Other);
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

        view.selections.push(Query::selection(hit.clone()));
        view.reveal(buffer, hit.end);
        self.remember_occurrences(query);
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

        // The primary stays the occurrence the person is on; the others are
        // added around it, as `Ctrl+D` would have added them one by one.
        let (view, buffer) = self.workspace.focus_mut();
        let at = view.selections.primary().range();
        let primary = selections
            .iter()
            .position(|selection| selection.range() == at)
            .unwrap_or(0);
        view.selections = SelectionSet::from_selections(selections, primary);
        view.scroll_to_cursor(buffer);
        self.remember_occurrences(query);
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
            // The run begins: it hunts the word, if the cursors took one.
            let query = agreed_text(text, &view.selections).map(Query::word);
            self.occurrences = Some(Occurrences {
                selections: view.selections.clone(),
                query,
            });
            return None;
        }

        // The same run going on: what it hunts was settled when it began.
        if let Some(hunt) = &self.occurrences
            && hunt.selections == view.selections
        {
            return hunt.query.clone();
        }

        // Anything else moving the cursors means the user chose this text,
        // exactly as it stands — if the selections agree on one.
        agreed_text(text, &view.selections).map(Query::literal)
    }

    fn remember_occurrences(&mut self, query: Query) {
        self.occurrences = Some(Occurrences {
            selections: self.view().selections.clone(),
            query: Some(query),
        });
    }
}

/// The text every selection holds, when they all hold the same; with
/// different texts there is no one thing to look for, and guessing — the
/// primary's, say — would be harder to predict than doing nothing.
fn agreed_text(text: &Text, selections: &SelectionSet) -> Option<String> {
    let needle = text.substring(selections.primary().range());
    selections
        .iter()
        .all(|selection| text.substring(selection.range()) == needle)
        .then_some(needle)
}
