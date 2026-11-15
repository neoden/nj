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
pub mod style;
pub mod ui_layout;
pub mod view;
pub mod workspace;

use std::collections::HashMap;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use crate::buffer::language::{self, Language};
use crate::buffer::{Buffer, EditKind};
use crate::text::layout;
use crate::text::movement::{self, Class, Direction, Motion};
use crate::text::search::{Query, Session};
use crate::text::selection::{Selection, SelectionSet};
use crate::text::transaction::{CursorAfter, Transaction};
use clipboard::{Clipboard, Desktop};
use command::{ClickKind, Command, Edit, File, PanelCommand, Select};
use find::Occurrences;
use job::{JobId, Jobs, Message, Report};
use panel::field::Field;
use panel::find::Find;
use panel::goto_line::GotoLine;
use panel::open_file::OpenFile;
use panel::picker::{self, Candidate, Pick, Picker};
use panel::replace::Replace;
use panel::save_as::SaveAs;
use panel::{After, Panel};
use ui_layout::UiLayout;
use view::View;
use workspace::{BufferId, Workspace};

/// How the editor is leaving: the usual way, handing on what was saved, or
/// aborting — nothing written, and a failure code for whoever is waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    Quit,
    Abort,
}

/// The editor behind its boundary: commands go in through
/// `execute`, reports from its own workers through `receive`, and what comes
/// out is read-only. No `&mut` to anything inside leaves this type, and no
/// thread but the one holding it ever touches it.
pub struct Editor {
    workspace: Workspace,
    /// How the terminal was last divided; what the focused view's area came
    /// from.
    ui_layout: UiLayout,
    clipboard: Clipboard,
    panel: Option<Panel>,
    /// Shown on the status bar until the next command.
    message: Option<String>,
    /// The search in progress. The search field starts it and it stays when
    /// the field closes, so its hits are found again whenever the text
    /// changes.
    search: Option<Session>,
    /// The query searched for last, for when no search is going on: `F3`
    /// brings it back, and the search field opens holding it.
    last: Option<Query>,
    /// What was replaced with last; the replace form opens holding it, so a
    /// walk that was stopped is picked up with `Enter`.
    last_replacement: Option<String>,
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
            ui_layout: UiLayout::default(),
            clipboard: Clipboard::default(),
            panel: None,
            message: None,
            search: None,
            last: None,
            last_replacement: None,
            exit: None,
            occurrences: None,
            desktop: None,
            jobs,
            watchers: HashMap::new(),
            inbox,
        };
        editor.watch(editor.workspace.focused_buffer());
        // The buffer comes unparsed. Later ones are opened by a command,
        // and `execute` starts their parse.
        editor.parse(editor.workspace.focused_buffer());
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
            Report::Parsed { buffer, fresh } => {
                if let Some(open) = self.workspace.buffer_if_open(buffer) {
                    open.take_syntax(fresh);
                    // An edit made meanwhile has left the tree behind
                    // again: the next parse starts now.
                    self.parse(buffer);
                }
            }
            Report::Ended(job) => self.jobs.ended(job),
        }
        // As at the end of `execute`: a report may one day change the text.
        self.session();
    }

    /// Handles every report the tasks in flight will send, and returns when
    /// they have all ended. For tests, which want a picker filled before
    /// they type into it; the loop never needs it. Whatever else arrives
    /// meanwhile is dropped — a remote line with it, which closes that
    /// connection, since a client waits for each answer before it sends the
    /// next line and so has nothing in flight while this runs.
    pub fn finish_jobs(&mut self) {
        while self.jobs.any_pending() {
            match self.wait() {
                Some(Message::Report(report)) => self.receive(report),
                Some(Message::Input(_) | Message::Remote(_) | Message::Terminate) => {}
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

    /// Whether a search is in progress, whether or not it has found
    /// anything.
    pub fn has_search(&self) -> bool {
        self.search.is_some()
    }

    pub fn ui_layout(&self) -> &UiLayout {
        &self.ui_layout
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
        self.ui_layout = UiLayout::divide(term_width, term_height, panel);
        view.resize(self.ui_layout.text, buffer);
    }

    /// The only way in. Routes to the method that does the work;
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
            // Both are commands of the text. An open panel has the keyboard
            // and no hits of its own, whatever it is for.
            Command::FindNext(direction) => match self.panel.is_some() {
                true => {}
                false => self.search_next(direction),
            },
            Command::Select(Select::AllOccurrences) => {
                match (self.panel.is_some(), self.has_search()) {
                    (true, _) => {}
                    // What is lit is what gets the cursors, wherever the
                    // cursor happens to stand.
                    (false, true) => self.select_all_matches(),
                    (false, false) => self.select_all_occurrences(),
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
            Command::Select(Select::Click { row, column, kind }) => self.click(row, column, kind),
            Command::Select(Select::Drag { row, column }) => self.drag(row, column),

            Command::File(File::Save) => self.save(After::Stay),
            Command::File(File::SaveAs(path)) => self.save_as(path, After::Stay),
            Command::File(File::Open(path)) => self.open(path),
            Command::File(File::SetLanguage(name)) => match language::named(&name) {
                Some(language) => self.set_language(language),
                None => self.message = Some(format!("No language called {name}")),
            },

            Command::Panel(PanelCommand::Find) => self.open_find(),
            Command::Panel(PanelCommand::Search(direction)) => self.submit_find(direction),
            Command::Panel(PanelCommand::Replace) => self.open_replace(),
            Command::Panel(PanelCommand::StartReplacing) => self.submit_replace(),
            Command::Panel(PanelCommand::GotoLine) => self.open_goto_line(),
            Command::Panel(PanelCommand::Jump) => self.submit_goto_line(),
            Command::Panel(PanelCommand::OpenFile) => self.open_open_file(),
            Command::Panel(PanelCommand::OpenPath) => self.submit_open_file(),
            Command::Panel(PanelCommand::SaveAs(after)) => self.open_save_as(after),
            Command::Panel(PanelCommand::SavePath) => self.submit_save_as(),
            Command::Panel(PanelCommand::Picker(pick)) => self.open_picker(pick),
            Command::Panel(PanelCommand::Close) => self.close_panel(),
            Command::Panel(PanelCommand::Pick) => self.accept_pick(),
            Command::Panel(PanelCommand::Answer(answer)) => self.answer(answer),
            Command::Panel(PanelCommand::ReplaceHit) => self.replace_hit(),
            Command::Panel(PanelCommand::SkipHit) => self.skip_hit(),
            Command::Panel(PanelCommand::ReplaceAll) => self.replace_rest(),
            Command::Panel(PanelCommand::NextField) => {
                if let Some(Panel::Replace(replace)) = &mut self.panel {
                    replace.switch_field();
                }
            }
            Command::Panel(PanelCommand::MoveChoice(delta)) => {
                if let Some(Panel::Picker(picker)) = &mut self.panel {
                    picker.move_selection(delta);
                }
            }
        }
        // The frame drawn next reads the hits through `&self` and cannot
        // catch them up itself, so they are caught up here, whatever the
        // command did to the text.
        self.session();
        // The syntax tree is not caught up here: an edit left it with the
        // structure of the text before, the frame draws from it as it is,
        // and the parse that brings it up to the text is a job.
        self.parse(self.workspace.focused_buffer());
    }

    /// Starts the parse of a buffer's text on a worker, if the text was
    /// edited since the last parse and no parse of it is running. The tree
    /// comes back as a report.
    fn parse(&mut self, buffer: BufferId) {
        let Some(work) = self.workspace.buffer_mut(buffer).syntax_work() else {
            return;
        };
        self.jobs.spawn(move |worker| {
            worker.report(Report::Parsed {
                buffer,
                fresh: work.run(),
            });
        });
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

    /// An edited query sifts a picker's list as it is typed. The fields of
    /// the other forms drive nothing until `Enter`.
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
        let snapped = selections.mapped(|selection| Selection {
            anchor: text.snap(selection.anchor),
            head: text.snap(selection.head),
            goal_column: None,
        });
        view.place(buffer, snapped);
    }

    /// Screen cells into the view's own coordinates: past its gutter, down
    /// from its top. A click outside the text is nobody's.
    fn cell_in_view(&self, row: usize, column: usize) -> Option<(usize, usize)> {
        let view = self.view();
        let row = row.checked_sub(view.area.y)?;
        (row < view.height()).then(|| (row, column.saturating_sub(view.area.x + view.gutter)))
    }

    fn click(&mut self, row: usize, column: usize, kind: ClickKind) {
        let Some((row, column)) = self.cell_in_view(row, column) else {
            return;
        };
        // A click in the text takes the keyboard back there, and whatever
        // panel was up — a field, a list, a question — gives way to it, as
        // it would to Esc. The click is then a click: it puts the cursor
        // where it landed, so that a drag from it has a cursor to start at.
        self.close_panel();
        let (view, buffer) = self.workspace.focus_mut();
        let offset = view.offset_at_screen(buffer, row, column);
        let clicked = match kind {
            ClickKind::Place => SelectionSet::single(offset),
            ClickKind::Add => {
                let mut added = view.selections().clone();
                added.push(Selection::cursor(offset));
                added
            }
            // Extending is one selection's business: the primary keeps its
            // anchor, and any other cursor goes, as under a plain click.
            ClickKind::Extend => Selection {
                anchor: view.selections().primary().anchor,
                head: offset,
                goal_column: None,
            }
            .into(),
        };
        // The click landed on screen, so the screen stays: a cursor placed
        // on the top row must not pull the margin above it into view.
        view.put(clicked);
    }

    /// A drag follows a click, which closed any panel there was. One still
    /// open means the button went down outside the text, and so the drag
    /// has no cursor to start at.
    fn drag(&mut self, row: usize, column: usize) {
        if self.panel.is_some() {
            return;
        }
        let Some((row, column)) = self.cell_in_view(row, column) else {
            return;
        };
        let (view, buffer) = self.workspace.focus_mut();
        let dragged = Selection {
            anchor: view.selections().primary().anchor,
            head: view.offset_at_screen(buffer, row, column),
            goal_column: None,
        };
        view.place(buffer, dragged.into());
    }

    /// What a query field opens holding. A selection spanning lines means
    /// something else entirely, so only a fragment is taken; with nothing
    /// selected it is the query of the search going on, or of the last one.
    fn query_seed(&self) -> Option<String> {
        let previous = self
            .search
            .as_ref()
            .map(Session::query)
            .or(self.last.as_ref())
            .map(|query| query.needle.clone());
        self.selected_text()
            .filter(|text| !text.contains('\n'))
            .or(previous)
    }

    /// Opening starts nothing and ends nothing: a search going on goes on
    /// until `Enter` puts another in its place.
    fn open_find(&mut self) {
        self.close_panel();
        let seed = self.query_seed();
        self.panel = Some(Panel::Find(Find::new(seed.as_deref())));
    }

    /// `Enter` in the search field: it closes, and the search starts.
    fn submit_find(&mut self, direction: Direction) {
        let Some(Panel::Find(find)) = &self.panel else {
            return;
        };
        let needle = find.query.contents();
        self.close_panel();
        self.start_search(needle, direction);
    }

    /// Like the search field, opening the replace fields touches no search.
    fn open_replace(&mut self) {
        self.close_panel();
        let seed = self.query_seed();
        self.panel = Some(Panel::Replace(Replace::new(
            seed.as_deref(),
            self.last_replacement.as_deref(),
        )));
    }

    /// `Enter` in the replace fields: they close, and the question is asked
    /// at the first hit.
    fn submit_replace(&mut self) {
        let Some(Panel::Replace(replace)) = &self.panel else {
            return;
        };
        let (needle, replacement) = (replace.query.contents(), replace.replacement.contents());
        self.close_panel();
        self.start_replacing(needle, replacement);
    }

    fn open_goto_line(&mut self) {
        self.close_panel();
        self.panel = Some(Panel::GotoLine(GotoLine::new()));
    }

    /// `Enter` in the line number field: it closes, and the cursor goes to
    /// the line, or the bar says that what was typed is not one.
    fn submit_goto_line(&mut self) {
        let Some(Panel::GotoLine(goto_line)) = &self.panel else {
            return;
        };
        let typed = goto_line.line.contents();
        self.close_panel();
        match parse_line_number(&typed) {
            Some(line) => self.goto_line(line),
            None => self.message = Some("Not a line number".to_owned()),
        }
    }

    fn open_open_file(&mut self) {
        self.close_panel();
        self.panel = Some(Panel::OpenFile(OpenFile::new()));
    }

    /// `Enter` in the field for a path to open: it closes, and the file is
    /// opened. With nothing typed there is nothing to open.
    fn submit_open_file(&mut self) {
        let Some(Panel::OpenFile(open_file)) = &self.panel else {
            return;
        };
        let typed = open_file.path.contents();
        self.close_panel();
        if !typed.trim().is_empty() {
            self.open(PathBuf::from(typed.trim()));
        }
    }

    pub(super) fn open_save_as(&mut self, after: After) {
        self.close_panel();
        let name = self.buffer().path().map(|path| path.display().to_string());
        self.panel = Some(Panel::SaveAs(SaveAs::new(after, name.as_deref())));
    }

    /// `Enter` in the field for a name to save under: it closes, and the
    /// buffer is saved there. With nothing typed there is nothing to do, and
    /// what the save was for is dropped with the field.
    fn submit_save_as(&mut self) {
        let Some(Panel::SaveAs(save_as)) = &self.panel else {
            return;
        };
        let (typed, after) = (save_as.path.contents(), save_as.after.clone());
        self.close_panel();
        if !typed.trim().is_empty() {
            self.save_as(PathBuf::from(typed.trim()), after);
        }
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
        let Some(search) = &self.search else {
            return String::new();
        };
        let needle = quoted(&search.query().needle);
        // A search with nothing to count is still on, and says so
        // the way the message did, rather than as a count of zero.
        if search.hits().is_empty() {
            return format!("Not found {needle}");
        }
        let count = search.count(self.view().selections().primary().min());
        format!("{count} {needle}")
    }

    fn selected_text(&self) -> Option<String> {
        let primary = self.view().selections().primary();
        match primary.is_empty() {
            true => None,
            false => Some(self.buffer().text().substring(primary.range())),
        }
    }

    /// Closes the panel. A search going on is not the panel's, and stays.
    fn close_panel(&mut self) {
        if let Some(Panel::Picker(picker)) = &self.panel {
            self.jobs.cancel(picker.job);
        }
        // A replacement is over when its question closes, however it closes.
        if let Some(Panel::Replacing(replacing)) = &self.panel {
            self.message = Some(format!("Replaced {}", replacing.replaced));
            self.end_search();
        }
        self.panel = None;
    }

    fn goto_line(&mut self, line: usize) {
        let (view, buffer) = self.workspace.focus_mut();
        let text = buffer.text();
        let line = line.min(text.line_count().saturating_sub(1));
        view.place(buffer, Selection::cursor(text.line_start(line)).into());
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
    /// typing at the keyboard and a replacement at a hit, and only the
    /// first of those belongs in a typing group.
    fn insert(&mut self, inserted: &str, kind: EditKind) {
        let transaction = Transaction::edit(
            self.view().selections(),
            CursorAfter::EndOfChange,
            |selection| (selection.range(), inserted.to_owned()),
        );
        self.apply(transaction, kind);
    }

    fn copy(&mut self) {
        let (view, buffer) = self.workspace.focus();
        self.clipboard.copy_from(buffer.text(), view.selections());

        let text = self.clipboard.text();
        if let Some(desktop) = &mut self.desktop {
            desktop.write(&text);
        }
    }

    fn cut(&mut self) {
        self.copy();
        let (view, buffer) = self.workspace.focus();
        if let Some(transaction) = self.clipboard.cut_from(buffer.text(), view.selections()) {
            self.apply(transaction, EditKind::Other);
        }
    }

    fn paste(&mut self) {
        self.adopt_foreign_clipboard();
        if self.clipboard.is_empty() {
            return;
        }
        let (view, buffer) = self.workspace.focus();
        let transaction = self.clipboard.paste_into(buffer.text(), view.selections());
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
        let transaction = lines::newline(buffer.text(), view.selections());
        self.apply(transaction, EditKind::Other);
    }

    /// `Tab` indents whole lines when a selection crosses one, and otherwise
    /// moves every cursor to the next tab stop from where it stands.
    fn indent(&mut self) {
        let (view, buffer) = self.workspace.focus();
        let selecting = view
            .selections()
            .iter()
            .any(|selection| !selection.is_empty());
        let transaction = match selecting {
            true => lines::indent(buffer.text(), view.selections(), buffer.indent()),
            false => lines::tab_stops(
                buffer.text(),
                view.selections(),
                buffer.indent(),
                buffer.tab_width(),
            ),
        };
        self.apply(transaction, EditKind::Other);
    }

    fn outdent(&mut self) {
        let (view, buffer) = self.workspace.focus();
        if let Some(transaction) = lines::outdent(buffer.text(), view.selections(), buffer.indent())
        {
            self.apply(transaction, EditKind::Other);
        }
    }

    fn toggle_comment(&mut self) {
        let (view, buffer) = self.workspace.focus();
        let Some(token) = buffer.language().line_comment else {
            self.message = Some("No line comment for this language".to_owned());
            return;
        };
        if let Some(transaction) =
            lines::toggle_comment(buffer.text(), view.selections(), token, buffer.tab_width())
        {
            self.apply(transaction, EditKind::Other);
        }
    }

    pub fn cursor_column(&self) -> usize {
        let (view, buffer) = self.workspace.focus();
        layout::column_at(
            buffer.text(),
            view.selections().primary().head,
            buffer.tab_width(),
        )
    }

    /// A cursor with nothing selected eats the neighbouring text instead.
    fn delete(&mut self, motion: Motion) {
        let text = self.buffer().text();
        let indent = self.buffer().indent();
        // Cursors are disjoint, but what each one reaches for is not: two
        // cursors in one word both want the whole word. What an earlier
        // cursor takes is not taken again.
        let mut taken = 0;
        let changes = self.view().selections().iter().map(|selection| {
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
            Transaction::over(self.view().selections(), CursorAfter::EndOfChange, changes);

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
        let primary = view.selections().primary();
        view.place(buffer, primary.into());
    }

    fn cancel_selection(&mut self) {
        let view = self.view_mut();
        let cursors = view
            .selections()
            .mapped(|selection| Selection::cursor(selection.head));
        view.put(cursors);
    }

    /// Takes the lines a selection touches, break included. Pressing again
    /// swallows the next line, because the selection now ends on its start.
    fn select_line(&mut self) {
        let (view, buffer) = self.workspace.focus_mut();
        let text = buffer.text();
        let lines = view.selections().mapped(|selection| {
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
        view.place(buffer, lines);
    }

    /// The view stays where it is: jumping to the end of the file is not what
    /// anyone wants from selecting everything.
    fn select_all(&mut self) {
        let (view, buffer) = self.workspace.focus_mut();
        let all = Selection {
            anchor: 0,
            head: buffer.text().len(),
            goal_column: None,
        };
        view.put(all.into());
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
        if let Some(selections) = self.workspace.focus_edit().1.undo() {
            self.edited(selections);
        }
    }

    fn redo(&mut self) {
        if let Some(selections) = self.workspace.focus_edit().1.redo() {
            self.edited(selections);
        }
    }

    /// The buffer changes here, in `undo` and in `redo`, and nowhere else
    /// in the editor.
    fn apply(&mut self, transaction: Transaction, kind: EditKind) {
        let selections = self.workspace.focus_edit().1.apply(transaction, kind);
        self.edited(selections);
    }

    /// The text of the focused buffer has just changed, and these are the
    /// cursors the change left behind. They are placed in the current text,
    /// so there is nothing to catch up on. The search is not told: it sees
    /// the version move when its hits are next asked for.
    fn edited(&mut self, selections: SelectionSet) {
        let (view, buffer) = self.workspace.focus_mut();
        view.place(buffer, selections);
        view.mark_current(buffer);
    }

    /// The search going on, with its hits brought up to the text as it is
    /// now. The one way a command reads them, so that a hit found before
    /// an edit cannot be used after it.
    pub(super) fn session(&mut self) -> Option<&Session> {
        let buffer = self.workspace.focus().1;
        let search = self.search.as_mut()?;
        search.catch_up(buffer.text(), buffer.version());
        Some(search)
    }
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
