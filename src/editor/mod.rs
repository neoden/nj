//! The editor as a whole: what commands mean, which buffers and views there
//! are, what panel is open, and the one boundary everything crosses.

pub mod clipboard;
pub mod command;
mod file;
mod find;
pub mod input;
pub mod job;
pub mod key;
pub mod keymap;
pub mod lines;
pub mod panel;
pub mod screen;
pub mod style;
pub mod view;
pub mod workspace;

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use crate::buffer::language::{self, Language};
use crate::buffer::{Buffer, EditKind};
use crate::text::Text;
use crate::text::layout;
use crate::text::movement::{self, Class, Direction, Motion};
use crate::text::search::{Query, Session};
use crate::text::selection::{Selection, SelectionSet};
use crate::text::transaction::{CursorAfter, Transaction};
use clipboard::{Clipboard, Desktop};
use command::{Command, Edit, File, PanelCommand, Select};
use find::Occurrences;
use job::{JobId, Jobs, Message, Report};
use panel::picker::{self, Candidate, Pick, Picker};
use panel::prompt::{Field, Prompt, Purpose};
use panel::{After, Panel};
use screen::Screen;
use view::View;
use workspace::{BufferId, Workspace};

/// How the editor is leaving: the usual way, handing on what was saved, or
/// aborting — nothing written, and a failure code for whoever is waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Quit,
    Abort,
}

/// The editor behind its boundary (ADR 16): commands go in through
/// `execute`, reports from its own workers through `receive`, and what comes
/// out is read-only. No `&mut` to anything inside leaves this type, and no
/// thread but the one holding it ever touches it (ADR 21).
pub struct Editor {
    workspace: Workspace,
    /// How the terminal was last divided; what the focused view's area came
    /// from.
    screen: Screen,
    clipboard: Clipboard,
    panel: Option<Panel>,
    /// Shown on the status bar until the next command.
    message: Option<String>,
    /// The search in progress. A prompt starts it and it stays when the
    /// prompt closes, so its hits are found again whenever the text changes.
    search: Option<Session>,
    /// The query searched for last, for when no search is going on: `F3`
    /// brings it back, and a search prompt opens holding it.
    last: Option<Query>,
    /// Set once the loop should stop; how says what the process ends with.
    exit: Option<Exit>,
    occurrences: Option<Occurrences>,
    /// The clipboard outside the process, when there is one to talk to.
    desktop: Option<Box<dyn Desktop>>,
    jobs: Jobs,
    /// The job keeping an eye on each buffer's file, to be cancelled when
    /// the buffer gets another.
    watchers: HashMap<BufferId, JobId>,
    /// Where everything arrives: input forwarded from the terminal, and the
    /// reports of the jobs.
    inbox: Receiver<Message>,
}

impl Editor {
    pub fn new(buffer: Buffer) -> Self {
        let (jobs, inbox) = Jobs::new();
        let mut editor = Self {
            workspace: Workspace::single(buffer),
            screen: Screen::default(),
            clipboard: Clipboard::default(),
            panel: None,
            message: None,
            search: None,
            last: None,
            exit: None,
            occurrences: None,
            desktop: None,
            jobs,
            watchers: HashMap::new(),
            inbox,
        };
        editor.watch(editor.workspace.focused_buffer());
        editor
    }

    /// For whoever else has something to put in the inbox: the thread
    /// reading the terminal.
    pub fn sender(&self) -> Sender<Message> {
        self.jobs.sender()
    }

    /// Blocks until something arrives. `None` only if every sender is gone,
    /// which cannot happen while the jobs hold one.
    pub fn wait(&self) -> Option<Message> {
        self.inbox.recv().ok()
    }

    /// Whatever else has arrived already, without waiting.
    pub fn pending(&self) -> Option<Message> {
        self.inbox.try_recv().ok()
    }

    /// The editor's own work coming back to it. Whether it is still wanted
    /// is decided here: the job may have been cancelled, the panel closed.
    pub fn receive(&mut self, report: Report) {
        match report {
            Report::Candidates { job, batch } => {
                if let Some(Panel::Picker(picker)) = &mut self.panel
                    && picker.job == job
                {
                    picker.accept(batch);
                }
            }
            Report::FileChanged(id) => {
                if self.workspace.buffer(id).changed_on_disk() {
                    self.message = Some("File changed on disk".to_owned());
                }
            }
            Report::Ended(job) => self.jobs.ended(job),
        }
    }

    /// Handles every report the tasks in flight will send, and returns when
    /// they have all ended. For tests, which want a picker filled before
    /// they type into it; the loop never needs it.
    pub fn finish_jobs(&mut self) {
        while self.jobs.any_pending() {
            match self.wait() {
                Some(Message::Report(report)) => self.receive(report),
                Some(Message::Input(_)) => {}
                None => return,
            }
        }
    }

    /// The buffer and the view in focus, which is what nearly every command
    /// works on. Read through these; a command that reads one while changing
    /// the other takes both from the workspace at once.
    pub fn buffer(&self) -> &Buffer {
        self.workspace.focus().1
    }

    fn buffer_mut(&mut self) -> &mut Buffer {
        self.workspace.focus_edit().1
    }

    pub fn view(&self) -> &View {
        self.workspace.focus().0
    }

    fn view_mut(&mut self) -> &mut View {
        self.workspace.focus_mut().0
    }

    pub fn panel(&self) -> Option<&Panel> {
        self.panel.as_ref()
    }

    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// The hits of the search in progress, for the renderer to underline.
    pub fn matches(&self) -> &[Range<usize>] {
        self.search.as_ref().map_or(&[], Session::hits)
    }

    /// Whether a search is in progress: one with something to look for,
    /// whether or not it has found any.
    pub fn has_search(&self) -> bool {
        self.search
            .as_ref()
            .is_some_and(|search| !search.query().is_empty())
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    /// Set once the loop should stop, and how the process then ends.
    pub fn exit(&self) -> Option<Exit> {
        self.exit
    }

    /// Something the process wants the user to know, from outside any
    /// command: the terminal's shortcomings, say.
    pub fn say(&mut self, message: impl Into<String>) {
        self.message = Some(message.into());
    }

    /// Divides the terminal and hands the focused view its share.
    pub fn resize(&mut self, term_width: usize, term_height: usize) {
        let panel = self.panel_height();
        let (view, buffer) = self.workspace.focus_mut();
        let line_count = buffer.text().line_count();
        self.screen = Screen::divide(
            term_width,
            term_height,
            view::gutter_width(line_count),
            panel,
        );
        view.resize(self.screen.text, buffer);
    }

    /// The only way in (ADR 16). Routes to the method that does the work;
    /// what a command means is decided here and nowhere else.
    pub fn execute(&mut self, command: Command) {
        // A message answers the last command; the next one supersedes it.
        // Scrolling answers nothing, and a wheel tick must not take the
        // message with it.
        if !matches!(command, Command::Scroll(_)) {
            self.message = None;
        }

        match command {
            // These go wherever the keyboard is: an open panel's field, or
            // the text. A panel without a field swallows them.
            Command::Move { motion, extend } => self.at_focus(
                |field| {
                    field.move_cursor(motion, extend);
                    false
                },
                |editor| editor.move_selections(motion, extend),
            ),
            Command::Edit(Edit::Insert(text)) => self.at_focus(
                |field| {
                    field.insert(&text);
                    true
                },
                |editor| editor.insert(&text, insert_kind(&text)),
            ),
            Command::Edit(Edit::Delete(motion)) => self.at_focus(
                |field| {
                    field.delete(motion);
                    true
                },
                |editor| editor.delete(motion),
            ),
            Command::Select(Select::All) => self.at_focus(
                |field| {
                    field.select_all();
                    false
                },
                |editor| editor.select_all(),
            ),
            // The hits get walked while a query is being typed and once its
            // prompt has closed. A panel that is not a search has nothing to
            // find.
            Command::FindNext(direction) => match (self.searching(), self.panel.is_some()) {
                (true, _) => self.go_to_match(direction),
                (false, false) => self.search_next(direction),
                (false, true) => {}
            },
            Command::Select(Select::AllOccurrences) => {
                match (self.searching(), self.panel.is_some()) {
                    (true, _) => self.select_all_matches(),
                    // What is lit is what gets the cursors, wherever the
                    // cursor happens to stand.
                    (false, false) if self.has_search() => self.select_all_matches(),
                    (false, false) => self.select_all_occurrences(),
                    (false, true) => {}
                }
            }

            Command::EndSearch => self.end_search(),
            Command::Scroll(rows) => self.scroll(rows),
            Command::Quit => self.request_quit(),
            Command::Abort => self.request_abort(),

            Command::Edit(Edit::Newline) => self.insert_newline(),
            Command::Edit(Edit::Indent) => self.indent(),
            Command::Edit(Edit::Outdent) => self.outdent(),
            Command::Edit(Edit::ToggleComment) => self.toggle_comment(),
            Command::Edit(Edit::Undo) => self.undo(),
            Command::Edit(Edit::Redo) => self.redo(),
            Command::Edit(Edit::Copy) => self.copy(),
            Command::Edit(Edit::Cut) => self.cut(),
            Command::Edit(Edit::Paste) => self.paste(),

            Command::Select(Select::Line) => self.select_line(),
            Command::Select(Select::Cancel) => self.cancel_selection(),
            Command::Select(Select::RemoveSecondary) => self.remove_secondary_cursors(),
            Command::Select(Select::AddCursor(direction)) => self.add_cursor(direction),
            Command::Select(Select::NextOccurrence) => self.select_next_occurrence(),
            Command::Select(Select::Set(selections)) => self.set_selections(selections),
            Command::Select(Select::Click { row, column, add }) => self.click(row, column, add),
            Command::Select(Select::Drag { row, column }) => self.drag(row, column),

            Command::File(File::Save) => self.save(After::Stay),
            Command::File(File::SaveAs(path)) => self.save_as(path, After::Stay),
            Command::File(File::Open(path)) => self.open(path),
            Command::File(File::SetLanguage(name)) => match language::by_name(&name) {
                Some(language) => self.set_language(language),
                None => self.message = Some(format!("No language called {name}")),
            },

            Command::Panel(PanelCommand::Prompt(purpose)) => self.open_prompt(purpose),
            Command::Panel(PanelCommand::Picker(pick)) => self.open_picker(pick),
            Command::Panel(PanelCommand::Close) => self.close_panel(),
            Command::Panel(PanelCommand::Submit { backwards }) => self.submit_prompt(backwards),
            Command::Panel(PanelCommand::Pick) => self.accept_pick(),
            Command::Panel(PanelCommand::Answer(answer)) => self.answer(answer),
            Command::Panel(PanelCommand::NextField) => {
                if let Some(Panel::Prompt(prompt)) = &mut self.panel {
                    prompt.focus_next_field();
                }
            }
            Command::Panel(PanelCommand::MoveChoice(delta)) => {
                if let Some(Panel::Picker(picker)) = &mut self.panel {
                    picker.move_selection(delta);
                }
            }
        }
    }

    /// Runs a command where the keyboard is: on an open panel's field, on
    /// the text when no panel is open, and nowhere when the panel has no
    /// field. `on_field` reports whether it changed the contents.
    fn at_focus(
        &mut self,
        on_field: impl FnOnce(&mut Field) -> bool,
        on_text: impl FnOnce(&mut Self),
    ) {
        let edited = match &mut self.panel {
            Some(panel) => panel.field_mut().is_some_and(on_field),
            None => {
                on_text(self);
                false
            }
        };
        if edited {
            self.field_edited();
        }
    }

    /// An edited query sifts a picker's list as it is typed. A prompt's
    /// fields drive nothing until `Enter`.
    fn field_edited(&mut self) {
        if let Some(Panel::Picker(picker)) = &mut self.panel {
            picker.refilter();
        }
    }

    /// Offsets from outside are not trusted: a socket may send any number,
    /// and a number past the end or inside a character is a panic later.
    fn set_selections(&mut self, selections: SelectionSet) {
        let (view, buffer) = self.workspace.focus_mut();
        let text = buffer.text();
        view.selections = selections.mapped(|selection| Selection {
            anchor: text.snap(selection.anchor),
            head: text.snap(selection.head),
            goal_column: None,
        });
        view.scroll_to_cursor(buffer);
    }

    /// Screen cells into the view's own coordinates: past its gutter, down
    /// from its top. A click outside the text is nobody's.
    fn cell_in_view(&self, row: usize, column: usize) -> Option<(usize, usize)> {
        let view = self.view();
        let row = row.checked_sub(view.area.y)?;
        (row < view.height()).then(|| (row, column.saturating_sub(view.area.x + view.gutter)))
    }

    fn click(&mut self, row: usize, column: usize, add: bool) {
        let Some((row, column)) = self.cell_in_view(row, column) else {
            return;
        };
        // A click in the text takes the keyboard back there: a field or a
        // list gives way to it. A question does not, having to be answered.
        match &self.panel {
            Some(Panel::Confirm(_)) => return,
            Some(_) => self.close_panel(),
            None => {}
        }
        let (view, buffer) = self.workspace.focus_mut();
        let offset = view.offset_at_screen(buffer, row, column);
        match add {
            true => view.selections.push(Selection::cursor(offset)),
            false => view.selections = SelectionSet::single(offset),
        }
    }

    /// A drag follows a click, which closed any panel that gives way. One
    /// still open is a question, or the button went down outside the text.
    fn drag(&mut self, row: usize, column: usize) {
        if self.panel.is_some() {
            return;
        }
        let Some((row, column)) = self.cell_in_view(row, column) else {
            return;
        };
        let (view, buffer) = self.workspace.focus_mut();
        view.selections = Selection {
            anchor: view.selections.primary().anchor,
            head: view.offset_at_screen(buffer, row, column),
            goal_column: None,
        }
        .into();
        view.scroll_to_cursor(buffer);
    }

    fn open_prompt(&mut self, purpose: Purpose) {
        self.close_panel();
        let mut prompt = Prompt::new(purpose.clone());

        if purpose.searches() {
            // The search going on gives way to the one this prompt types.
            // Its query is kept, not taken: a prompt closed with nothing in
            // it leaves `F3` the search there was before.
            self.end_search();
            let previous = self.last.as_ref().map(|query| query.needle.clone());
            // A selection spanning lines means something else entirely, so
            // only a fragment seeds the query; with nothing selected it is
            // the query searched for last. What is seeded stays selected, so
            // typing replaces it instead of appending to it.
            let seed = self
                .selected_text()
                .filter(|text| !text.contains('\n'))
                .or(previous);
            if let Some(seed) = seed {
                prompt.insert(&seed);
                prompt.select_all();
            }

            // Opening moves no cursor. The hits of the seed light up, and
            // going to one is left to `Enter`.
            let mut search = Session::default();
            search.retarget(Query::literal(prompt.contents(0)), self.buffer().text());
            self.search = Some(search);
        }
        // A file's own name is the likeliest start of the new one.
        if let (Purpose::SaveAs(_), Some(path)) = (&purpose, self.buffer().path()) {
            prompt.insert(&path.display().to_string());
        }

        self.panel = Some(Panel::Prompt(prompt));
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.panel.as_ref()?.as_prompt()
    }

    /// The candidates are read on a worker and arrive as reports, however
    /// many there are and wherever they come from.
    fn open_picker(&mut self, pick: Pick) {
        self.close_panel();
        let job = match pick {
            Pick::Language => self.jobs.spawn(|worker| {
                let names = language::LANGUAGES
                    .iter()
                    .chain([&language::PLAIN])
                    .map(|language| Candidate::plain(language.name));
                picker::stream(names, &worker);
            }),
            // Named after itself, with the key it is bound to beside it.
            Pick::Command => self.jobs.spawn(|worker| {
                let entries = command::PALETTE.iter().map(|(name, command)| Candidate {
                    label: (*name).to_owned(),
                    detail: keymap::key_for(command).map_or(String::new(), |key| key.to_string()),
                });
                picker::stream(entries, &worker);
            }),
        };
        self.panel = Some(Panel::Picker(Picker::new(pick, job)));
    }

    /// The panel closes before the choice is carried out, since the choice
    /// may open a panel of its own.
    fn accept_pick(&mut self) {
        let Some(Panel::Picker(picker)) = &self.panel else {
            return;
        };
        let Some(chosen) = picker.selection() else {
            return;
        };

        match picker.pick {
            Pick::Language => match language::by_name(&chosen.label) {
                Some(language) => self.set_language(language),
                None => return,
            },
            Pick::Command => {
                let found = command::PALETTE
                    .iter()
                    .find(|(name, _)| *name == chosen.label)
                    .map(|(_, command)| command.clone());
                let Some(command) = found else {
                    return;
                };
                self.close_panel();
                return self.execute(command);
            }
        }
        self.close_panel();
    }

    pub fn panel_height(&self) -> usize {
        self.panel.as_ref().map_or(0, Panel::height)
    }

    /// The situational half of the status bar belongs to whatever is going
    /// on: an open picker, or a search.
    pub fn panel_status(&self) -> String {
        if let Some(Panel::Picker(picker)) = &self.panel {
            return format!("{}/{}", picker.matched(), picker.total());
        }
        let Some(search) = self.search.as_ref().filter(|_| self.has_search()) else {
            return String::new();
        };
        let count = search.count(self.view().selections.primary().min());

        match (self.panel.is_some(), self.searching()) {
            // The query is in the field right above; the count is the news.
            (true, true) => count,
            (true, false) => String::new(),
            (false, _) => format!("{count} {}", quoted(&search.query().needle)),
        }
    }

    /// Whether the open panel is a prompt whose query drives the search. The
    /// search itself outlives it, so its being there says nothing of this.
    fn searching(&self) -> bool {
        self.prompt()
            .is_some_and(|prompt| prompt.purpose.searches())
    }

    fn selected_text(&self) -> Option<String> {
        let primary = self.view().selections.primary();
        match primary.is_empty() {
            true => None,
            false => Some(self.buffer().text().substring(primary.range())),
        }
    }

    /// Closes the panel where the search left off. There is no separate
    /// "accept", so leaving is how you keep what you found.
    fn close_panel(&mut self) {
        if let Some(Panel::Picker(picker)) = &self.panel {
            self.jobs.cancel(picker.job);
        }
        self.panel = None;
        // A search prompt closed with nothing in it started no search.
        if !self.has_search() {
            self.search = None;
        }
    }

    /// `Enter` answers what the strip asked and closes it.
    fn submit_prompt(&mut self, backwards: bool) {
        let Some(prompt) = self.prompt() else {
            return;
        };
        let purpose = prompt.purpose.clone();
        let first = prompt.contents(0);

        match purpose {
            Purpose::Search => {
                self.start_search(
                    first,
                    match backwards {
                        true => Direction::Backward,
                        false => Direction::Forward,
                    },
                );
            }
            Purpose::GotoLine => match parse_line_number(&first) {
                Some(line) => self.goto_line(line),
                None => self.message = Some("Not a line number".to_owned()),
            },
            Purpose::Replace => {
                let replacement = prompt.contents(1);
                // With no query there is nothing to replace, and the hits
                // still in hand are those of another search.
                if !first.is_empty() {
                    self.start_search(first, Direction::Forward);
                    self.replace_all(&replacement);
                }
            }
            // Empty means nothing to do; the panel closes all the same.
            Purpose::Open | Purpose::SaveAs(_) if first.trim().is_empty() => {}
            Purpose::Open => {
                self.close_panel();
                return self.open(PathBuf::from(first.trim()));
            }
            Purpose::SaveAs(after) => {
                self.close_panel();
                return self.save_as(PathBuf::from(first.trim()), after);
            }
        }

        self.close_panel();
    }

    fn goto_line(&mut self, line: usize) {
        let (view, buffer) = self.workspace.focus_mut();
        let text = buffer.text();
        let line = line.min(text.line_count().saturating_sub(1));
        view.selections = Selection::cursor(text.line_start(line)).into();
        view.scroll_to_cursor(buffer);
    }

    fn set_language(&mut self, language: &'static Language) {
        self.buffer_mut().set_language(language);
    }

    /// Left out by default, so that tests never touch the real clipboard
    /// unless they hand in one of their own.
    pub fn with_desktop(mut self, desktop: impl Desktop + 'static) -> Self {
        self.desktop = Some(Box::new(desktop));
        self
    }

    fn move_selections(&mut self, motion: Motion, extend: bool) {
        let (view, buffer) = self.workspace.focus_mut();
        view.move_selections(buffer, motion, extend);
    }

    /// What kind of edit this is comes from the caller: the same text is
    /// typing at the keyboard and a replacement from a prompt, and only the
    /// first of those belongs in a typing group.
    fn insert(&mut self, inserted: &str, kind: EditKind) {
        let transaction = Transaction::edit(
            &self.view().selections,
            CursorAfter::EndOfChange,
            |selection| (selection.range(), inserted.to_owned()),
        );
        self.apply(transaction, kind);
    }

    fn copy(&mut self) {
        let (view, buffer) = self.workspace.focus();
        self.clipboard.copy_from(buffer.text(), &view.selections);

        let text = self.clipboard.text();
        if let Some(desktop) = &mut self.desktop {
            desktop.write(&text);
        }
    }

    fn cut(&mut self) {
        self.copy();
        let (view, buffer) = self.workspace.focus();
        if let Some(transaction) = self.clipboard.cut_from(buffer.text(), &view.selections) {
            self.apply(transaction, EditKind::Other);
        }
    }

    fn paste(&mut self) {
        self.adopt_foreign_clipboard();
        if self.clipboard.is_empty() {
            return;
        }
        let (view, buffer) = self.workspace.focus();
        let transaction = self.clipboard.paste_into(buffer.text(), &view.selections);
        self.apply(transaction, EditKind::Other);
    }

    /// Someone else copied since we did, so our pieces are stale: what the
    /// desktop holds is one piece of text, and it is not made of lines.
    fn adopt_foreign_clipboard(&mut self) {
        let Some(text) = self.desktop.as_mut().and_then(|desktop| desktop.read()) else {
            return;
        };
        if text != self.clipboard.text() {
            self.clipboard.set(vec![text], false);
        }
    }

    fn insert_newline(&mut self) {
        let (view, buffer) = self.workspace.focus();
        let transaction = lines::newline(buffer.text(), &view.selections);
        self.apply(transaction, EditKind::Other);
    }

    /// `Tab` indents whole lines when a selection crosses one, and otherwise
    /// moves every cursor to the next tab stop from where it stands.
    fn indent(&mut self) {
        let (view, buffer) = self.workspace.focus();
        let selecting = view
            .selections
            .iter()
            .any(|selection| !selection.is_empty());
        let transaction = match selecting {
            true => lines::indent(buffer.text(), &view.selections, buffer.indent),
            false => lines::tab_stops(
                buffer.text(),
                &view.selections,
                buffer.indent,
                buffer.tab_width(),
            ),
        };
        self.apply(transaction, EditKind::Other);
    }

    fn outdent(&mut self) {
        let (view, buffer) = self.workspace.focus();
        if let Some(transaction) = lines::outdent(buffer.text(), &view.selections, buffer.indent) {
            self.apply(transaction, EditKind::Other);
        }
    }

    fn toggle_comment(&mut self) {
        let (view, buffer) = self.workspace.focus();
        let Some(token) = buffer.language.line_comment else {
            self.message = Some("No line comment for this language".to_owned());
            return;
        };
        if let Some(transaction) =
            lines::toggle_comment(buffer.text(), &view.selections, token, buffer.tab_width())
        {
            self.apply(transaction, EditKind::Other);
        }
    }

    pub fn cursor_column(&self) -> usize {
        let (view, buffer) = self.workspace.focus();
        column_at(
            buffer.text(),
            view.selections.primary().head,
            buffer.tab_width(),
        )
    }

    /// A cursor with nothing selected eats the neighbouring text instead.
    fn delete(&mut self, motion: Motion) {
        let text = self.buffer().text();
        let indent = self.buffer().indent;
        // Cursors are disjoint, but what each one reaches for is not: two
        // cursors in one word both want the whole word. What an earlier
        // cursor takes is not taken again.
        let mut taken = 0;
        let changes = self.view().selections.iter().map(|selection| {
            let head = selection.head;
            let range = if !selection.is_empty() {
                selection.range()
            } else if let Some(range) = lines::deletion_to_tab_stop(text, indent, motion, head) {
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
            let range = range.start.max(taken)..range.end.max(taken);
            taken = range.end;
            (range, String::new())
        });
        let transaction =
            Transaction::over(&self.view().selections, CursorAfter::EndOfChange, changes);

        let kind = match motion {
            Motion::Grapheme(direction) => EditKind::Delete(direction),
            _ => EditKind::Other,
        };
        if !transaction.is_empty() {
            self.apply(transaction, kind);
        }
    }

    fn remove_secondary_cursors(&mut self) {
        let (view, buffer) = self.workspace.focus_mut();
        view.selections.collapse_to_primary();
        view.scroll_to_cursor(buffer);
    }

    fn cancel_selection(&mut self) {
        self.view_mut().selections.collapse_to_cursors();
    }

    /// Takes the lines a selection touches, break included. Pressing again
    /// swallows the next line, because the selection now ends on its start.
    fn select_line(&mut self) {
        let (view, buffer) = self.workspace.focus_mut();
        let text = buffer.text();
        view.selections = view.selections.mapped(|selection| {
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
        view.scroll_to_cursor(buffer);
    }

    /// The view stays where it is: jumping to the end of the file is not what
    /// anyone wants from selecting everything.
    fn select_all(&mut self) {
        let (view, buffer) = self.workspace.focus_mut();
        view.selections = Selection {
            anchor: 0,
            head: buffer.text().len(),
            goal_column: None,
        }
        .into();
    }

    /// Moves the page without moving the cursors, which may leave them off
    /// screen until something moves them again.
    fn scroll(&mut self, delta: isize) {
        let (view, buffer) = self.workspace.focus_mut();
        view.scroll_lines(delta, buffer);
    }

    fn add_cursor(&mut self, direction: Direction) {
        let (view, buffer) = self.workspace.focus_mut();
        view.add_cursor(buffer, direction);
    }

    fn undo(&mut self) {
        let (view, buffer) = self.workspace.focus_edit();
        if let Some(selections) = buffer.undo() {
            view.selections = selections;
            view.mark_current(buffer);
            view.scroll_to_cursor(buffer);
            self.refresh_search();
        }
    }

    fn redo(&mut self) {
        let (view, buffer) = self.workspace.focus_edit();
        if let Some(selections) = buffer.redo() {
            view.selections = selections;
            view.mark_current(buffer);
            view.scroll_to_cursor(buffer);
            self.refresh_search();
        }
    }

    /// The buffer changes here and nowhere else in the editor; the view
    /// gets the cursors the transaction placed, which are already current.
    fn apply(&mut self, transaction: Transaction, kind: EditKind) {
        let (view, buffer) = self.workspace.focus_edit();
        view.selections = buffer.apply(transaction, kind);
        view.mark_current(buffer);
        view.scroll_to_cursor(buffer);
        self.refresh_search();
    }

    /// The hits are offsets into the text as it was when they were found.
    /// Called from the three places the text changes: `apply`, `undo`, `redo`.
    fn refresh_search(&mut self) {
        if let Some(search) = &mut self.search {
            search.refresh(self.workspace.focus().1.text());
        }
    }
}

/// The display column an offset sits in on its line.
fn column_at(text: &Text, offset: usize, tab_width: usize) -> usize {
    let line = text.line_of(offset);
    layout::column_of_byte(
        &text.line_str(line),
        offset - text.line_start(line),
        tab_width,
    )
}

/// Columns of a query the status bar has room for; the count beside it and
/// the document on the right matter more than the rest of a long one.
const QUERY_COLUMNS: usize = 24;

/// A query for the status bar: in quotes, so that a space at either end
/// shows, and cut at the end when it is long, since a query is told by how
/// it starts.
fn quoted(needle: &str) -> String {
    // A tab counts as one column, as it does everywhere on the status bar.
    if layout::line_width(needle, 1) <= QUERY_COLUMNS {
        return format!("\"{needle}\"");
    }
    let end = layout::byte_at_column(needle, QUERY_COLUMNS - 1, 1);
    format!("\"{}…\"", &needle[..end])
}

/// Line numbers are what people count from one.
fn parse_line_number(input: &str) -> Option<usize> {
    let line = input.trim().parse::<usize>().ok()?;
    Some(line.saturating_sub(1))
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
mod tests;
