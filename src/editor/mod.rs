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
    /// The search a prompt is typing. Lives only while the prompt does, so
    /// nothing has to remember to invalidate its hits.
    search: Option<Session>,
    /// What `F3` repeats once the prompt has closed.
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

    /// The hits of the search being typed, for the renderer to underline.
    pub fn matches(&self) -> &[Range<usize>] {
        self.search.as_ref().map_or(&[], Session::hits)
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
            // While a search is up its hits are in hand and get walked; with
            // no panel at all the last query is run against the text again.
            // A panel that is not a search has nothing to find.
            Command::FindNext(direction) => match (self.search.is_some(), self.panel.is_some()) {
                (true, _) => self.go_to_match(direction),
                (false, false) => self.search_next(direction),
                (false, true) => {}
            },
            Command::Select(Select::AllOccurrences) => {
                match (self.search.is_some(), self.panel.is_some()) {
                    (true, _) => self.select_all_matches(),
                    (false, false) => self.select_all_occurrences(),
                    (false, true) => {}
                }
            }

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
        (row < view.height()).then(|| (row, column.saturating_sub(view.area.x + view.gutter)))
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
        self.close_panel();
        let mut prompt = Prompt::new(purpose.clone());

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
        // A file's own name is the likeliest start of the new one.
        if let (Purpose::SaveAs(_), Some(path)) = (&purpose, self.buffer().path()) {
            prompt.insert(&path.display().to_string());
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

    /// The situational half of the status bar belongs to whatever is open.
    pub fn panel_status(&self) -> String {
        match (&self.panel, &self.search) {
            (Some(Panel::Picker(picker)), _) => format!("{}/{}", picker.matched(), picker.total()),
            (Some(_), Some(search)) => search.status(self.view().selections.primary().min()),
            _ => String::new(),
        }
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
        self.search = None;
    }

    /// `Enter` walks the hits while searching; for anything else it answers
    /// the question and closes the strip.
    fn submit_prompt(&mut self, backwards: bool) {
        let Some(prompt) = self.prompt() else {
            return;
        };
        let purpose = prompt.purpose.clone();
        let first = prompt.contents(0);

        match purpose {
            Purpose::Search => {
                return self.go_to_match(match backwards {
                    true => Direction::Backward,
                    false => Direction::Forward,
                });
            }
            Purpose::GotoLine => match parse_line_number(&first) {
                Some(line) => self.goto_line(line),
                None => self.message = Some("Not a line number".to_owned()),
            },
            Purpose::Replace => {
                let replacement = prompt.contents(1);
                self.replace_all(&replacement);
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
    /// moves every cursor to the next tab stop from where it stands.
    fn indent(&mut self) {
        let (view, buffer) = self.workspace.focus();
        let selecting = view
            .selections
            .iter()
            .any(|selection| !selection.is_empty());
        let transaction = match selecting {
            true => lines::indent(buffer.text(), &view.selections, buffer.indent),
            false => {
                let (text, indent, tab_width) = (buffer.text(), buffer.indent, buffer.tab_width());
                Transaction::edit(&view.selections, CursorAfter::EndOfChange, |selection| {
                    let column = column_at(text, selection.head, tab_width);
                    (selection.range(), lines::to_tab_stop(indent, column))
                })
            }
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
        if let Some(transaction) = lines::toggle_comment(buffer.text(), &view.selections, token) {
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
        let transaction = Transaction::edit(
            &self.view().selections,
            CursorAfter::EndOfChange,
            |selection| {
                let head = selection.head;
                let range = if !selection.is_empty() {
                    selection.range()
                } else if let Some(range) = lines::deletion_to_tab_stop(text, indent, motion, head)
                {
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

/// The display column an offset sits in on its line.
fn column_at(text: &Text, offset: usize, tab_width: usize) -> usize {
    let line = text.line_of(offset);
    layout::column_of_byte(
        &text.line_str(line),
        offset - text.line_start(line),
        tab_width,
    )
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
