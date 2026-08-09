use std::ops::Range;

use crate::clipboard::{Clipboard, SystemClipboard};
use crate::document::{Document, EditKind};
use crate::key;
use crate::indent;
use crate::language::{self, Indent, Language};
use crate::layout;
use crate::picker::{Pick, Picker};
use crate::movement::{self, Class, Direction, Motion};
use crate::panel::{Confirm, Panel, Question};
use crate::prompt::{Prompt, Purpose};
use crate::selection::{Selection, SelectionSet};
use crate::text::Text;
use crate::transaction::{self, Change, CursorAfter, Transaction};
use crate::view::View;

pub struct Editor {
    pub document: Document,
    pub view: View,
    pub clipboard: Clipboard,
    pub panel: Option<Panel>,
    pub message: Option<String>,
    /// Every hit of the open query. Lives only while the prompt does, so
    /// nothing has to remember to invalidate it.
    pub matches: Vec<Range<usize>>,
    /// The query `matches` holds the hits of, so that extending it can narrow
    /// them rather than start over.
    collected: String,
    pub quit: bool,
    occurrences: Option<Occurrences>,
    last_search: Option<String>,
    system: Option<SystemClipboard>,
}

impl Editor {
    pub fn new(document: Document) -> Self {
        let view = View {
            tab_width: document.language.tab_width,
            ..View::default()
        };
        Self {
            document,
            view,
            clipboard: Clipboard::default(),
            panel: None,
            message: None,
            matches: Vec::new(),
            collected: String::new(),
            quit: false,
            occurrences: None,
            last_search: None,
            system: None,
        }
    }

    pub fn open_prompt(&mut self, purpose: Purpose) {
        let mut prompt = Prompt::new(purpose, self.view.selections.clone());

        // A selection spanning lines means something else entirely, so only a
        // fragment seeds the query. What is seeded stays selected, so typing
        // replaces it instead of appending to it.
        if matches!(purpose, Purpose::Search | Purpose::Replace)
            && let Some(selected) = self.selected_text().filter(|text| !text.contains('\n'))
        {
            prompt.insert(&selected);
            prompt.select_all();
        }

        self.panel = Some(Panel::Prompt(prompt));
        self.prompt_changed();
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.panel.as_ref()?.as_prompt()
    }

    pub fn open_picker(&mut self, pick: Pick) {
        let mut picker = match pick {
            Pick::Language => Picker::new(
                pick,
                language::LANGUAGES
                    .iter()
                    .chain([&language::PLAIN])
                    .map(|language| language.name.to_owned()),
            ),
        };
        picker.pull();
        self.panel = Some(Panel::Picker(picker));
    }

    /// True while a picker still has candidates to read, which is what keeps
    /// the main loop from going back to sleep.
    pub fn is_loading(&self) -> bool {
        match &self.panel {
            Some(Panel::Picker(picker)) => picker.is_loading(),
            _ => false,
        }
    }

    pub fn load_more(&mut self) {
        if let Some(Panel::Picker(picker)) = &mut self.panel {
            picker.pull();
        }
    }

    pub fn accept_pick(&mut self) {
        let Some(Panel::Picker(picker)) = &self.panel else {
            return;
        };
        let Some(chosen) = picker.selection() else {
            return;
        };

        match picker.pick {
            Pick::Language => match language::by_name(chosen) {
                Some(language) => self.set_language(language),
                None => return,
            },
        }
        self.close_panel();
    }

    pub fn panel_height(&self) -> usize {
        self.panel.as_ref().map_or(0, Panel::height)
    }

    /// The situational half of the status bar belongs to whatever is open.
    pub fn panel_status(&self) -> String {
        match &self.panel {
            Some(Panel::Prompt(prompt))
                if matches!(prompt.purpose, Purpose::Search | Purpose::Replace) =>
            {
                self.search_status()
            }
            Some(Panel::Picker(picker)) => format!("{}/{}", picker.len(), picker.total()),
            _ => String::new(),
        }
    }

    fn selected_text(&self) -> Option<String> {
        let primary = self.view.selections.primary();
        match primary.is_empty() {
            true => None,
            false => Some(self.document.text.slice(primary.range()).to_string()),
        }
    }

    /// Closes the panel where the search left off. There is no separate
    /// "accept", so leaving is how you keep what you found.
    pub fn close_panel(&mut self) {
        self.panel = None;
        self.matches.clear();
        self.collected.clear();
    }

    /// `Enter` walks the hits while searching; for anything else it answers
    /// the question and closes the strip.
    pub fn submit_prompt(&mut self, backwards: bool) {
        let Some(prompt) = self.prompt() else {
            return;
        };
        let purpose = prompt.purpose;
        let first = prompt.contents(0);

        match purpose {
            Purpose::Search => {
                return self.go_to_match(match backwards {
                    true => Direction::Backward,
                    false => Direction::Forward,
                })
            }
            Purpose::GotoLine => match parse_line_number(&first) {
                Some(line) => self.goto_line(line),
                None => self.message = Some("Not a line number".to_owned()),
            },
            Purpose::Replace => {
                let replacement = prompt.contents(1);
                self.replace_all(&first, &replacement);
            }
        }

        self.close_panel();
    }

    /// Incremental search: every keystroke collects the hits afresh and lands
    /// on the first one at or after where the cursor stood when the prompt
    /// opened, so the match never drifts forward as the query grows.
    pub fn prompt_changed(&mut self) {
        let Some(prompt) = self.prompt() else {
            return;
        };
        if !matches!(prompt.purpose, Purpose::Search | Purpose::Replace) {
            return;
        }

        let needle = prompt.contents(0);
        let origin = prompt.origin.clone();
        self.collect_matches(&needle);
        self.last_search = (!needle.is_empty()).then_some(needle);

        if self.matches.is_empty() {
            self.view.selections = origin;
            return;
        }

        let from = origin.primary().min();
        let found = self
            .matches
            .iter()
            .find(|found| found.start >= from)
            .or_else(|| self.matches.first())
            .cloned();
        if let Some(found) = found {
            self.select_match(found.start, found.len());
        }
    }

    fn collect_matches(&mut self, needle: &str) {
        if needle.is_empty() {
            self.matches.clear();
            self.collected.clear();
            return;
        }

        let text = &self.document.text;
        // Hits of a longer query are a subset of the shorter one's, so typing
        // sifts what is already there instead of reading the document again.
        match !self.collected.is_empty() && needle.starts_with(&self.collected) {
            true => self.matches.retain_mut(|found| {
                let matched = text.matches_at(found.start, needle);
                found.end = found.start + needle.len();
                matched
            }),
            false => self.matches = text.matches(needle).collect(),
        }
        self.collected = needle.to_owned();
    }

    /// Turns the hits into cursors and hands the keyboard back to the text,
    /// which is the only reason anyone wants a cursor on every hit.
    pub fn select_all_matches(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        let at = self.view.selections.primary().min();
        let primary = self
            .matches
            .iter()
            .position(|found| found.start == at)
            .unwrap_or(0);
        let selections = self
            .matches
            .iter()
            .map(|found| Selection {
                anchor: found.start,
                head: found.end,
                goal_column: None,
            })
            .collect();

        self.view.selections = SelectionSet::from_selections(selections, primary);
        self.close_panel();
        self.view.scroll_to_cursor(&self.document.text);
    }

    pub fn go_to_match(&mut self, direction: Direction) {
        if self.matches.is_empty() {
            self.message = Some("Not found".to_owned());
            return;
        }

        let from = self.view.selections.primary().min();
        let found = match direction {
            Direction::Forward => self
                .matches
                .iter()
                .find(|found| found.start > from)
                .or_else(|| self.matches.first()),
            Direction::Backward => self
                .matches
                .iter()
                .rev()
                .find(|found| found.start < from)
                .or_else(|| self.matches.last()),
        }
        .cloned();

        if let Some(found) = found {
            self.select_match(found.start, found.len());
        }
    }

    /// `found: 3/17`.
    fn search_status(&self) -> String {
        if self.matches.is_empty() {
            return "Found: none".to_owned();
        }
        let at = self.view.selections.primary().min();
        let current = self
            .matches
            .iter()
            .position(|found| found.start == at)
            .map_or(0, |index| index + 1);
        format!("Found: {current}/{}", self.matches.len())
    }

    pub fn search_next(&mut self, direction: Direction) {
        let Some(needle) = self.last_search.clone() else {
            return;
        };
        let from = match direction {
            Direction::Forward => self.view.selections.primary().max(),
            Direction::Backward => self.view.selections.primary().min(),
        };

        match self.find(&needle, from, direction) {
            Some(at) => self.select_match(at, needle.len()),
            None => self.message = Some("Not found".to_owned()),
        }
    }

    /// Both directions wrap, which is what every editor's find does.
    fn find(&self, needle: &str, from: usize, direction: Direction) -> Option<usize> {
        let text = &self.document.text;
        match direction {
            Direction::Forward => text.find(needle, from).or_else(|| text.find(needle, 0)),
            Direction::Backward => {
                last_match_before(text, needle, from).or_else(|| last_match_before(text, needle, text.len()))
            }
        }
    }

    fn select_match(&mut self, at: usize, length: usize) {
        self.view.selections = Selection {
            anchor: at,
            head: at + length,
            goal_column: None,
        }
        .into();
        self.view.scroll_to_cursor(&self.document.text);
    }

    fn replace_all(&mut self, needle: &str, replacement: &str) {
        if needle.is_empty() {
            return;
        }

        let matches: Vec<Selection> = self
            .document
            .text
            .matches(needle)
            .map(|found| Selection {
                anchor: found.start,
                head: found.end,
                goal_column: None,
            })
            .collect();

        if matches.is_empty() {
            self.message = Some("Not found".to_owned());
            return;
        }

        self.message = Some(format!("Replaced {}", matches.len()));
        self.view.selections = SelectionSet::from_selections(matches, 0);
        self.insert(replacement);
    }

    fn goto_line(&mut self, line: usize) {
        let text = &self.document.text;
        let line = line.min(text.line_count().saturating_sub(1));
        self.view.selections = Selection::cursor(text.line_start(line)).into();
        self.view.scroll_to_cursor(text);
    }

    pub fn save(&mut self) -> bool {
        match self.document.save() {
            Ok(()) => true,
            Err(error) => {
                self.message = Some(format!("{error:#}"));
                false
            }
        }
    }

    /// Nothing is ever discarded without being asked about first.
    pub fn request_quit(&mut self) {
        match self.document.is_modified() {
            false => self.quit = true,
            true => self.panel = Some(Panel::Confirm(Confirm::quit())),
        }
    }

    /// A failed save cancels the quit and says why, rather than losing the
    /// file that could not be written.
    pub fn answer(&mut self, answer: char) {
        let Some(Panel::Confirm(confirm)) = &self.panel else {
            return;
        };

        // The letters are commands, not text, so they are read off the
        // physical key rather than the current keyboard layout.
        match (confirm.about, key::physical_key(answer)) {
            (Question::Quit, 's') => self.quit = self.save(),
            (Question::Quit, 'q') => self.quit = true,
            _ => return,
        }
        self.close_panel();
    }

    pub fn set_language(&mut self, language: &'static Language) {
        self.document.language = language;
        self.document.indent = indent::detect(&self.document.text, language.indent);
        self.view.tab_width = language.tab_width;
    }

    /// Left out by default so that tests never touch the real clipboard.
    pub fn with_system_clipboard(mut self) -> Self {
        self.system = Some(SystemClipboard::new());
        self
    }

    pub fn move_selections(&mut self, motion: Motion, extend: bool) {
        self.view
            .move_selections(&self.document.text, motion, extend);
    }

    pub fn insert(&mut self, inserted: &str) {
        let transaction = Transaction::edit(
            &self.view.selections,
            CursorAfter::EndOfChange,
            |selection| (selection.range(), inserted.to_owned()),
        );
        self.apply(transaction, insert_kind(inserted));
    }

    /// With nothing selected, whole lines are taken — and put back as whole
    /// lines, which is what every editor with this binding does. A line under
    /// several cursors is taken once, however many of them sit on it.
    pub fn copy(&mut self) {
        let lines = self.view.selections.iter().all(Selection::is_empty);
        let text = &self.document.text;
        let pieces = match lines {
            true => self
                .touched_lines()
                .into_iter()
                .map(|line| {
                    let mut piece = text.slice(line_range(text, line)).to_string();
                    if !piece.ends_with('\n') {
                        piece.push('\n');
                    }
                    piece
                })
                .collect(),
            false => self
                .view
                .selections
                .iter()
                .map(|selection| text.slice(selection.range()).to_string())
                .collect(),
        };

        self.clipboard.set(pieces, lines);

        let text = self.clipboard.text();
        if let Some(system) = &mut self.system {
            system.write(&text);
        }
    }

    pub fn cut(&mut self) {
        self.copy();

        if self.clipboard.holds_lines() {
            return self.cut_lines();
        }

        let transaction = Transaction::edit(
            &self.view.selections,
            CursorAfter::EndOfChange,
            |selection| (selection.range(), String::new()),
        );

        if !transaction.is_empty() {
            self.apply(transaction, EditKind::Other);
        }
    }

    /// One change per line rather than per cursor: a line removed twice would
    /// take the line after it as well.
    fn cut_lines(&mut self) {
        let text = &self.document.text;
        let changes: Vec<Change> = self
            .touched_lines()
            .into_iter()
            .map(|line| Change {
                range: line_range(text, line),
                text: String::new(),
            })
            .collect();

        let after = self.view.selections.mapped(|selection| {
            Selection::cursor(transaction::displace(selection.head, &changes))
        });

        let transaction = Transaction::over_lines(&self.view.selections, changes, after);
        self.apply(transaction, EditKind::Other);
    }

    pub fn paste(&mut self) {
        self.adopt_foreign_clipboard();
        if self.clipboard.is_empty() {
            return;
        }

        // Whole lines land above the line a cursor is on, so cursors sharing a
        // line share the one insertion, and only that one takes a piece.
        let lines = self.clipboard.holds_lines();
        let leading = match lines {
            true => self.first_on_each_line(|selection| selection.head),
            false => vec![true; self.view.selections.len()],
        };
        let mut pieces = self
            .clipboard
            .pieces_for(leading.iter().filter(|&&first| first).count())
            .into_iter();
        let mut leading = leading.into_iter();

        let text = &self.document.text;
        let cursor = match lines {
            true => CursorAfter::Displaced,
            false => CursorAfter::EndOfChange,
        };
        let transaction = Transaction::edit(&self.view.selections, cursor, |selection| {
            if !leading.next().unwrap_or(true) {
                return (selection.head..selection.head, String::new());
            }
            let range = match lines {
                true => {
                    let start = text.line_start(text.line_of(selection.head));
                    start..start
                }
                false => selection.range(),
            };
            (range, pieces.next().unwrap_or_default())
        });

        self.apply(transaction, EditKind::Other);
    }

    /// Someone else copied since we did, so our pieces are stale: what the
    /// desktop holds is one piece of text, and it is not made of lines.
    fn adopt_foreign_clipboard(&mut self) {
        let Some(text) = self.system.as_mut().and_then(SystemClipboard::read) else {
            return;
        };
        if text != self.clipboard.text() {
            self.clipboard.set(vec![text], false);
        }
    }

    /// A new line starts where the current one starts. Nothing language-aware
    /// happens here: opening a block deepens the indent only once there is a
    /// parser to say so.
    pub fn insert_newline(&mut self) {
        let text = &self.document.text;
        let mut leading = self.first_on_each_line(Selection::min).into_iter();

        let transaction = Transaction::edit(
            &self.view.selections,
            CursorAfter::EndOfChange,
            |selection| {
                let line = text.line_of(selection.min());
                let start = text.line_start(line);
                let content = text.line_str(line);
                let local = selection.min() - start;

                let indent: String = content[..local]
                    .chars()
                    .take_while(|character| character.is_whitespace())
                    .collect();

                // A line holding nothing but this indent is left behind empty
                // rather than as a row of spaces.
                let abandoned = indent.len() == local
                    && content[local..].is_empty()
                    && selection.is_empty()
                    && leading.next().unwrap_or(false);

                let range = match abandoned {
                    true => start..selection.max(),
                    false => selection.range(),
                };
                (range, format!("\n{indent}"))
            },
        );

        self.apply(transaction, EditKind::Other);
    }

    /// Whether each selection is the first one on its line, so that a line
    /// edit happens once however many cursors sit on it.
    fn first_on_each_line(&self, offset: impl Fn(&Selection) -> usize) -> Vec<bool> {
        let text = &self.document.text;
        let mut previous = None;
        self.view
            .selections
            .iter()
            .map(|selection| {
                let line = text.line_of(offset(selection));
                let first = previous != Some(line);
                previous = Some(line);
                first
            })
            .collect()
    }

    /// `Tab` indents whole lines when a selection crosses one, and otherwise
    /// moves to the next tab stop, which is not the same as inserting a whole
    /// level when the cursor sits mid-line.
    pub fn indent(&mut self) {
        let selecting = self
            .view
            .selections
            .iter()
            .any(|selection| !selection.is_empty());

        match selecting {
            true => self.indent_lines(),
            false => {
                let inserted = match self.document.indent {
                    Indent::Tabs => "\t".to_owned(),
                    Indent::Spaces(width) => " ".repeat(width - self.cursor_column() % width),
                };
                self.insert(&inserted);
            }
        }
    }

    pub fn outdent(&mut self) {
        let text = &self.document.text;
        let unit = self.document.indent;
        let changes = self
            .touched_lines()
            .into_iter()
            .filter_map(|line| {
                let start = text.line_start(line);
                let content = text.line_str(line);
                let removed = match unit {
                    Indent::Tabs => content.starts_with('\t').then_some(1),
                    Indent::Spaces(width) => match content.starts_with('\t') {
                        true => Some(1),
                        false => match content.chars().take_while(|&c| c == ' ').count() {
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
            .collect::<Vec<_>>();

        if !changes.is_empty() {
            self.apply_to_lines(changes);
        }
    }

    fn indent_lines(&mut self) {
        let text = &self.document.text;
        let unit = self.document.indent.unit();
        let changes = self
            .touched_lines()
            .into_iter()
            .map(|line| {
                let start = text.line_start(line);
                Change {
                    range: start..start,
                    text: unit.clone(),
                }
            })
            .collect();

        self.apply_to_lines(changes);
    }

    /// A selection that took part in a line edit ends up covering those lines
    /// whole, so the indentation it just gained is inside it rather than
    /// hanging off its left edge.
    fn apply_to_lines(&mut self, changes: Vec<Change>) {
        let text = &self.document.text;
        let after = self.view.selections.mapped(|selection| {
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

        let transaction = Transaction::over_lines(&self.view.selections, changes, after);
        self.apply(transaction, EditKind::Other);
    }

    /// Comments the touched lines, or uncomments them when every one of them
    /// is already commented. Blank lines are left out either way.
    pub fn toggle_comment(&mut self) {
        let Some(token) = self.document.language.line_comment else {
            self.message = Some("No line comment for this language".to_owned());
            return;
        };

        let text = &self.document.text;
        let lines: Vec<usize> = self
            .touched_lines()
            .into_iter()
            .filter(|&line| !text.line_str(line).trim().is_empty())
            .collect();
        if lines.is_empty() {
            return;
        }

        let commented = lines
            .iter()
            .all(|&line| text.line_str(line).trim_start().starts_with(token));

        let changes = match commented {
            true => lines
                .iter()
                .map(|&line| {
                    let start = text.line_start(line);
                    let content = text.line_str(line);
                    let at = content.len() - content.trim_start().len();
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
                // what keeps a commented-out block readable.
                let column = lines
                    .iter()
                    .map(|&line| {
                        let content = text.line_str(line);
                        content.len() - content.trim_start().len()
                    })
                    .min()
                    .unwrap_or(0);

                lines
                    .iter()
                    .map(|&line| {
                        let at = text.line_start(line) + column;
                        Change {
                            range: at..at,
                            text: format!("{token} "),
                        }
                    })
                    .collect()
            }
        };

        self.apply_to_lines(changes);
    }

    /// Lines under any cursor, each named once however many cursors sit on it.
    /// A selection ending exactly on a line start does not reach that line —
    /// it only swallowed the break before it.
    fn touched_lines(&self) -> Vec<usize> {
        let text = &self.document.text;
        let mut lines: Vec<usize> = self
            .view
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

    fn cursor_column(&self) -> usize {
        let text = &self.document.text;
        let head = self.view.selections.primary().head;
        let line = text.line_of(head);
        layout::column_of_byte(
            &text.line_str(line),
            head - text.line_start(line),
            self.view.tab_width,
        )
    }

    /// A cursor with nothing selected eats the neighbouring text instead.
    pub fn delete(&mut self, motion: Motion) {
        let text = &self.document.text;
        let indent = self.document.indent;
        let transaction = Transaction::edit(
            &self.view.selections,
            CursorAfter::EndOfChange,
            |selection| {
                let head = selection.head;
                let range = if !selection.is_empty() {
                    selection.range()
                } else if let Some(range) = indent_deletion(text, indent, motion, head) {
                    range
                } else {
                    let target = match motion {
                        Motion::Word(direction) => movement::word(text, head, direction),
                        Motion::LineEdge(direction) => movement::line_edge(text, head, direction),
                        Motion::Grapheme(direction) => movement::grapheme(text, head, direction),
                        _ => head,
                    };
                    head.min(target)..head.max(target)
                };
                (range, String::new())
            },
        );

        let kind = match motion {
            Motion::Grapheme(direction) => EditKind::Delete(direction),
            _ => EditKind::Other,
        };
        if !transaction.is_empty() {
            self.apply(transaction, kind);
        }
    }

    pub fn remove_secondary_cursors(&mut self) {
        self.view.selections.collapse_to_primary();
        self.view.scroll_to_cursor(&self.document.text);
    }

    pub fn cancel_selection(&mut self) {
        self.view.selections.collapse_to_cursors();
    }

    /// Takes the lines a selection touches, break included. Pressing again
    /// swallows the next line, because the selection now ends on its start.
    pub fn select_line(&mut self) {
        let text = &self.document.text;
        self.view.selections = self.view.selections.mapped(|selection| {
            let last = text.line_of(selection.max());
            Selection {
                anchor: text.line_start(text.line_of(selection.min())),
                head: match last + 1 < text.line_count() {
                    true => text.line_start(last + 1),
                    false => text.len(),
                },
                goal_column: None,
            }
        });
        self.view.scroll_to_cursor(&self.document.text);
    }

    /// The view stays where it is: jumping to the end of the file is not what
    /// anyone wants from selecting everything.
    pub fn select_all(&mut self) {
        self.view.selections = Selection {
            anchor: 0,
            head: self.document.text.len(),
            goal_column: None,
        }
        .into();
    }

    /// Moves the page without moving the cursors, which may leave them off
    /// screen until something moves them again.
    pub fn scroll(&mut self, delta: isize) {
        self.view.scroll_lines(delta, &self.document.text);
    }

    pub fn add_cursor(&mut self, direction: Direction) {
        self.view.add_cursor(&self.document.text, direction);
    }

    /// First press selects the word under each cursor; every press after that
    /// adds the next occurrence of it, wrapping around the document.
    pub fn select_next_occurrence(&mut self) {
        let Some(search) = self.take_search() else {
            return;
        };

        let text = &self.document.text;
        let from = self.view.selections.primary().max();
        let Some(at) = search
            .find(text, from)
            .or_else(|| search.find(text, 0))
        else {
            return;
        };

        let found = search.selection_at(at);
        if self
            .view
            .selections
            .iter()
            .any(|selection| selection.range() == found.range())
        {
            return;
        }

        self.view.selections.push(found);
        self.remember_occurrences(search.whole_word);
        self.view.scroll_to_cursor(&self.document.text);
    }

    pub fn select_all_occurrences(&mut self) {
        let Some(search) = self.take_search() else {
            return;
        };

        let text = &self.document.text;
        let mut selections = Vec::new();
        let mut at = 0;
        while let Some(found) = search.find(text, at) {
            at = found + search.needle.len();
            selections.push(search.selection_at(found));
        }

        self.view.selections = SelectionSet::from_selections(selections, 0);
        self.remember_occurrences(search.whole_word);
        self.view.scroll_to_cursor(&self.document.text);
    }

    /// Bare cursors first expand to the word they touch; only once something
    /// is selected is there anything to search for.
    fn take_search(&mut self) -> Option<Search> {
        let text = &self.document.text;
        if self.view.selections.iter().any(Selection::is_empty) {
            self.view.selections = self.view.selections.mapped(|selection| {
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
            .is_some_and(|hunt| hunt.whole_word && hunt.selections == self.view.selections);

        let range = self.view.selections.primary().range();
        Some(Search {
            whole_word,
            needle: self.document.text.slice(range).to_string(),
        })
    }

    fn remember_occurrences(&mut self, whole_word: bool) {
        self.occurrences = Some(Occurrences {
            selections: self.view.selections.clone(),
            whole_word,
        });
    }

    pub fn undo(&mut self) {
        if let Some(selections) = self.document.undo() {
            self.view.selections = selections;
            self.view.scroll_to_cursor(&self.document.text);
        }
    }

    pub fn redo(&mut self) {
        if let Some(selections) = self.document.redo() {
            self.view.selections = selections;
            self.view.scroll_to_cursor(&self.document.text);
        }
    }

    fn apply(&mut self, transaction: Transaction, kind: EditKind) {
        self.view.selections = self.document.apply(transaction, kind);
        self.view.scroll_to_cursor(&self.document.text);
    }
}

fn last_match_before(text: &Text, needle: &str, limit: usize) -> Option<usize> {
    text.matches(needle)
        .take_while(|found| found.start < limit)
        .last()
        .map(|found| found.start)
}

/// Backspace inside a run of leading spaces removes back to the previous tab
/// stop, mirroring what Tab does forwards. Anywhere else it is an ordinary
/// deletion, or alignment inside a line would be eaten a level at a time.
fn indent_deletion(
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

/// Line numbers are what people count from one.
fn parse_line_number(input: &str) -> Option<usize> {
    let line = input.trim().parse::<usize>().ok()?;
    Some(line.saturating_sub(1))
}

/// A whole line with the break after it, which is what a copy takes when
/// nothing is selected.
fn line_range(text: &Text, line: usize) -> Range<usize> {
    let start = text.line_start(line);
    start..(start + text.line(line).len() + 1).min(text.len())
}

/// The cursors a run of `Ctrl+D` left behind, so the next press can tell
/// whether it is the same run.
struct Occurrences {
    selections: SelectionSet,
    whole_word: bool,
}

struct Search {
    needle: String,
    /// Set when the run uncovered the word itself: renaming `i` leaves `if`
    /// alone.
    whole_word: bool,
}

impl Search {
    fn find(&self, text: &Text, from: usize) -> Option<usize> {
        let mut at = from;
        loop {
            let found = text.find(&self.needle, at)?;
            if !self.whole_word || self.is_whole_word(text, found) {
                return Some(found);
            }
            at = movement::grapheme(text, found, Direction::Forward);
        }
    }

    fn is_whole_word(&self, text: &Text, at: usize) -> bool {
        let before = movement::grapheme(text, at, Direction::Backward);
        let word_before = before != at && text.char_at(before).map(Class::of) == Some(Class::Word);
        let word_after = text.char_at(at + self.needle.len()).map(Class::of) == Some(Class::Word);
        !word_before && !word_after
    }

    fn selection_at(&self, at: usize) -> Selection {
        Selection {
            anchor: at,
            head: at + self.needle.len(),
            goal_column: None,
        }
    }
}

/// Only single characters join a group; a paste or a line break stands alone.
fn insert_kind(inserted: &str) -> EditKind {
    let mut characters = inserted.chars();
    match (characters.next(), characters.next()) {
        (Some(character), None) if character != '\n' => EditKind::Insert(Class::of(character)),
        _ => EditKind::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::movement::Direction;
    use crate::selection::{Selection, SelectionSet};
    use crate::text::Text;

    fn editor(content: &str, offset: usize) -> Editor {
        let mut editor = Editor::new(Document::from_text(Text::from_str(content)));
        editor
            .view
            .fit(80, 25, editor.document.text.line_count(), 0);
        editor.view.selections = SelectionSet::single(offset);
        editor
    }

    fn content(editor: &Editor) -> String {
        editor.document.text.to_string()
    }

    #[test]
    fn typing_lands_after_the_cursor() {
        let mut editor = editor("ac", 1);
        editor.insert("b");

        assert_eq!(content(&editor), "abc");
        assert_eq!(editor.view.selections.primary().head, 2);
    }

    #[test]
    fn typing_replaces_a_selection() {
        let mut editor = editor("hello world", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 5,
            goal_column: None,
        }
        .into();
        editor.insert("bye");

        assert_eq!(content(&editor), "bye world");
        assert!(editor.view.selections.primary().is_empty());
    }

    #[test]
    fn backspace_eats_one_character_and_delete_the_next() {
        let mut editor = editor("абв", 4);
        editor.delete(Motion::Grapheme(Direction::Backward));
        assert_eq!(content(&editor), "ав");

        editor.delete(Motion::Grapheme(Direction::Forward));
        assert_eq!(content(&editor), "а");
    }

    #[test]
    fn backspace_at_the_start_of_a_document_does_nothing() {
        let mut editor = editor("x", 0);
        editor.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(content(&editor), "x");
        assert!(!editor.document.is_modified());
    }

    #[test]
    fn deleting_a_word_stops_at_its_boundary() {
        let mut editor = editor("foo bar.baz", 11);
        editor.delete(Motion::Word(Direction::Backward));
        assert_eq!(content(&editor), "foo bar.");

        editor.delete(Motion::Word(Direction::Backward));
        assert_eq!(content(&editor), "foo bar");
    }

    #[test]
    fn every_cursor_types_at_once() {
        let mut editor = editor("a a a", 0);
        editor.view.selections.push(Selection::cursor(2));
        editor.view.selections.push(Selection::cursor(4));
        editor.insert("!");

        assert_eq!(content(&editor), "!a !a !a");
        assert_eq!(editor.view.selections.len(), 3);
    }

    #[test]
    fn undo_and_redo_walk_the_history() {
        let mut editor = editor("start", 5);
        editor.insert(" one");
        editor.insert(" two");
        assert_eq!(content(&editor), "start one two");

        editor.undo();
        assert_eq!(content(&editor), "start one");
        editor.undo();
        assert_eq!(content(&editor), "start");
        editor.undo();
        assert_eq!(content(&editor), "start");

        editor.redo();
        editor.redo();
        assert_eq!(content(&editor), "start one two");
    }

    #[test]
    fn undo_puts_the_cursors_back() {
        let mut editor = editor("a a", 1);
        editor.view.selections.push(Selection::cursor(3));
        editor.insert("!");
        assert_eq!(content(&editor), "a! a!");

        editor.undo();
        let heads: Vec<_> = editor
            .view
            .selections
            .iter()
            .map(|selection| selection.head)
            .collect();
        assert_eq!(heads, [1, 3]);
    }

    fn ranges(editor: &Editor) -> Vec<std::ops::Range<usize>> {
        editor
            .view
            .selections
            .iter()
            .map(Selection::range)
            .collect()
    }

    #[test]
    fn the_first_press_selects_the_word_under_the_cursor() {
        let mut editor = editor("foo bar foo", 1);
        editor.select_next_occurrence();

        assert_eq!(editor.view.selections.len(), 1);
        assert_eq!(editor.view.selections.primary().range(), 0..3);
    }

    #[test]
    fn a_cursor_resting_after_a_word_still_finds_it() {
        let mut editor = editor("foo bar", 3);
        editor.select_next_occurrence();

        assert_eq!(editor.view.selections.primary().range(), 0..3);
    }

    #[test]
    fn further_presses_add_occurrences_and_wrap() {
        let mut editor = editor("foo bar foo baz foo", 0);
        editor.select_next_occurrence();
        editor.select_next_occurrence();
        assert_eq!(ranges(&editor), [0..3, 8..11]);

        editor.select_next_occurrence();
        assert_eq!(ranges(&editor), [0..3, 8..11, 16..19]);

        // Everything is selected, so the wrap finds nothing new.
        editor.select_next_occurrence();
        assert_eq!(ranges(&editor), [0..3, 8..11, 16..19]);
    }

    #[test]
    fn a_whole_word_selection_skips_words_containing_it() {
        let mut editor = editor("foo foobar foo", 0);
        editor.select_next_occurrence();
        editor.select_next_occurrence();

        assert_eq!(ranges(&editor), [0..3, 11..14]);
    }

    #[test]
    fn a_word_selected_by_hand_is_taken_literally() {
        let mut editor = editor("foo foobar foo", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 3,
            goal_column: None,
        }
        .into();
        editor.select_next_occurrence();

        assert_eq!(ranges(&editor), [0..3, 4..7]);
    }

    #[test]
    fn a_partial_selection_matches_anywhere() {
        let mut editor = editor("foo foobar", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 2,
            goal_column: None,
        }
        .into();
        editor.select_next_occurrence();

        assert_eq!(ranges(&editor), [0..2, 4..6]);
    }

    #[test]
    fn selecting_all_occurrences_replaces_the_cursors() {
        let mut editor = editor("x = x + x", 0);
        editor.select_next_occurrence();
        editor.select_all_occurrences();

        assert_eq!(ranges(&editor), [0..1, 4..5, 8..9]);
    }

    #[test]
    fn every_occurrence_can_be_edited_at_once() {
        let mut editor = editor("id, id, id", 0);
        editor.select_next_occurrence();
        editor.select_all_occurrences();
        editor.insert("key");

        assert_eq!(content(&editor), "key, key, key");
    }

    #[test]
    fn cursors_stack_up_and_down() {
        let mut editor = editor("aaaa\nbbbb\ncccc", 6);
        editor.add_cursor(Direction::Forward);
        assert_eq!(ranges(&editor), [6..6, 11..11]);

        editor.add_cursor(Direction::Backward);
        assert_eq!(ranges(&editor), [1..1, 6..6, 11..11]);
    }

    #[test]
    fn cursors_do_not_stack_past_the_document() {
        let mut editor = editor("only", 2);
        editor.add_cursor(Direction::Backward);
        editor.add_cursor(Direction::Forward);

        assert_eq!(editor.view.selections.len(), 1);
    }

    #[test]
    fn a_stacked_cursor_keeps_the_goal_column_over_a_short_line() {
        let mut editor = editor("aaaa\nx\ncccc", 3);
        editor.add_cursor(Direction::Forward);
        editor.add_cursor(Direction::Forward);

        assert_eq!(ranges(&editor), [3..3, 6..6, 10..10]);
    }

    fn prompt_mut(editor: &mut Editor) -> &mut Prompt {
        editor
            .panel
            .as_mut()
            .and_then(Panel::as_prompt_mut)
            .expect("panel holds a prompt")
    }

    fn type_into_prompt(editor: &mut Editor, typed: &str) {
        for character in typed.chars() {
            prompt_mut(editor).insert(&character.to_string());
            editor.prompt_changed();
        }
    }

    fn indented(content: &str, indent: Indent, offset: usize) -> Editor {
        let mut editor = editor(content, offset);
        editor.document.indent = indent;
        editor
    }

    fn rust_editor(content: &str, offset: usize) -> Editor {
        let mut editor = editor(content, offset);
        editor.set_language(language::by_name("Rust").unwrap());
        editor
    }

    #[test]
    fn commenting_and_uncommenting_a_line() {
        let mut editor = rust_editor("    code();\n", 4);
        editor.toggle_comment();
        assert_eq!(content(&editor), "    // code();\n");

        editor.toggle_comment();
        assert_eq!(content(&editor), "    code();\n");
    }

    #[test]
    fn markers_line_up_with_the_shallowest_line() {
        let mut editor = rust_editor("  one();\n      two();\n", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 20,
            goal_column: None,
        }
        .into();
        editor.toggle_comment();

        assert_eq!(content(&editor), "  // one();\n  //     two();\n");
    }

    #[test]
    fn a_half_commented_block_becomes_fully_commented() {
        let mut editor = rust_editor("// one();\ntwo();\n", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 16,
            goal_column: None,
        }
        .into();
        editor.toggle_comment();

        assert_eq!(content(&editor), "// // one();\n// two();\n");
    }

    #[test]
    fn blank_lines_are_left_alone() {
        let mut editor = rust_editor("one();\n\ntwo();\n", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 14,
            goal_column: None,
        }
        .into();
        editor.toggle_comment();

        assert_eq!(content(&editor), "// one();\n\n// two();\n");
    }

    #[test]
    fn a_language_without_line_comments_says_so() {
        let mut editor = editor("{}\n", 0);
        editor.set_language(language::by_name("JSON").unwrap());
        editor.toggle_comment();

        assert_eq!(content(&editor), "{}\n");
        assert!(editor.message.is_some());
    }

    #[test]
    fn a_new_line_keeps_the_indentation() {
        let mut editor = editor("    deep\n", 8);
        editor.insert_newline();

        assert_eq!(content(&editor), "    deep\n    \n");
        assert_eq!(editor.view.selections.primary().head, 13);
    }

    #[test]
    fn only_the_indentation_before_the_cursor_is_copied() {
        let mut editor = editor("      deep\n", 3);
        editor.insert_newline();

        // Three spaces were copied; the other three stayed with the text.
        assert_eq!(content(&editor), "   \n      deep\n");
    }

    #[test]
    fn a_line_left_holding_only_indentation_is_emptied() {
        let mut editor = editor("    \n", 4);
        editor.insert_newline();

        assert_eq!(content(&editor), "\n    \n");
    }

    #[test]
    fn every_cursor_keeps_its_own_indentation() {
        let mut editor = editor("  one\n    two\n", 5);
        editor.view.selections.push(Selection::cursor(13));
        editor.insert_newline();

        assert_eq!(content(&editor), "  one\n  \n    two\n    \n");
    }

    #[test]
    fn a_new_line_replaces_what_was_selected() {
        let mut editor = editor("    keep drop\n", 0);
        editor.view.selections = Selection {
            anchor: 9,
            head: 13,
            goal_column: None,
        }
        .into();
        editor.insert_newline();

        assert_eq!(content(&editor), "    keep \n    \n");
    }

    #[test]
    fn tab_moves_to_the_next_stop_rather_than_a_whole_level() {
        let mut editor = indented("ab", Indent::Spaces(4), 2);
        editor.indent();
        assert_eq!(content(&editor), "ab  ");

        editor.indent();
        assert_eq!(content(&editor), "ab      ");
    }

    #[test]
    fn tab_inserts_a_tab_when_the_file_uses_them() {
        let mut editor = indented("ab", Indent::Tabs, 2);
        editor.indent();
        assert_eq!(content(&editor), "ab\t");
    }

    #[test]
    fn a_selection_across_lines_indents_them_all() {
        let mut editor = indented("one\ntwo\nthree\n", Indent::Spaces(2), 0);
        editor.view.selections = Selection {
            anchor: 1,
            head: 9,
            goal_column: None,
        }
        .into();
        editor.indent();

        assert_eq!(content(&editor), "  one\n  two\n  three\n");
        // Snapped to the line start, so the new indentation is inside the
        // selection rather than hanging off its left edge.
        assert_eq!(editor.view.selections.primary().range(), 0..15);
    }

    #[test]
    fn a_line_under_two_cursors_is_indented_once() {
        let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 5,
            goal_column: None,
        }
        .into();
        // A second cursor on a line the selection already covers.
        editor.view.selections.push(Selection::cursor(6));
        editor.indent();

        assert_eq!(content(&editor), "  one\n  two\n");
    }

    #[test]
    fn a_selection_ending_on_a_line_start_leaves_that_line_alone() {
        let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
        // What Ctrl+L produces: the line and the break after it.
        editor.view.selections = Selection {
            anchor: 0,
            head: 4,
            goal_column: None,
        }
        .into();
        editor.indent();

        assert_eq!(content(&editor), "  one\ntwo\n");
    }

    #[test]
    fn any_selection_at_all_means_indent_the_line() {
        let mut editor = indented("one two\n", Indent::Spaces(2), 0);
        editor.view.selections = Selection {
            anchor: 4,
            head: 7,
            goal_column: None,
        }
        .into();
        editor.indent();

        // The selected word is indented with its line, not replaced by a tab.
        assert_eq!(content(&editor), "  one two\n");
    }

    #[test]
    fn bare_cursors_get_a_tab_stop_each() {
        let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
        editor.view.selections.push(Selection::cursor(4));
        editor.indent();

        assert_eq!(content(&editor), "  one\n  two\n");
    }

    #[test]
    fn backspace_in_the_indentation_removes_a_whole_level() {
        let mut editor = indented("        code\n", Indent::Spaces(4), 8);
        editor.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(content(&editor), "    code\n");
    }

    #[test]
    fn backspace_lands_on_the_previous_stop_not_a_fixed_step() {
        let mut editor = indented("      code\n", Indent::Spaces(4), 6);
        editor.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(content(&editor), "    code\n");
    }

    #[test]
    fn backspace_after_text_removes_one_character() {
        let mut editor = indented("    a    b", Indent::Spaces(4), 9);
        editor.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(content(&editor), "    a   b");
    }

    #[test]
    fn backspace_with_tabs_is_left_alone() {
        let mut editor = indented("\t\tcode\n", Indent::Tabs, 2);
        editor.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(content(&editor), "\tcode\n");
    }

    #[test]
    fn outdent_removes_one_level_and_no_more() {
        let mut editor = indented("      deep\n", Indent::Spaces(2), 8);
        editor.outdent();
        assert_eq!(content(&editor), "    deep\n");

        editor.outdent();
        assert_eq!(content(&editor), "  deep\n");
    }

    #[test]
    fn outdent_stops_at_the_left_margin() {
        let mut editor = indented(" x\n", Indent::Spaces(4), 2);
        editor.outdent();
        assert_eq!(content(&editor), "x\n");

        // Nothing left to remove, so nothing is recorded: one undo is enough.
        editor.outdent();
        assert_eq!(content(&editor), "x\n");

        editor.undo();
        assert_eq!(content(&editor), " x\n");
    }

    #[test]
    fn indenting_is_one_undo_step() {
        let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 7,
            goal_column: None,
        }
        .into();
        editor.indent();
        assert_eq!(content(&editor), "  one\n  two\n");

        editor.undo();
        assert_eq!(content(&editor), "one\ntwo\n");
    }

    #[test]
    fn undoing_back_to_what_was_saved_is_not_a_change() {
        let path = std::env::temp_dir().join(format!("ned-{}-modified", std::process::id()));
        std::fs::write(&path, "start").unwrap();

        let mut editor = Editor::new(Document::open(&path).unwrap());
        editor.view.selections = SelectionSet::single(5);
        assert!(!editor.document.is_modified());

        editor.insert("!");
        assert!(editor.document.is_modified());

        editor.save();
        assert!(!editor.document.is_modified());

        // Back to what the file holds, so there is nothing to warn about.
        editor.undo();
        assert!(editor.document.is_modified());
        editor.redo();
        assert!(!editor.document.is_modified());

        // And an edit undone past the save is a change again.
        editor.insert("?");
        assert!(editor.document.is_modified());
        editor.undo();
        assert!(!editor.document.is_modified());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn typing_after_a_save_starts_its_own_revision() {
        let path = std::env::temp_dir().join(format!("ned-{}-revision", std::process::id()));
        std::fs::write(&path, "").unwrap();

        let mut editor = Editor::new(Document::open(&path).unwrap());
        type_text(&mut editor, "wor");
        editor.save();

        // The same word carries on, but it must not join the group that was
        // written to disk.
        type_text(&mut editor, "d");
        assert!(editor.document.is_modified());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn an_unchanged_document_quits_at_once() {
        let mut editor = editor("text", 0);
        editor.request_quit();

        assert!(editor.quit);
        assert!(editor.panel.is_none());
    }

    #[test]
    fn unsaved_changes_ask_first() {
        let mut editor = editor("text", 0);
        editor.insert("!");
        editor.request_quit();

        assert!(!editor.quit);
        assert!(matches!(editor.panel, Some(Panel::Confirm(_))));
    }

    #[test]
    fn the_question_can_be_answered_or_dropped() {
        let mut editor = editor("text", 0);
        editor.insert("!");

        editor.request_quit();
        editor.close_panel();
        assert!(!editor.quit);

        editor.request_quit();
        editor.answer('q');
        assert!(editor.quit);
        assert!(editor.panel.is_none());
    }

    #[test]
    fn the_answer_key_is_the_physical_one() {
        let mut editor = editor("text", 0);
        editor.insert("!");
        editor.request_quit();

        // `й` sits where `q` does on a US keyboard.
        editor.answer('й');
        assert!(editor.quit);
    }

    #[test]
    fn a_save_that_fails_cancels_the_quit() {
        // A scratch buffer has nowhere to be written to.
        let mut editor = editor("text", 0);
        editor.insert("!");
        editor.request_quit();
        editor.answer('s');

        assert!(!editor.quit);
        assert!(editor.message.is_some());
    }

    #[test]
    fn goto_line_moves_the_cursor_to_its_start() {
        let mut editor = editor("one\ntwo\nthree\n", 0);
        editor.open_prompt(Purpose::GotoLine);
        type_into_prompt(&mut editor, "3");
        editor.submit_prompt(false);

        assert_eq!(editor.view.selections.primary().head, 8);
        assert!(editor.panel.is_none());
    }

    #[test]
    fn goto_line_complains_about_nonsense() {
        let mut editor = editor("one\ntwo\n", 0);
        editor.open_prompt(Purpose::GotoLine);
        type_into_prompt(&mut editor, "later");
        editor.submit_prompt(false);

        assert_eq!(editor.view.selections.primary().head, 0);
        assert!(editor.message.is_some());
    }

    #[test]
    fn select_all_covers_the_document_and_stays_put() {
        let mut editor = editor(&"line\n".repeat(100), 0);
        editor.view.fit(80, 11, editor.document.text.line_count(), 0);
        editor.move_selections(Motion::Document(Direction::Forward), false);
        let top = editor.view.top_line;

        editor.select_all();
        assert_eq!(
            editor.view.selections.primary().range(),
            0..editor.document.text.len()
        );
        assert_eq!(editor.view.top_line, top);
    }

    #[test]
    fn scrolling_leaves_the_cursors_alone() {
        let mut editor = editor(&"line\n".repeat(100), 0);
        editor.view.fit(80, 11, editor.document.text.line_count(), 0);

        editor.scroll(5);
        assert_eq!(editor.view.top_line, 5);
        assert_eq!(editor.view.selections.primary().head, 0);

        editor.scroll(-99);
        assert_eq!(editor.view.top_line, 0);
    }

    #[test]
    fn opening_search_takes_the_selected_text() {
        let mut editor = editor("alpha beta gamma", 0);
        editor.view.selections = Selection {
            anchor: 6,
            head: 10,
            goal_column: None,
        }
        .into();
        editor.open_prompt(Purpose::Search);

        let prompt = editor.prompt().unwrap();
        assert_eq!(prompt.contents(0), "beta");
        // Seeded and selected, so the next keystroke replaces it.
        assert_eq!(prompt.fields[0].selection(), 0..4);
        // The match under the cursor stays put rather than jumping ahead.
        assert_eq!(editor.view.selections.primary().range(), 6..10);
    }

    #[test]
    fn a_selection_across_lines_does_not_seed_the_query() {
        let mut editor = editor("one\ntwo", 0);
        editor.view.selections = Selection {
            anchor: 0,
            head: 7,
            goal_column: None,
        }
        .into();
        editor.open_prompt(Purpose::Search);

        assert_eq!(editor.prompt().unwrap().contents(0), "");
    }

    #[test]
    fn search_follows_every_keystroke() {
        let mut editor = editor("alpha beta gamma", 0);
        editor.open_prompt(Purpose::Search);

        type_into_prompt(&mut editor, "b");
        assert_eq!(editor.view.selections.primary().range(), 6..7);

        type_into_prompt(&mut editor, "eta");
        assert_eq!(editor.view.selections.primary().range(), 6..10);
    }

    #[test]
    fn search_starts_over_from_where_it_opened() {
        let mut editor = editor("aa ab", 0);
        editor.open_prompt(Purpose::Search);

        // Typing `a` then `b` must not chase the first match forward.
        type_into_prompt(&mut editor, "ab");
        assert_eq!(editor.view.selections.primary().range(), 3..5);
    }

    #[test]
    fn escape_closes_the_search_where_it_left_off() {
        let mut editor = editor("alpha beta", 2);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "beta");
        editor.close_panel();

        assert_eq!(editor.view.selections.primary().range(), 6..10);
        assert!(editor.panel.is_none());
        assert!(editor.matches.is_empty());
    }

    #[test]
    fn enter_walks_the_matches_and_wraps() {
        let mut editor = editor("x . x . x", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "x");
        assert_eq!(editor.view.selections.primary().range(), 0..1);

        editor.submit_prompt(false);
        assert_eq!(editor.view.selections.primary().range(), 4..5);
        editor.submit_prompt(false);
        assert_eq!(editor.view.selections.primary().range(), 8..9);
        editor.submit_prompt(false);
        assert_eq!(editor.view.selections.primary().range(), 0..1);

        editor.submit_prompt(true);
        assert_eq!(editor.view.selections.primary().range(), 8..9);

        // Walking the hits keeps the panel open.
        assert!(editor.panel.is_some());
    }

    #[test]
    fn every_hit_is_collected_and_counted() {
        let mut editor = editor("x . x . x", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "x");

        assert_eq!(editor.matches, [0..1, 4..5, 8..9]);
        assert_eq!(editor.panel_status(), "Found: 1/3");

        editor.submit_prompt(false);
        assert_eq!(editor.panel_status(), "Found: 2/3");
    }

    #[test]
    fn every_hit_can_become_a_cursor() {
        let mut editor = editor("id, id, id", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "id");
        editor.submit_prompt(false);

        editor.select_all_matches();
        assert_eq!(ranges(&editor), [0..2, 4..6, 8..10]);
        // The keyboard goes back to the text, which is the point.
        assert!(editor.panel.is_none());
        // The hit we were standing on stays the primary one.
        assert_eq!(editor.view.selections.primary().range(), 4..6);

        editor.insert("key");
        assert_eq!(content(&editor), "key, key, key");
    }

    #[test]
    fn narrowing_a_query_finds_what_scanning_would() {
        let mut editor = editor("ab abc abcd", 0);
        editor.open_prompt(Purpose::Search);

        type_into_prompt(&mut editor, "a");
        assert_eq!(editor.matches, [0..1, 3..4, 7..8]);

        type_into_prompt(&mut editor, "b");
        assert_eq!(editor.matches, [0..2, 3..5, 7..9]);

        type_into_prompt(&mut editor, "cd");
        assert_eq!(editor.matches, vec![7..11]);

        // Backspace cannot narrow, so this one has to read the document again.
        prompt_mut(&mut editor).delete(Motion::Grapheme(Direction::Backward));
        editor.prompt_changed();
        assert_eq!(editor.matches, [3..6, 7..10]);
    }

    #[test]
    fn narrowing_does_not_cut_a_character_in_half() {
        let mut editor = editor("аб абв", 0);
        editor.open_prompt(Purpose::Search);

        type_into_prompt(&mut editor, "а");
        assert_eq!(editor.matches, [0..2, 5..7]);

        type_into_prompt(&mut editor, "бв");
        assert_eq!(editor.matches, vec![5..11]);
    }

    #[test]
    fn a_query_with_no_hits_counts_zero() {
        let mut editor = editor("nothing here", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "zzz");

        assert!(editor.matches.is_empty());
        assert_eq!(editor.panel_status(), "Found: none");
    }

    #[test]
    fn f3_still_works_once_the_strip_is_closed() {
        let mut editor = editor("x . x", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "x");
        editor.close_panel();

        editor.search_next(Direction::Forward);
        assert_eq!(editor.view.selections.primary().range(), 4..5);
    }

    #[test]
    fn replace_changes_every_occurrence_in_one_step() {
        let mut editor = editor("one two one", 0);
        editor.open_prompt(Purpose::Replace);
        type_into_prompt(&mut editor, "one");
        prompt_mut(&mut editor).focus_next_field();
        type_into_prompt(&mut editor, "1");
        editor.submit_prompt(false);

        assert_eq!(content(&editor), "1 two 1");

        editor.undo();
        assert_eq!(content(&editor), "one two one");
    }

    #[test]
    fn selecting_a_line_takes_its_break_too() {
        let mut editor = editor("first\nsecond\nthird\n", 8);
        editor.select_line();

        assert_eq!(editor.view.selections.primary().range(), 6..13);
    }

    #[test]
    fn selecting_again_swallows_the_next_line() {
        let mut editor = editor("first\nsecond\nthird\n", 8);
        editor.select_line();
        editor.select_line();

        assert_eq!(editor.view.selections.primary().range(), 6..19);
    }

    #[test]
    fn a_selection_across_lines_expands_to_all_of_them() {
        let mut editor = editor("first\nsecond\nthird\n", 0);
        editor.view.selections = Selection {
            anchor: 3,
            head: 8,
            goal_column: None,
        }
        .into();
        editor.select_line();

        assert_eq!(editor.view.selections.primary().range(), 0..13);
    }

    #[test]
    fn selecting_the_last_line_stops_at_the_end() {
        let mut editor = editor("first\nlast", 7);
        editor.select_line();
        assert_eq!(editor.view.selections.primary().range(), 6..10);

        editor.select_line();
        assert_eq!(editor.view.selections.primary().range(), 6..10);
    }

    #[test]
    fn selected_lines_can_be_cut_at_every_cursor() {
        let mut editor = editor("one\ntwo\nthree\n", 0);
        editor.view.selections.push(Selection::cursor(8));
        editor.select_line();
        editor.cut();

        assert_eq!(content(&editor), "two\n");
    }

    #[test]
    fn copying_and_pasting_a_selection() {
        let mut editor = editor("hello world", 0);
        editor.select_next_occurrence();
        editor.copy();

        editor.view.selections = SelectionSet::single(11);
        editor.paste();
        assert_eq!(content(&editor), "hello worldhello");
    }

    #[test]
    fn pasting_replaces_a_selection() {
        let mut editor = editor("keep drop", 5);
        editor.select_next_occurrence();
        editor.copy();
        editor.paste();

        assert_eq!(content(&editor), "keep drop");
        assert_eq!(editor.view.selections.primary().head, 9);
    }

    #[test]
    fn cutting_removes_what_it_took() {
        let mut editor = editor("hello world", 6);
        editor.select_next_occurrence();
        editor.cut();

        assert_eq!(content(&editor), "hello ");
        assert_eq!(editor.clipboard.text(), "world");
    }

    #[test]
    fn copying_without_a_selection_takes_the_whole_line() {
        let mut editor = editor("first\nsecond\n", 8);
        editor.copy();

        assert_eq!(editor.clipboard.text(), "second\n");
        assert!(editor.clipboard.holds_lines());
    }

    #[test]
    fn pasting_a_line_puts_it_above_and_keeps_the_column() {
        let mut editor = editor("first\nsecond\n", 8);
        editor.copy();
        editor.paste();

        assert_eq!(content(&editor), "first\nsecond\nsecond\n");
        // The cursor stayed on the same character of its own line.
        assert_eq!(editor.view.selections.primary().head, 15);
    }

    #[test]
    fn a_last_line_without_a_break_still_copies_as_a_line() {
        let mut editor = editor("only", 2);
        editor.copy();

        assert_eq!(editor.clipboard.text(), "only\n");
    }

    #[test]
    fn cutting_a_line_removes_it_whole() {
        let mut editor = editor("first\nsecond\nthird\n", 8);
        editor.cut();

        assert_eq!(content(&editor), "first\nthird\n");
    }

    #[test]
    fn a_line_under_two_cursors_is_cut_once() {
        let mut editor = editor("one\ntwo\nthree\n", 0);
        editor.view.selections.push(Selection::cursor(2));
        editor.cut();

        assert_eq!(content(&editor), "two\nthree\n");
        assert_eq!(editor.clipboard.text(), "one\n");
    }

    #[test]
    fn a_line_under_two_cursors_is_pasted_once() {
        let mut editor = editor("one\ntwo\n", 0);
        editor.copy();
        editor.view.selections.push(Selection::cursor(2));
        editor.paste();

        assert_eq!(content(&editor), "one\none\ntwo\n");
    }

    #[test]
    fn cursors_on_separate_lines_still_get_a_line_each() {
        let mut editor = editor("one\ntwo\nthree\n", 0);
        editor.view.selections.push(Selection::cursor(4));
        editor.cut();

        assert_eq!(content(&editor), "three\n");
        assert_eq!(editor.clipboard.text(), "one\ntwo\n");
    }

    #[test]
    fn each_cursor_pastes_its_own_piece() {
        let mut editor = editor("aa bb", 0);
        editor.view.selections = SelectionSet::from_selections(
            vec![
                Selection {
                    anchor: 0,
                    head: 2,
                    goal_column: None,
                },
                Selection {
                    anchor: 3,
                    head: 5,
                    goal_column: None,
                },
            ],
            0,
        );
        editor.copy();
        editor.paste();

        assert_eq!(content(&editor), "aa bb");
        assert_eq!(editor.clipboard.pieces_for(2), ["aa", "bb"]);
    }

    #[test]
    fn one_cursor_pastes_everything_that_was_copied() {
        let mut editor = editor("aa bb", 0);
        editor.view.selections = SelectionSet::from_selections(
            vec![
                Selection {
                    anchor: 0,
                    head: 2,
                    goal_column: None,
                },
                Selection {
                    anchor: 3,
                    head: 5,
                    goal_column: None,
                },
            ],
            0,
        );
        editor.copy();

        editor.view.selections = SelectionSet::single(5);
        editor.paste();
        assert_eq!(content(&editor), "aa bbaa\nbb");
    }

    #[test]
    fn pasting_is_one_undo_step() {
        let mut editor = editor("word", 0);
        editor.select_next_occurrence();
        editor.copy();
        editor.view.selections = SelectionSet::single(4);
        editor.paste();
        assert_eq!(content(&editor), "wordword");

        editor.undo();
        assert_eq!(content(&editor), "word");
    }

    fn type_text(editor: &mut Editor, text: &str) {
        for character in text.chars() {
            editor.insert(&character.to_string());
        }
    }

    #[test]
    fn typing_a_word_undoes_in_one_step() {
        let mut editor = editor("", 0);
        type_text(&mut editor, "hello");

        editor.undo();
        assert_eq!(content(&editor), "");
    }

    #[test]
    fn a_word_boundary_starts_a_new_step() {
        let mut editor = editor("", 0);
        type_text(&mut editor, "foo bar");

        editor.undo();
        assert_eq!(content(&editor), "foo ");
        editor.undo();
        assert_eq!(content(&editor), "foo");
        editor.undo();
        assert_eq!(content(&editor), "");
    }

    #[test]
    fn moving_the_cursor_starts_a_new_step() {
        let mut editor = editor("", 0);
        type_text(&mut editor, "ab");
        editor.move_selections(Motion::Grapheme(Direction::Backward), false);
        editor.insert("c");
        assert_eq!(content(&editor), "acb");

        editor.undo();
        assert_eq!(content(&editor), "ab");
    }

    #[test]
    fn a_line_break_stands_alone() {
        let mut editor = editor("", 0);
        type_text(&mut editor, "ab");
        editor.insert("\n");
        type_text(&mut editor, "cd");

        editor.undo();
        assert_eq!(content(&editor), "ab\n");
        editor.undo();
        assert_eq!(content(&editor), "ab");
    }

    #[test]
    fn pasted_text_stands_alone() {
        let mut editor = editor("", 0);
        type_text(&mut editor, "ab");
        editor.insert("pasted");

        editor.undo();
        assert_eq!(content(&editor), "ab");
    }

    #[test]
    fn backspaces_undo_together() {
        let mut editor = editor("hello", 5);
        for _ in 0..3 {
            editor.delete(Motion::Grapheme(Direction::Backward));
        }
        assert_eq!(content(&editor), "he");

        editor.undo();
        assert_eq!(content(&editor), "hello");
    }

    #[test]
    fn typing_after_deleting_starts_a_new_step() {
        let mut editor = editor("ab", 2);
        editor.delete(Motion::Grapheme(Direction::Backward));
        editor.insert("c");

        editor.undo();
        assert_eq!(content(&editor), "a");
    }

    #[test]
    fn redo_restores_a_whole_group() {
        let mut editor = editor("", 0);
        type_text(&mut editor, "word");
        editor.undo();
        assert_eq!(content(&editor), "");

        editor.redo();
        assert_eq!(content(&editor), "word");
        assert_eq!(editor.view.selections.primary().head, 4);
    }

    #[test]
    fn editing_after_undo_forgets_the_redo() {
        let mut editor = editor("x", 1);
        editor.insert("y");
        editor.undo();
        editor.insert("z");

        assert_eq!(content(&editor), "xz");
        editor.redo();
        assert_eq!(content(&editor), "xz");
    }
}
