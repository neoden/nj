//! The editor's dealings with files: saving, saving as, opening another
//! file in place of this one, leaving — and the questions any of those may
//! raise on the way. What a save was for is carried through the question
//! and picked up again once the text has reached the disk.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;

use super::job::Report;
use super::panel::{After, Answer, Confirm, Panel};
use super::workspace::BufferId;
use super::{Editor, Exit, key};
use crate::buffer::{Backing, Buffer, Stamp};

/// How often the file behind a buffer is looked at. Seconds are plenty: the
/// check is a `stat`, and the save it guards asks again anyway.
const WATCH_INTERVAL: Duration = Duration::from_secs(1);

impl Editor {
    /// Keeps an eye on the file behind a buffer, so that another writer is
    /// noticed before `Ctrl+S` runs into them (F1). The worker only says
    /// that the stamp moved; whether it matters is checked on arrival.
    pub(super) fn watch(&mut self, id: BufferId) {
        // Whatever file the buffer had before is no longer its concern.
        if let Some(job) = self.watchers.remove(&id) {
            self.jobs.cancel(job);
        }
        let Backing::File { path, on_disk } = self.workspace.buffer(id).backing() else {
            return;
        };
        let (path, mut seen) = (path.clone(), *on_disk);
        let job = self.jobs.spawn_forever(move |worker| {
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
        self.watchers.insert(id, job);
    }

    /// Saves, and carries on with what the save was for once the text has
    /// reached the disk — and not before, since saving may ask first.
    pub(super) fn save(&mut self, after: After) {
        // A buffer with no name yet is asked for one; what the save was for
        // rides along.
        if self.buffer().is_unnamed() {
            return self.open_save_as(after);
        }
        if self.write(after.clone()) {
            self.proceed(after);
        }
    }

    /// Writes the buffer where it came from, unless another writer got
    /// there first: then it asks rather than overwriting their work blindly
    /// (ADR 20), and the answer carries `after` on. True once written.
    fn write(&mut self, after: After) -> bool {
        if self.buffer().changed_on_disk() {
            self.panel = Some(Panel::Confirm(Confirm::overwrite(after)));
            return false;
        }
        self.write_anyway()
    }

    /// The write the question above leads to.
    fn write_anyway(&mut self) -> bool {
        let outcome = self.buffer_mut().save();
        self.report(outcome)
    }

    /// A failed save says why and goes no further, rather than losing the
    /// text that could not be written.
    fn report(&mut self, outcome: Result<()>) -> bool {
        match outcome {
            Ok(()) => true,
            Err(error) => {
                self.message = Some(format!("{error:#}"));
                false
            }
        }
    }

    fn proceed(&mut self, after: After) {
        match after {
            After::Stay => {}
            After::Quit => self.exit = Some(Exit::Quit),
            After::Open(path) => self.replace(path),
        }
    }

    /// Asks before saving over a file that is already there — before the
    /// buffer is renamed, so that declining leaves nothing half done. The
    /// buffer's own file is not "there" in that sense: that is a plain save,
    /// with the question a plain save asks.
    pub(super) fn save_as(&mut self, path: PathBuf, after: After) {
        if self.buffer().path() == Some(path.as_path()) {
            return self.save(after);
        }
        // Not a file that could be overwritten, so not worth the question.
        if path.is_dir() {
            self.message = Some(format!("{} is a directory", path.display()));
            return;
        }
        if path.exists() {
            self.panel = Some(Panel::Confirm(Confirm::replace(path, after)));
            return;
        }
        self.save_under(path, after);
    }

    fn save_under(&mut self, path: PathBuf, after: After) {
        let copy = self.buffer().is_pipeline();
        let name = path.display().to_string();
        let outcome = self.buffer_mut().save_as(path);
        if !self.report(outcome) {
            return;
        }
        let id = self.workspace.focused_buffer();
        self.watch(id);
        if copy {
            self.message = Some(format!("Copied to {name}"));
        }
        self.proceed(after);
    }

    /// The process has been asked to end — a signal, the terminal going
    /// away. The one exit that does not ask: there is nobody to ask, and
    /// the terminal may already be gone. Nothing is written; what was last
    /// saved is what stands, as after `q` to the quit question.
    pub fn terminate(&mut self) {
        self.exit = Some(Exit::Quit);
    }

    /// Nothing is ever discarded without being asked about first.
    pub(super) fn request_quit(&mut self) {
        match self.buffer().is_modified() {
            false => self.exit = Some(Exit::Quit),
            true => self.panel = Some(Panel::Confirm(Confirm::before_quitting())),
        }
    }

    /// Nothing is written and the process fails — on purpose, so it is
    /// still asked about when there is something to lose.
    pub(super) fn request_abort(&mut self) {
        match self.buffer().is_modified() {
            false => self.exit = Some(Exit::Abort),
            true => self.panel = Some(Panel::Confirm(Confirm::abort())),
        }
    }

    /// Another file in place of this one. A pipeline buffer stays: its text
    /// is owed to standard output and has no other copy.
    pub(super) fn open(&mut self, path: PathBuf) {
        if self.buffer().is_pipeline() {
            self.message = Some("A pipeline buffer cannot be replaced".to_owned());
            return;
        }
        match self.buffer().is_modified() {
            false => self.replace(path),
            true => self.panel = Some(Panel::Confirm(Confirm::before_opening(path))),
        }
    }

    /// Whatever referred to the old text goes with it.
    fn replace(&mut self, path: PathBuf) {
        let buffer = match Buffer::open(&path) {
            Ok(buffer) => buffer,
            Err(error) => {
                self.message = Some(format!("{error:#}"));
                return;
            }
        };
        self.close_panel();
        self.occurrences = None;
        self.message = None;
        let id = self.workspace.replace(buffer);
        self.refresh_search();
        self.watch(id);
    }

    /// The question closes before its answer is carried out, since the
    /// answer may raise a question of its own.
    pub(super) fn answer(&mut self, key: char) {
        let Some(Panel::Confirm(confirm)) = &self.panel else {
            return;
        };
        // The letters are commands, not text, so they are read off the
        // physical key rather than the current keyboard layout.
        let Some(answer) = confirm.answer(key::physical_key(key)) else {
            return;
        };
        self.close_panel();

        match answer {
            Answer::Save(after) => self.save(after),
            Answer::Quit => self.exit = Some(Exit::Quit),
            Answer::Open(path) => self.replace(path),
            Answer::Overwrite(after) => {
                if self.write_anyway() {
                    self.proceed(after);
                }
            }
            Answer::Replace(path, after) => self.save_under(path, after),
            Answer::Abort => self.exit = Some(Exit::Abort),
        }
    }
}
