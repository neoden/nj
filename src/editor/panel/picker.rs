use super::prompt::Field;
use crate::editor::job::{JobId, Report, Worker};
use crate::text::fuzzy;

/// Rows of candidates shown at once. The query line sits above them.
pub const VISIBLE: usize = 10;

/// Candidates per report. Bounded so a directory with a hundred thousand
/// files still draws its first frame at once, and so that receiving one
/// batch stays within a frame (ADR 21).
const BATCH: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pick {
    Language,
    /// The command palette (F2): every command that can be asked for by name.
    Command,
}

/// One thing to choose from. The label is what is matched and what is
/// chosen; the detail is only shown beside it — the key a command is bound
/// to, say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub label: String,
    pub detail: String,
}

impl Candidate {
    pub fn plain(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: String::new(),
        }
    }
}

/// Reads a source on a worker and reports it in batches. What the picker
/// was built for: a source that takes a while does not hold up the frame.
pub fn stream(source: impl Iterator<Item = Candidate>, worker: &Worker) {
    let mut source = source.peekable();
    while source.peek().is_some() && !worker.cancelled() {
        let batch: Vec<Candidate> = source.by_ref().take(BATCH).collect();
        if !worker.report(Report::Candidates {
            job: worker.id(),
            batch,
        }) {
            return;
        }
    }
}

pub struct Picker {
    pub pick: Pick,
    pub query: Field,
    /// The worker reading the candidates; its reports are the only ones
    /// this picker takes.
    pub job: JobId,
    candidates: Vec<Candidate>,
    /// The candidates the query matches, best first.
    matches: Vec<Match>,
    pub selected: usize,
}

/// A candidate that matched, with how well it did — kept together, so that
/// the ranking is one list rather than two in step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Match {
    /// Index into `candidates`.
    candidate: usize,
    score: i32,
}

impl Picker {
    pub fn new(pick: Pick, job: JobId) -> Self {
        Self {
            pick,
            query: Field {
                label: ">",
                ..Field::default()
            },
            job,
            candidates: Vec::new(),
            matches: Vec::new(),
            selected: 0,
        }
    }

    pub fn height(&self) -> usize {
        1 + self.matches.len().min(VISIBLE)
    }

    pub fn total(&self) -> usize {
        self.candidates.len()
    }

    /// How many candidates the query matches, of `total`.
    pub fn matched(&self) -> usize {
        self.matches.len()
    }

    pub fn selection(&self) -> Option<&Candidate> {
        let found = self.matches.get(self.selected)?;
        Some(&self.candidates[found.candidate])
    }

    /// The `rows` matches to show, kept around the selected one. How many
    /// rows there are is the screen's decision, which may be fewer than
    /// `VISIBLE` asked for.
    pub fn visible(&self, rows: usize) -> impl Iterator<Item = (usize, &Candidate)> {
        let first = self.selected.saturating_sub(rows.saturating_sub(1));
        self.matches
            .iter()
            .enumerate()
            .skip(first)
            .take(rows)
            .map(|(row, found)| (row, &self.candidates[found.candidate]))
    }

    /// Takes a batch the worker reported. The batch is ranked on its own and
    /// merged into the ranking so far — one pass over each, with nothing
    /// seen before scored again (P9). Among equals the earlier arrival
    /// stays ahead, which is the order a stable sort in `refilter` gives.
    pub fn accept(&mut self, batch: Vec<Candidate>) {
        let query = self.query.contents();
        let first = self.candidates.len();
        let mut arrived: Vec<Match> = batch
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                fuzzy::score(&candidate.label, &query).map(|score| Match {
                    candidate: first + index,
                    score,
                })
            })
            .collect();
        arrived.sort_by_key(|found| std::cmp::Reverse(found.score));

        let mut merged = Vec::with_capacity(self.matches.len() + arrived.len());
        let mut old = std::mem::take(&mut self.matches).into_iter().peekable();
        let mut new = arrived.into_iter().peekable();
        while let (Some(before), Some(after)) = (old.peek(), new.peek()) {
            let next = match before.score >= after.score {
                true => old.next(),
                false => new.next(),
            };
            merged.extend(next);
        }
        merged.extend(old);
        merged.extend(new);

        self.matches = merged;
        self.candidates.extend(batch);
    }

    pub fn refilter(&mut self) {
        let query = self.query.contents();
        self.matches = self
            .candidates
            .iter()
            .enumerate()
            .filter_map(|(candidate, found)| {
                fuzzy::score(&found.label, &query).map(|score| Match { candidate, score })
            })
            .collect();
        self.matches
            .sort_by_key(|found| std::cmp::Reverse(found.score));
        self.selected = 0;
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let last = self.matches.len() - 1;
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }
}

impl std::fmt::Debug for Picker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Picker")
            .field("pick", &self.pick)
            .field("candidates", &self.candidates.len())
            .field("matches", &self.matches.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::editor::job::{Jobs, Message};

    fn batch(items: &[&str]) -> Vec<Candidate> {
        items.iter().map(|&item| Candidate::plain(item)).collect()
    }

    fn picker(items: &[&str]) -> Picker {
        let mut picker = Picker::new(Pick::Language, JobId::default());
        picker.accept(batch(items));
        picker
    }

    fn shown(picker: &Picker) -> Vec<&str> {
        picker
            .visible(VISIBLE)
            .map(|(_, item)| item.label.as_str())
            .collect()
    }

    #[test]
    fn everything_shows_until_something_is_typed() {
        let picker = picker(&["rust", "python", "go"]);
        assert_eq!(picker.matched(), 3);
    }

    #[test]
    fn a_source_is_streamed_in_batches_and_then_the_worker_ends() {
        let (mut jobs, rx) = Jobs::new();
        let items: Vec<Candidate> = (0..BATCH + 1)
            .map(|n| Candidate::plain(n.to_string()))
            .collect();
        let id = jobs.spawn(move |worker| stream(items.into_iter(), &worker));

        let mut reports = Vec::new();
        loop {
            match rx.recv().unwrap() {
                Message::Report(Report::Ended(job)) => {
                    assert_eq!(job, id);
                    break;
                }
                Message::Report(Report::Candidates { job, batch }) => {
                    assert_eq!(job, id);
                    reports.push(batch.len());
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        assert_eq!(reports, [BATCH, 1]);
    }

    #[test]
    fn typing_narrows_and_ranks() {
        let mut picker = picker(&["rust", "python", "typescript", "tidy"]);
        picker.query.insert("ty");
        picker.refilter();

        // `python` has no `y` after its `t`, so it is not a match at all.
        assert_eq!(shown(&picker), ["typescript", "tidy"]);
    }

    #[test]
    fn a_query_matching_nothing_leaves_an_empty_list() {
        let mut picker = picker(&["rust", "go"]);
        picker.query.insert("zzz");
        picker.refilter();

        assert_eq!(picker.matched(), 0);
        assert_eq!(picker.selection(), None);
    }

    #[test]
    fn candidates_arriving_later_land_in_the_right_place() {
        let mut picker = Picker::new(Pick::Language, JobId::default());
        picker.query.insert("go");
        picker.accept(batch(&["zebra", "go", "gopher"]));

        // Sorted by score, not by arrival.
        assert_eq!(shown(&picker), ["go", "gopher"]);

        // A later batch merges in: better ahead, equals behind what came
        // first, exactly as scoring everything at once would order it.
        picker.accept(batch(&["ago", "go", "gopher"]));
        let mut at_once = Picker::new(Pick::Language, JobId::default());
        at_once.query.insert("go");
        at_once.accept(batch(&["zebra", "go", "gopher", "ago", "go", "gopher"]));
        assert_eq!(shown(&picker), shown(&at_once));
        assert_eq!(shown(&picker), ["go", "go", "gopher", "gopher", "ago"]);
    }

    #[test]
    fn a_short_strip_still_shows_the_selection() {
        let mut picker = picker(&["one", "two", "three", "four"]);
        picker.move_selection(3);
        let rows: Vec<_> = picker.visible(2).map(|(row, _)| row).collect();
        assert_eq!(rows, [2, 3]);
    }

    #[test]
    fn the_selection_walks_the_list_and_stops_at_its_ends() {
        let mut picker = picker(&["one", "two", "three"]);
        let chosen = |picker: &Picker| picker.selection().map(|c| c.label.clone());
        assert_eq!(chosen(&picker).as_deref(), Some("one"));

        picker.move_selection(1);
        assert_eq!(chosen(&picker).as_deref(), Some("two"));

        picker.move_selection(99);
        assert_eq!(chosen(&picker).as_deref(), Some("three"));

        picker.move_selection(-99);
        assert_eq!(chosen(&picker).as_deref(), Some("one"));
    }

    #[test]
    fn typing_starts_the_selection_over() {
        let mut picker = picker(&["rust", "python", "go"]);
        picker.move_selection(2);
        picker.query.insert("o");
        picker.refilter();

        assert_eq!(picker.selected, 0);
    }
}
