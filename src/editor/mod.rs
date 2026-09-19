//! The editor as a whole: what commands mean, which buffers and views there
//! are, what panel is open, and the one boundary everything crosses.

pub mod clipboard;
pub mod command;
pub mod job;
pub mod key;
pub mod keymap;
pub mod lines;
pub mod panel;
pub mod screen;
pub mod style;
pub mod view;
pub mod workspace;

use std::ops::Range;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use crate::buffer::language::{self, Language};
use crate::buffer::{Backing, Buffer, EditKind, Stamp};
use crate::text::layout;
use crate::text::movement::{self, Class, Direction, Motion};
use crate::text::search::{Query, Session};
use crate::text::selection::{Selection, SelectionSet};
use crate::text::transaction::{CursorAfter, Transaction};
use clipboard::{Clipboard, Desktop};
use command::{Command, Edit, File, PanelCommand, Select};
use job::{Jobs, Message, Report};
use panel::picker::{self, Pick, Picker};
use panel::prompt::{Field, Prompt, Purpose};
use panel::{Confirm, Panel, Question};
use screen::Screen;
use view::View;
use workspace::{BufferId, Workspace};

/// How often the file behind a buffer is looked at. Seconds are plenty: the
/// check is a `stat`, and the save it guards asks again anyway.
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

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
    /// The search a prompt is typing. Lives only while the prompt does, so
    /// nothing has to remember to invalidate its hits.
    search: Option<Session>,
    /// What `F3` repeats once the prompt has closed.
    last: Option<Query>,
    quit: bool,
    occurrences: Option<Occurrences>,
    /// The clipboard outside the process, when there is one to talk to.
    desktop: Option<Box<dyn Desktop>>,
    jobs: Jobs,
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
            quit: false,
            occurrences: None,
            desktop: None,
            jobs,
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

    /// Keeps an eye on the file behind a buffer, so that another writer is
    /// noticed before `Ctrl+S` runs into them (F1). The worker only says
    /// that the stamp moved; whether it matters is checked on arrival.
    fn watch(&mut self, id: BufferId) {
        let Backing::File { path, on_disk } = &self.workspace.buffer(id).backing else {
            return;
        };
        let (path, mut seen) = (path.clone(), *on_disk);
        self.jobs.spawn_forever(move |worker| {
            while !worker.cancelled() {
                std::thread::sleep(WATCH_INTERVAL);
                let now = Stamp::of(&path);
                if now != seen {
                    seen = now;
                    if !worker.report(Report::FileChanged(id)) {
                        return;
                    }
                }
            }
        });
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

    /// The hits of the search being typed, for the renderer to underline.
    pub fn matches(&self) -> &[Range<usize>] {
        self.search.as_ref().map_or(&[], Session::hits)
    }

    pub fn screen(&self) -> &Screen {
        &self.screen
    }

    pub fn should_quit(&self) -> bool {
        self.quit
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
        self.screen = Screen::divide(term_width, term_height, view::gutter_width(line_count), panel);
        view.resize(self.screen.text, line_count);
    }

    /// The only way in (ADR 16). Routes to the method that does the work;
    /// what a command means is decided here and nowhere else.
    pub fn execute(&mut self, command: Command) {
        // A message answers the last command; the next one supersedes it.
        self.message = None;

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
                |editor| editor.insert(&text),
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
            // While a panel is up the hits are in hand and get walked; once
            // it is gone the last query is run against the text again.
            Command::FindNext(direction) => match self.panel.is_some() {
                true => self.go_to_match(direction),
                false => self.search_next(direction),
            },
            Command::Select(Select::AllOccurrences) => match self.panel.is_some() {
                true => self.select_all_matches(),
                false => self.select_all_occurrences(),
            },

            Command::Scroll(rows) => self.scroll(rows),
            Command::Quit => self.request_quit(),

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

            Command::File(File::Save) => {
                self.save(false);
            }
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

    /// An edited query drives whatever the panel is for; a moved caret does
    /// not, or a search would drag the cursor back to the first hit.
    fn field_edited(&mut self) {
        match &mut self.panel {
            Some(Panel::Picker(picker)) => picker.refilter(),
            Some(Panel::Prompt(_)) => self.prompt_changed(),
            _ => {}
        }
    }

    fn set_selections(&mut self, selections: SelectionSet) {
        let (view, buffer) = self.workspace.focus_mut();
        view.selections = selections;
        view.scroll_to_cursor(buffer);
    }

    /// Screen cells into the view's own coordinates: past its gutter, down
    /// from its top. A click outside the text is nobody's.
    fn cell_in_view(&self, row: usize, column: usize) -> Option<(usize, usize)> {
        let view = self.view();
        let row = row.checked_sub(view.area.y)?;
        (row < view.height()).then(|| {
            (row, column.saturating_sub(view.area.x + view.gutter))
        })
    }

    fn click(&mut self, row: usize, column: usize, add: bool) {
        let Some((row, column)) = self.cell_in_view(row, column) else {
            return;
        };
        let (view, buffer) = self.workspace.focus_mut();
        let offset = view.offset_at_screen(buffer, row, column);
        match add {
            true => view.selections.push(Selection::cursor(offset)),
            false => view.selections = SelectionSet::single(offset),
        }
    }

    fn drag(&mut self, row: usize, column: usize) {
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
        let mut prompt = Prompt::new(purpose);

        if purpose.searches() {
            self.search = Some(Session::new(self.view().selections.clone()));
            // A selection spanning lines means something else entirely, so
            // only a fragment seeds the query. What is seeded stays selected,
            // so typing replaces it instead of appending to it.
            if let Some(selected) = self.selected_text().filter(|text| !text.contains('\n')) {
                prompt.insert(&selected);
                prompt.select_all();
            }
        }

        self.panel = Some(Panel::Prompt(prompt));
        self.prompt_changed();
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
                    .map(|language| language.name.to_owned());
                picker::stream(names, &worker);
            }),
        };
        self.panel = Some(Panel::Picker(Picker::new(pick, job)));
    }

    fn accept_pick(&mut self) {
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
        match (&self.panel, &self.search) {
            (Some(Panel::Picker(picker)), _) => format!("{}/{}", picker.len(), picker.total()),
            (Some(_), Some(search)) => search.status(self.view().selections.primary().min()),
            _ => String::new(),
        }
    }

    fn selected_text(&self) -> Option<String> {
        let primary = self.view().selections.primary();
        match primary.is_empty() {
            true => None,
            false => Some(self.buffer().text().slice(primary.range()).to_string()),
        }
    }

    /// Closes the panel where the search left off. There is no separate
    /// "accept", so leaving is how you keep what you found.
    fn close_panel(&mut self) {
        if let Some(Panel::Picker(picker)) = &self.panel {
            self.jobs.cancel(picker.job);
        }
        self.panel = None;
        self.search = None;
    }

    /// `Enter` walks the hits while searching; for anything else it answers
    /// the question and closes the strip.
    fn submit_prompt(&mut self, backwards: bool) {
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
                self.replace_all(&replacement);
            }
        }

        self.close_panel();
    }

    /// Incremental search: every keystroke retargets the session and lands
    /// on the first hit at or after where the cursor stood when the prompt
    /// opened, so the match never drifts forward as the query grows.
    fn prompt_changed(&mut self) {
        let Some(needle) = self.prompt().map(|prompt| prompt.contents(0)) else {
            return;
        };
        let Some(search) = &mut self.search else {
            return;
        };

        let query = Query::literal(needle);
        search.retarget(query.clone(), self.workspace.focus().1.text());
        self.last = (!query.is_empty()).then_some(query);

        let origin = search.origin.clone();
        match search.landing(origin.primary().min()) {
            Some(hit) => self.select_hit(hit),
            None => self.view_mut().selections = origin,
        }
    }

    /// Turns the hits into cursors and hands the keyboard back to the text,
    /// which is the only reason anyone wants a cursor on every hit.
    fn select_all_matches(&mut self) {
        let Some(search) = &self.search else {
            return;
        };
        if search.hits().is_empty() {
            return;
        }

        let selections = search.selections(self.view().selections.primary().min());
        let (view, buffer) = self.workspace.focus_mut();
        view.selections = selections;
        view.scroll_to_cursor(buffer);
        self.close_panel();
    }

    /// Walks the hits the prompt has in hand.
    fn go_to_match(&mut self, direction: Direction) {
        let from = self.from(direction);
        let hit = self.search.as_ref().and_then(|search| search.next_hit(from, direction));
        match hit {
            Some(hit) => self.select_hit(hit),
            None => self.message = Some("Not found".to_owned()),
        }
    }

    /// Repeats the last search against the text, with the prompt gone.
    fn search_next(&mut self, direction: Direction) {
        let Some(query) = &self.last else {
            return;
        };
        let from = self.from(direction);
        match query.find(&self.buffer().text(), from, direction) {
            Some(hit) => self.select_hit(hit),
            None => self.message = Some("Not found".to_owned()),
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
    /// rest — so a replacement is one undo step.
    fn replace_all(&mut self, replacement: &str) {
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
        self.insert(replacement);
        self.message = Some(format!("Replaced {hits}"));
    }

    fn goto_line(&mut self, line: usize) {
        let (view, buffer) = self.workspace.focus_mut();
        let text = &buffer.text();
        let line = line.min(text.line_count().saturating_sub(1));
        view.selections = Selection::cursor(text.line_start(line)).into();
        view.scroll_to_cursor(buffer);
    }

    /// Saves unless another writer got there first, in which case it asks
    /// rather than overwriting their work blindly (ADR 20). Returns whether
    /// the text reached the disk; a question pending counts as no.
    fn save(&mut self, then_quit: bool) -> bool {
        if self.buffer().changed_on_disk() {
            self.panel = Some(Panel::Confirm(Confirm::overwrite(then_quit)));
            return false;
        }
        self.write()
    }

    fn write(&mut self) -> bool {
        match self.buffer_mut().save() {
            Ok(()) => true,
            Err(error) => {
                self.message = Some(format!("{error:#}"));
                false
            }
        }
    }

    /// Nothing is ever discarded without being asked about first.
    fn request_quit(&mut self) {
        match self.buffer().is_modified() {
            false => self.quit = true,
            true => self.panel = Some(Panel::Confirm(Confirm::quit())),
        }
    }

    /// A failed save cancels the quit and says why, rather than losing the
    /// file that could not be written. Saving may raise a question of its
    /// own, which then replaces this one.
    fn answer(&mut self, answer: char) {
        let Some(Panel::Confirm(confirm)) = &self.panel else {
            return;
        };

        // The letters are commands, not text, so they are read off the
        // physical key rather than the current keyboard layout.
        match (confirm.about, key::physical_key(answer)) {
            (Question::Quit, 's') => {
                self.close_panel();
                self.quit = self.save(true);
            }
            (Question::Quit, 'q') => {
                self.close_panel();
                self.quit = true;
            }
            (Question::Overwrite { then_quit }, 'o') => {
                self.close_panel();
                self.quit = self.write() && then_quit;
            }
            _ => {}
        }
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

    fn insert(&mut self, inserted: &str) {
        let transaction = Transaction::edit(
            &self.view().selections,
            CursorAfter::EndOfChange,
            |selection| (selection.range(), inserted.to_owned()),
        );
        self.apply(transaction, insert_kind(inserted));
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
    /// moves to the next tab stop.
    fn indent(&mut self) {
        let (view, buffer) = self.workspace.focus();
        let selecting = view.selections.iter().any(|selection| !selection.is_empty());
        match selecting {
            true => {
                let transaction = lines::indent(buffer.text(), &view.selections, buffer.indent);
                self.apply(transaction, EditKind::Other);
            }
            false => {
                let inserted = lines::to_tab_stop(buffer.indent, self.cursor_column());
                self.insert(&inserted);
            }
        }
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
        if let Some(transaction) = lines::toggle_comment(buffer.text(), &view.selections, token) {
            self.apply(transaction, EditKind::Other);
        }
    }

    pub fn cursor_column(&self) -> usize {
        let (view, buffer) = self.workspace.focus();
        let text = &buffer.text();
        let head = view.selections.primary().head;
        let line = text.line_of(head);
        layout::column_of_byte(
            &text.line_str(line),
            head - text.line_start(line),
            buffer.tab_width(),
        )
    }

    /// A cursor with nothing selected eats the neighbouring text instead.
    fn delete(&mut self, motion: Motion) {
        let text = &self.buffer().text();
        let indent = self.buffer().indent;
        let transaction = Transaction::edit(
            &self.view().selections,
            CursorAfter::EndOfChange,
            |selection| {
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
        let text = &buffer.text();
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

    /// First press selects the word under each cursor; every press after that
    /// adds the next occurrence of it, wrapping around the document.
    fn select_next_occurrence(&mut self) {
        let Some(query) = self.take_query() else {
            return;
        };

        let (view, buffer) = self.workspace.focus_mut();
        let from = view.selections.primary().max();
        let Some(hit) = query.find(&buffer.text(), from, Direction::Forward) else {
            return;
        };
        if view.selections.iter().any(|selection| selection.range() == hit) {
            return;
        }

        view.selections.push(Query::selection(hit));
        view.scroll_to_cursor(buffer);
        self.remember_occurrences(query.whole_word);
    }

    fn select_all_occurrences(&mut self) {
        let Some(query) = self.take_query() else {
            return;
        };

        let (view, buffer) = self.workspace.focus_mut();
        let selections = query.all(&buffer.text()).map(Query::selection).collect();
        view.selections = SelectionSet::from_selections(selections, 0);
        view.scroll_to_cursor(buffer);
        self.remember_occurrences(query.whole_word);
    }

    /// Bare cursors first expand to the word they touch; only once something
    /// is selected is there anything to search for.
    fn take_query(&mut self) -> Option<Query> {
        let (view, buffer) = self.workspace.focus_mut();
        let text = &buffer.text();
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
            needle: self.buffer().text().slice(range).to_string(),
            whole_word,
        })
    }

    fn remember_occurrences(&mut self, whole_word: bool) {
        self.occurrences = Some(Occurrences {
            selections: self.view().selections.clone(),
            whole_word,
        });
    }

    fn undo(&mut self) {
        let (view, buffer) = self.workspace.focus_edit();
        if let Some(selections) = buffer.undo() {
            view.selections = selections;
            view.mark_current(buffer);
            view.scroll_to_cursor(buffer);
        }
    }

    fn redo(&mut self) {
        let (view, buffer) = self.workspace.focus_edit();
        if let Some(selections) = buffer.redo() {
            view.selections = selections;
            view.mark_current(buffer);
            view.scroll_to_cursor(buffer);
        }
    }

    /// The buffer changes here and nowhere else in the editor; the view
    /// gets the cursors the transaction placed, which are already current.
    fn apply(&mut self, transaction: Transaction, kind: EditKind) {
        let (view, buffer) = self.workspace.focus_edit();
        view.selections = buffer.apply(transaction, kind);
        view.mark_current(buffer);
        view.scroll_to_cursor(buffer);
    }
}

/// Line numbers are what people count from one.
fn parse_line_number(input: &str) -> Option<usize> {
    let line = input.trim().parse::<usize>().ok()?;
    Some(line.saturating_sub(1))
}

/// The cursors a run of `Ctrl+D` left behind, so the next press can tell
/// whether it is the same run.
struct Occurrences {
    selections: SelectionSet,
    whole_word: bool,
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
    use crate::buffer::language::Indent;
    use crate::text::movement::Direction;
    use crate::text::selection::{Selection, SelectionSet};
    use crate::text::Text;

    fn editor(content: &str, offset: usize) -> Editor {
        let mut editor = Editor::new(Buffer::from_text(Text::from_str(content)));
        editor.resize(80, 25);
        select(&mut editor, SelectionSet::single(offset));
        editor
    }

    fn content(editor: &Editor) -> String {
        editor.buffer().text().to_string()
    }

    /// Starting a test from a known place is a command like any other.
    fn select(editor: &mut Editor, selections: impl Into<SelectionSet>) {
        editor.execute(Command::Select(Select::Set(selections.into())));
    }

    fn select_range(editor: &mut Editor, anchor: usize, head: usize) {
        select(
            editor,
            Selection {
                anchor,
                head,
                goal_column: None,
            },
        );
    }

    fn add_cursor(editor: &mut Editor, offset: usize) {
        let mut selections = editor.view().selections.clone();
        selections.push(Selection::cursor(offset));
        select(editor, selections);
    }

    #[test]
    fn typing_lands_after_the_cursor() {
        let mut editor = editor("ac", 1);
        editor.insert("b");

        assert_eq!(content(&editor), "abc");
        assert_eq!(editor.view().selections.primary().head, 2);
    }

    #[test]
    fn typing_replaces_a_selection() {
        let mut editor = editor("hello world", 0);
        select_range(&mut editor, 0, 5);
        editor.insert("bye");

        assert_eq!(content(&editor), "bye world");
        assert!(editor.view().selections.primary().is_empty());
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
        assert!(!editor.buffer().is_modified());
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
        add_cursor(&mut editor, 2);
        add_cursor(&mut editor, 4);
        editor.insert("!");

        assert_eq!(content(&editor), "!a !a !a");
        assert_eq!(editor.view().selections.len(), 3);
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
        add_cursor(&mut editor, 3);
        editor.insert("!");
        assert_eq!(content(&editor), "a! a!");

        editor.undo();
        let heads: Vec<_> = editor
            .view()
            .selections
            .iter()
            .map(|selection| selection.head)
            .collect();
        assert_eq!(heads, [1, 3]);
    }

    fn ranges(editor: &Editor) -> Vec<std::ops::Range<usize>> {
        editor
            .view()
            .selections
            .iter()
            .map(Selection::range)
            .collect()
    }

    #[test]
    fn the_first_press_selects_the_word_under_the_cursor() {
        let mut editor = editor("foo bar foo", 1);
        editor.select_next_occurrence();

        assert_eq!(editor.view().selections.len(), 1);
        assert_eq!(editor.view().selections.primary().range(), 0..3);
    }

    #[test]
    fn a_cursor_resting_after_a_word_still_finds_it() {
        let mut editor = editor("foo bar", 3);
        editor.select_next_occurrence();

        assert_eq!(editor.view().selections.primary().range(), 0..3);
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
        select_range(&mut editor, 0, 3);
        editor.select_next_occurrence();

        assert_eq!(ranges(&editor), [0..3, 4..7]);
    }

    #[test]
    fn a_partial_selection_matches_anywhere() {
        let mut editor = editor("foo foobar", 0);
        select_range(&mut editor, 0, 2);
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

        assert_eq!(editor.view().selections.len(), 1);
    }

    #[test]
    fn a_stacked_cursor_keeps_the_goal_column_over_a_short_line() {
        let mut editor = editor("aaaa\nx\ncccc", 3);
        editor.add_cursor(Direction::Forward);
        editor.add_cursor(Direction::Forward);

        assert_eq!(ranges(&editor), [3..3, 6..6, 10..10]);
    }

    /// Goes through `execute` so that the focus routing is what puts the
    /// text into the prompt and runs the search again.
    fn type_into_prompt(editor: &mut Editor, typed: &str) {
        for character in typed.chars() {
            editor.execute(Command::Edit(Edit::Insert(character.to_string())));
        }
    }

    fn indented(content: &str, indent: Indent, offset: usize) -> Editor {
        let mut buffer = Buffer::from_text(Text::from_str(content));
        buffer.indent = indent;
        let mut editor = Editor::new(buffer);
        editor.resize(80, 25);
        select(&mut editor, SelectionSet::single(offset));
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
        select_range(&mut editor, 0, 20);
        editor.toggle_comment();

        assert_eq!(content(&editor), "  // one();\n  //     two();\n");
    }

    #[test]
    fn a_half_commented_block_becomes_fully_commented() {
        let mut editor = rust_editor("// one();\ntwo();\n", 0);
        select_range(&mut editor, 0, 16);
        editor.toggle_comment();

        assert_eq!(content(&editor), "// // one();\n// two();\n");
    }

    #[test]
    fn blank_lines_are_left_alone() {
        let mut editor = rust_editor("one();\n\ntwo();\n", 0);
        select_range(&mut editor, 0, 14);
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
        assert_eq!(editor.view().selections.primary().head, 13);
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
        add_cursor(&mut editor, 13);
        editor.insert_newline();

        assert_eq!(content(&editor), "  one\n  \n    two\n    \n");
    }

    #[test]
    fn a_new_line_replaces_what_was_selected() {
        let mut editor = editor("    keep drop\n", 0);
        select_range(&mut editor, 9, 13);
        editor.insert_newline();

        assert_eq!(content(&editor), "    keep \n    \n");
    }

    #[test]
    fn the_cursor_column_is_a_display_column() {
        // Three Cyrillic letters are six bytes but three columns.
        assert_eq!(editor("привет", 6).cursor_column(), 3);
        // A tab is one byte and a jump to the next stop.
        assert_eq!(editor("\tx", 2).cursor_column(), 5);
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
        select_range(&mut editor, 1, 9);
        editor.indent();

        assert_eq!(content(&editor), "  one\n  two\n  three\n");
        // Snapped to the line start, so the new indentation is inside the
        // selection rather than hanging off its left edge.
        assert_eq!(editor.view().selections.primary().range(), 0..15);
    }

    #[test]
    fn a_line_under_two_cursors_is_indented_once() {
        let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
        select_range(&mut editor, 0, 5);
        // A second cursor on a line the selection already covers.
        add_cursor(&mut editor, 6);
        editor.indent();

        assert_eq!(content(&editor), "  one\n  two\n");
    }

    #[test]
    fn a_selection_ending_on_a_line_start_leaves_that_line_alone() {
        let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
        // What Ctrl+L produces: the line and the break after it.
        select_range(&mut editor, 0, 4);
        editor.indent();

        assert_eq!(content(&editor), "  one\ntwo\n");
    }

    #[test]
    fn any_selection_at_all_means_indent_the_line() {
        let mut editor = indented("one two\n", Indent::Spaces(2), 0);
        select_range(&mut editor, 4, 7);
        editor.indent();

        // The selected word is indented with its line, not replaced by a tab.
        assert_eq!(content(&editor), "  one two\n");
    }

    #[test]
    fn bare_cursors_get_a_tab_stop_each() {
        let mut editor = indented("one\ntwo\n", Indent::Spaces(2), 0);
        add_cursor(&mut editor, 4);
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
        select_range(&mut editor, 0, 7);
        editor.indent();
        assert_eq!(content(&editor), "  one\n  two\n");

        editor.undo();
        assert_eq!(content(&editor), "one\ntwo\n");
    }

    #[test]
    fn undoing_back_to_what_was_saved_is_not_a_change() {
        let path = std::env::temp_dir().join(format!("ned-{}-modified", std::process::id()));
        std::fs::write(&path, "start").unwrap();

        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        select(&mut editor, SelectionSet::single(5));
        assert!(!editor.buffer().is_modified());

        editor.insert("!");
        assert!(editor.buffer().is_modified());

        editor.save(false);
        assert!(!editor.buffer().is_modified());

        // Back to what the file holds, so there is nothing to warn about.
        editor.undo();
        assert!(editor.buffer().is_modified());
        editor.redo();
        assert!(!editor.buffer().is_modified());

        // And an edit undone past the save is a change again.
        editor.insert("?");
        assert!(editor.buffer().is_modified());
        editor.undo();
        assert!(!editor.buffer().is_modified());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn typing_after_a_save_starts_its_own_revision() {
        let path = std::env::temp_dir().join(format!("ned-{}-revision", std::process::id()));
        std::fs::write(&path, "").unwrap();

        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        type_text(&mut editor, "wor");
        editor.save(false);

        // The same word carries on, but it must not join the group that was
        // written to disk.
        type_text(&mut editor, "d");
        assert!(editor.buffer().is_modified());

        let _ = std::fs::remove_file(&path);
    }

    /// ADR 20, the half that waits for nothing: a save never overwrites
    /// another writer's work without asking.
    #[test]
    fn a_file_changed_elsewhere_is_not_overwritten_unasked() {
        let path = std::env::temp_dir().join(format!("ned-{}-elsewhere", std::process::id()));
        std::fs::write(&path, "mine\n").unwrap();

        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        editor.insert("!");
        std::fs::write(&path, "theirs, and longer\n").unwrap();

        editor.execute(Command::File(File::Save));
        assert!(matches!(
            editor.panel,
            Some(Panel::Confirm(Confirm {
                about: Question::Overwrite { then_quit: false },
                ..
            }))
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs, and longer\n");
        assert!(editor.buffer().is_modified());

        editor.execute(Command::Panel(PanelCommand::Answer('o')));
        assert!(editor.panel.is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "!mine\n");
        assert!(!editor.buffer().is_modified());
        assert!(!editor.quit);

        let _ = std::fs::remove_file(&path);
    }

    /// The watcher only says the stamp moved; whether that is another writer
    /// or our own save is decided on arrival.
    #[test]
    fn a_report_of_a_changed_file_is_checked_before_it_is_believed() {
        let path = std::env::temp_dir().join(format!("ned-{}-watched", std::process::id()));
        std::fs::write(&path, "mine\n").unwrap();
        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        let id = editor.workspace.focused_buffer();

        // Our own save moves the stamp, and the report that follows it says
        // nothing.
        editor.insert("!");
        editor.execute(Command::File(File::Save));
        editor.receive(Report::FileChanged(id));
        assert_eq!(editor.message(), None);

        std::fs::write(&path, "theirs, and longer\n").unwrap();
        editor.receive(Report::FileChanged(id));
        assert_eq!(editor.message(), Some("File changed on disk"));

        let _ = std::fs::remove_file(&path);
    }

    /// The picker takes candidates from its own worker only: a batch from a
    /// picker that has since been closed is not the new one's.
    #[test]
    fn a_picker_ignores_candidates_meant_for_another() {
        fn picker(editor: &Editor) -> &Picker {
            match &editor.panel {
                Some(Panel::Picker(picker)) => picker,
                _ => panic!("a picker is open"),
            }
        }

        let mut editor = editor("", 0);
        editor.execute(Command::Panel(PanelCommand::Picker(Pick::Language)));
        let stale = picker(&editor).job;

        // Opened again: the first worker is cancelled, but a batch of its
        // may still be in the queue.
        editor.execute(Command::Panel(PanelCommand::Picker(Pick::Language)));
        editor.finish_jobs();
        assert_ne!(picker(&editor).job, stale);
        let total = picker(&editor).total();
        assert!(total > 0);

        editor.receive(Report::Candidates {
            job: stale,
            batch: vec!["Stray".to_owned()],
        });
        assert_eq!(picker(&editor).total(), total);
    }

    /// "Save and quit" that runs into another writer asks, and the answer
    /// still quits: the intention to leave is not lost with the question.
    #[test]
    fn save_and_quit_still_quits_after_agreeing_to_overwrite() {
        let path = std::env::temp_dir().join(format!("ned-{}-overwrite-quit", std::process::id()));
        std::fs::write(&path, "mine\n").unwrap();

        let mut editor = Editor::new(Buffer::open(&path).unwrap());
        editor.insert("!");
        std::fs::write(&path, "theirs, and longer\n").unwrap();

        editor.execute(Command::Quit);
        editor.execute(Command::Panel(PanelCommand::Answer('s')));
        assert!(!editor.quit);
        assert!(matches!(
            editor.panel,
            Some(Panel::Confirm(Confirm {
                about: Question::Overwrite { then_quit: true },
                ..
            }))
        ));

        // Declining leaves everything as it was: not written, not quit.
        editor.execute(Command::Panel(PanelCommand::Close));
        assert!(!editor.quit);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs, and longer\n");

        editor.execute(Command::Quit);
        editor.execute(Command::Panel(PanelCommand::Answer('s')));
        editor.execute(Command::Panel(PanelCommand::Answer('o')));
        assert!(editor.quit);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "!mine\n");

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

        assert_eq!(editor.view().selections.primary().head, 8);
        assert!(editor.panel.is_none());
    }

    #[test]
    fn goto_line_complains_about_nonsense() {
        let mut editor = editor("one\ntwo\n", 0);
        editor.open_prompt(Purpose::GotoLine);
        type_into_prompt(&mut editor, "later");
        editor.submit_prompt(false);

        assert_eq!(editor.view().selections.primary().head, 0);
        assert!(editor.message.is_some());
    }

    #[test]
    fn select_all_covers_the_document_and_stays_put() {
        let mut editor = editor(&"line\n".repeat(100), 0);
        editor.resize(80, 11);
        editor.move_selections(Motion::Document(Direction::Forward), false);
        let top = editor.view().top_line;

        editor.select_all();
        assert_eq!(
            editor.view().selections.primary().range(),
            0..editor.buffer().text().len()
        );
        assert_eq!(editor.view().top_line, top);
    }

    #[test]
    fn scrolling_leaves_the_cursors_alone() {
        let mut editor = editor(&"line\n".repeat(100), 0);
        editor.resize(80, 11);

        editor.scroll(5);
        assert_eq!(editor.view().top_line, 5);
        assert_eq!(editor.view().selections.primary().head, 0);

        editor.scroll(-99);
        assert_eq!(editor.view().top_line, 0);
    }

    #[test]
    fn opening_search_takes_the_selected_text() {
        let mut editor = editor("alpha beta gamma", 0);
        select_range(&mut editor, 6, 10);
        editor.open_prompt(Purpose::Search);

        let prompt = editor.prompt().unwrap();
        assert_eq!(prompt.contents(0), "beta");
        // Seeded and selected, so the next keystroke replaces it.
        assert_eq!(prompt.fields[0].selection(), 0..4);
        // The match under the cursor stays put rather than jumping ahead.
        assert_eq!(editor.view().selections.primary().range(), 6..10);
    }

    #[test]
    fn a_selection_across_lines_does_not_seed_the_query() {
        let mut editor = editor("one\ntwo", 0);
        select_range(&mut editor, 0, 7);
        editor.open_prompt(Purpose::Search);

        assert_eq!(editor.prompt().unwrap().contents(0), "");
    }

    #[test]
    fn search_follows_every_keystroke() {
        let mut editor = editor("alpha beta gamma", 0);
        editor.open_prompt(Purpose::Search);

        type_into_prompt(&mut editor, "b");
        assert_eq!(editor.view().selections.primary().range(), 6..7);

        type_into_prompt(&mut editor, "eta");
        assert_eq!(editor.view().selections.primary().range(), 6..10);
    }

    #[test]
    fn search_starts_over_from_where_it_opened() {
        let mut editor = editor("aa ab", 0);
        editor.open_prompt(Purpose::Search);

        // Typing `a` then `b` must not chase the first match forward.
        type_into_prompt(&mut editor, "ab");
        assert_eq!(editor.view().selections.primary().range(), 3..5);
    }

    #[test]
    fn escape_closes_the_search_where_it_left_off() {
        let mut editor = editor("alpha beta", 2);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "beta");
        editor.close_panel();

        assert_eq!(editor.view().selections.primary().range(), 6..10);
        assert!(editor.panel.is_none());
        assert!(editor.matches().is_empty());
    }

    #[test]
    fn enter_walks_the_matches_and_wraps() {
        let mut editor = editor("x . x . x", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "x");
        assert_eq!(editor.view().selections.primary().range(), 0..1);

        editor.submit_prompt(false);
        assert_eq!(editor.view().selections.primary().range(), 4..5);
        editor.submit_prompt(false);
        assert_eq!(editor.view().selections.primary().range(), 8..9);
        editor.submit_prompt(false);
        assert_eq!(editor.view().selections.primary().range(), 0..1);

        editor.submit_prompt(true);
        assert_eq!(editor.view().selections.primary().range(), 8..9);

        // Walking the hits keeps the panel open.
        assert!(editor.panel.is_some());
    }

    #[test]
    fn every_hit_is_collected_and_counted() {
        let mut editor = editor("x . x . x", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "x");

        assert_eq!(editor.matches(), [0..1, 4..5, 8..9]);
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
        assert_eq!(editor.view().selections.primary().range(), 4..6);

        editor.insert("key");
        assert_eq!(content(&editor), "key, key, key");
    }

    #[test]
    fn narrowing_a_query_finds_what_scanning_would() {
        let mut editor = editor("ab abc abcd", 0);
        editor.open_prompt(Purpose::Search);

        type_into_prompt(&mut editor, "a");
        assert_eq!(editor.matches(), [0..1, 3..4, 7..8]);

        type_into_prompt(&mut editor, "b");
        assert_eq!(editor.matches(), [0..2, 3..5, 7..9]);

        type_into_prompt(&mut editor, "cd");
        assert_eq!(editor.matches(), vec![7..11]);

        // Backspace cannot narrow, so this one has to read the document again.
        editor.execute(Command::Edit(Edit::Delete(Motion::Grapheme(Direction::Backward))));
        assert_eq!(editor.matches(), [3..6, 7..10]);
    }

    #[test]
    fn narrowing_does_not_cut_a_character_in_half() {
        let mut editor = editor("аб абв", 0);
        editor.open_prompt(Purpose::Search);

        type_into_prompt(&mut editor, "а");
        assert_eq!(editor.matches(), [0..2, 5..7]);

        type_into_prompt(&mut editor, "бв");
        assert_eq!(editor.matches(), vec![5..11]);
    }

    #[test]
    fn a_query_with_no_hits_counts_zero() {
        let mut editor = editor("nothing here", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "zzz");

        assert!(editor.matches().is_empty());
        assert_eq!(editor.panel_status(), "Found: none");
    }

    #[test]
    fn f3_still_works_once_the_strip_is_closed() {
        let mut editor = editor("x . x", 0);
        editor.open_prompt(Purpose::Search);
        type_into_prompt(&mut editor, "x");
        editor.close_panel();

        editor.search_next(Direction::Forward);
        assert_eq!(editor.view().selections.primary().range(), 4..5);
    }

    #[test]
    fn replace_changes_every_occurrence_in_one_step() {
        let mut editor = editor("one two one", 0);
        editor.open_prompt(Purpose::Replace);
        type_into_prompt(&mut editor, "one");
        editor.execute(Command::Panel(PanelCommand::NextField));
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

        assert_eq!(editor.view().selections.primary().range(), 6..13);
    }

    #[test]
    fn selecting_again_swallows_the_next_line() {
        let mut editor = editor("first\nsecond\nthird\n", 8);
        editor.select_line();
        editor.select_line();

        assert_eq!(editor.view().selections.primary().range(), 6..19);
    }

    #[test]
    fn a_selection_across_lines_expands_to_all_of_them() {
        let mut editor = editor("first\nsecond\nthird\n", 0);
        select_range(&mut editor, 3, 8);
        editor.select_line();

        assert_eq!(editor.view().selections.primary().range(), 0..13);
    }

    #[test]
    fn selecting_the_last_line_stops_at_the_end() {
        let mut editor = editor("first\nlast", 7);
        editor.select_line();
        assert_eq!(editor.view().selections.primary().range(), 6..10);

        editor.select_line();
        assert_eq!(editor.view().selections.primary().range(), 6..10);
    }

    #[test]
    fn selected_lines_can_be_cut_at_every_cursor() {
        let mut editor = editor("one\ntwo\nthree\n", 0);
        add_cursor(&mut editor, 8);
        editor.select_line();
        editor.cut();

        assert_eq!(content(&editor), "two\n");
    }

    #[test]
    fn copying_and_pasting_a_selection() {
        let mut editor = editor("hello world", 0);
        editor.select_next_occurrence();
        editor.copy();

        select(&mut editor, SelectionSet::single(11));
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
        assert_eq!(editor.view().selections.primary().head, 9);
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
        assert_eq!(editor.view().selections.primary().head, 15);
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
        add_cursor(&mut editor, 2);
        editor.cut();

        assert_eq!(content(&editor), "two\nthree\n");
        assert_eq!(editor.clipboard.text(), "one\n");
    }

    #[test]
    fn a_line_under_two_cursors_is_pasted_once() {
        let mut editor = editor("one\ntwo\n", 0);
        editor.copy();
        add_cursor(&mut editor, 2);
        editor.paste();

        assert_eq!(content(&editor), "one\none\ntwo\n");
    }

    #[test]
    fn cursors_on_separate_lines_still_get_a_line_each() {
        let mut editor = editor("one\ntwo\nthree\n", 0);
        add_cursor(&mut editor, 4);
        editor.cut();

        assert_eq!(content(&editor), "three\n");
        assert_eq!(editor.clipboard.text(), "one\ntwo\n");
    }

    #[test]
    fn each_cursor_pastes_its_own_piece() {
        let mut editor = editor("aa bb", 0);
        select(
            &mut editor,
            SelectionSet::from_selections(
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
            ),
        );
        editor.copy();
        editor.paste();

        assert_eq!(content(&editor), "aa bb");
        assert_eq!(editor.clipboard.pieces_for(2), ["aa", "bb"]);
    }

    #[test]
    fn one_cursor_pastes_everything_that_was_copied() {
        let mut editor = editor("aa bb", 0);
        select(
            &mut editor,
            SelectionSet::from_selections(
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
            ),
        );
        editor.copy();

        select(&mut editor, SelectionSet::single(5));
        editor.paste();
        assert_eq!(content(&editor), "aa bbaa\nbb");
    }

    #[test]
    fn pasting_is_one_undo_step() {
        let mut editor = editor("word", 0);
        editor.select_next_occurrence();
        editor.copy();
        select(&mut editor, SelectionSet::single(4));
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
        assert_eq!(editor.view().selections.primary().head, 4);
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
