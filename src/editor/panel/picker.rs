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
    /// Indices into `candidates`, best first.
    matches: Vec<usize>,
    scores: Vec<i32>,
    pub selected: usize,
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
            scores: Vec::new(),
            selected: 0,
        }
    }

    pub fn height(&self) -> usize {
        1 + self.matches.len().min(VISIBLE)
    }

    pub fn total(&self) -> usize {
        self.candidates.len()
    }

    pub fn len(&self) -> usize {
        self.matches.len()
    }

    pub fn selection(&self) -> Option<&Candidate> {
        let index = *self.matches.get(self.selected)?;
        Some(&self.candidates[index])
    }

    /// The window of matches on screen, kept around the selected one.
    pub fn visible(&self) -> impl Iterator<Item = (usize, &Candidate)> {
        let first = self.selected.saturating_sub(VISIBLE - 1);
        self.matches
            .iter()
            .enumerate()
            .skip(first)
            .take(VISIBLE)
            .map(|(row, &index)| (row, &self.candidates[index]))
    }

    /// Takes a batch the worker reported. New candidates are placed straight
    /// into the ranking, so the list stays sorted without re-scoring
    /// everything seen so far (P9).
    pub fn accept(&mut self, batch: Vec<Candidate>) {
        let query = self.query.contents();
        for candidate in batch {
            if let Some(score) = fuzzy::score(&candidate.label, &query) {
                let at = self.rank_of(score);
                self.matches.insert(at, self.candidates.len());
                self.scores.insert(at, score);
            }
            self.candidates.push(candidate);
        }
    }

    pub fn refilter(&mut self) {
        let query = self.query.contents();
        let mut ranked: Vec<_> = self
            .candidates
            .iter()
            .enumerate()
            .filter_map(|(index, candidate)| {
                fuzzy::score(&candidate.label, &query).map(|score| (score, index))
            })
            .collect();
        ranked.sort_by_key(|&(score, _)| std::cmp::Reverse(score));

        self.scores = ranked.iter().map(|&(score, _)| score).collect();
        self.matches = ranked.into_iter().map(|(_, index)| index).collect();
        self.selected = 0;
    }

    pub fn move_selection(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let last = self.matches.len() - 1;
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    fn rank_of(&self, score: i32) -> usize {
        self.scores.partition_point(|&other| other >= score)
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
        picker.visible().map(|(_, item)| item.label.as_str()).collect()
    }

    #[test]
    fn everything_shows_until_something_is_typed() {
        let picker = picker(&["rust", "python", "go"]);
        assert_eq!(picker.len(), 3);
    }

    #[test]
    fn a_source_is_streamed_in_batches_and_then_the_worker_ends() {
        let (mut jobs, rx) = Jobs::new();
        let items: Vec<Candidate> = (0..BATCH + 1).map(|n| Candidate::plain(n.to_string())).collect();
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

        assert_eq!(picker.len(), 0);
        assert_eq!(picker.selection(), None);
    }

    #[test]
    fn candidates_arriving_later_land_in_the_right_place() {
        let mut picker = Picker::new(Pick::Language, JobId::default());
        picker.query.insert("go");
        picker.accept(batch(&["zebra", "go", "gopher"]));

        // Sorted by score, not by arrival.
        assert_eq!(shown(&picker), ["go", "gopher"]);
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
