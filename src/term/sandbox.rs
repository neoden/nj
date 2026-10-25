//! The sandbox of ADR 27: the editor under the conditions a scenario
//! assumes — a fake clipboard, a directory of its own under `target`, and a
//! frame of the size the scenario says, 40×10 until it says. Two uses: a
//! headless run of a whole scenario, for the tests and the drive example
//! (ADR 25), and a live editor taking the testing instructions over the
//! socket (ADR 26). Debug builds only.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, Result, bail};

use super::grid::Grid;
use super::scenario::{self, Instruction, Line};
use super::{app, dump, render};
use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::editor::clipboard::Desktop;

/// The frame a scenario gets until it says `size`.
const DEFAULT_SIZE: (usize, usize) = (40, 10);

/// The conditions, and what the testing instructions need between them:
/// the frame before, for a snapshot to differ from.
pub struct Sandbox {
    _workdir: Workdir,
    pasteboard: Pasteboard,
    size: (usize, usize),
    previous: Option<Grid>,
}

impl Sandbox {
    /// Enters the run's directory; it is left and removed with the sandbox.
    pub fn enter() -> Result<Self> {
        Ok(Self {
            _workdir: Workdir::enter()?,
            pasteboard: Pasteboard::default(),
            size: DEFAULT_SIZE,
            previous: None,
        })
    }

    /// The clipboard to give the editor: the sandbox keeps the other end.
    pub fn desktop(&self) -> impl Desktop + 'static {
        self.pasteboard.clone()
    }

    /// The frame to draw in a terminal of `terminal` size: the scenario's.
    /// A terminal too small for it shows what fits, and the snapshot that
    /// would not match it is refused instead (`apply`).
    pub fn frame_size(&self, terminal: (usize, usize)) -> (usize, usize) {
        (terminal.0.min(self.size.0), terminal.1.min(self.size.1))
    }

    /// Carries out a testing instruction and answers it: `ok`, or the frame
    /// for a snapshot. Input is not taken here; it enters as input does.
    /// `terminal` is the live terminal's size, or `None` in a headless run;
    /// a frame the terminal cannot show whole is refused, since the screen
    /// and the snapshot would then say different things (ADR 27).
    pub fn apply(
        &mut self,
        editor: &mut Editor,
        instruction: Instruction,
        terminal: Option<(usize, usize)>,
    ) -> Result<String> {
        match instruction {
            Instruction::Input(_) => unreachable!("input is handled as input"),
            Instruction::Size(width, height) => {
                fits(terminal, (width, height))?;
                self.size = (width, height);
                editor.resize(width, height);
            }
            Instruction::File(name, text) => {
                fs::write(&name, text).with_context(|| format!("cannot write {name}"))?;
            }
            Instruction::Clipboard(text) => self.pasteboard.set(text),
            Instruction::Wait => editor.finish_jobs(),
            Instruction::Snapshot(_) => {
                fits(terminal, self.size)?;
                return Ok(self.frame(editor));
            }
        }
        Ok("ok\n".to_owned())
    }

    /// The frame as the screen shows it, dumped against the frame before.
    fn frame(&mut self, editor: &mut Editor) -> String {
        // As the loop does before every frame (`app::run`): the screen is
        // divided afresh, since a panel may have opened or closed.
        editor.resize(self.size.0, self.size.1);
        let mut grid = Grid::new(self.size.0, self.size.1);
        render::compose(&mut grid, editor);
        let frame = match dump::dump(&grid, self.previous.as_ref()) {
            text if text.is_empty() => "same\n".to_owned(),
            text => text,
        };
        self.previous = Some(grid);
        frame
    }
}

fn fits(terminal: Option<(usize, usize)>, frame: (usize, usize)) -> Result<()> {
    if let Some((columns, rows)) = terminal
        && (columns < frame.0 || rows < frame.1)
    {
        bail!(
            "the terminal is {columns}×{rows}, too small for a frame of {}×{}: enlarge it",
            frame.0,
            frame.1
        );
    }
    Ok(())
}

/// What a snapshot showed, with the caption it was taken under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub caption: String,
    pub frame: String,
}

/// Runs `script` against a fresh headless editor and returns its snapshots,
/// in order. An error names the line it happened on.
pub fn run(script: &str) -> Result<Vec<Snapshot>> {
    let mut sandbox = Sandbox::enter()?;
    let mut editor = Editor::new(Buffer::default()).with_desktop(sandbox.desktop());
    editor.resize(DEFAULT_SIZE.0, DEFAULT_SIZE.1);
    let mut snapshots = Vec::new();

    for (index, line) in scenario::lines(script).enumerate() {
        let at = || format!("line {}", index + 1);
        let instruction = match line {
            Line::Text(line) => scenario::parse(line).with_context(at)?,
            Line::Snapshot(caption) => Some(Instruction::Snapshot(caption.to_owned())),
            Line::Frame(_) => None,
        };
        match instruction {
            None => {}
            Some(Instruction::Input(events)) => {
                for event in events {
                    app::handle(event, &mut editor);
                }
            }
            Some(Instruction::Snapshot(caption)) => snapshots.push(Snapshot {
                frame: sandbox.frame(&mut editor),
                caption,
            }),
            Some(other) => {
                sandbox.apply(&mut editor, other, None).with_context(at)?;
            }
        }
    }
    Ok(snapshots)
}

/// The fake desktop clipboard. The sandbox shares it with the editor to
/// set what is on it from outside.
#[derive(Clone, Default)]
struct Pasteboard(Rc<RefCell<Option<String>>>);

impl Pasteboard {
    fn set(&self, text: String) {
        *self.0.borrow_mut() = Some(text);
    }
}

impl Desktop for Pasteboard {
    fn read(&mut self) -> Option<String> {
        self.0.borrow().clone()
    }

    fn write(&mut self, text: &str) {
        self.set(text.to_owned());
    }
}

/// The run's directory: made on entry, left on exit, taken down with
/// everything in it when the run ends, however it ends. A process has one
/// working directory, so it has one run at a time: a second `enter` in the
/// same process waits for the first to end, which is what lets the unit
/// tests here run in parallel with each other.
struct Workdir {
    path: PathBuf,
    _turn: MutexGuard<'static, ()>,
}

static ONE_RUN_AT_A_TIME: Mutex<()> = Mutex::new(());

impl Workdir {
    fn enter() -> Result<Self> {
        // A run that panicked poisons the lock; its directory is still ours
        // to replace.
        let turn = ONE_RUN_AT_A_TIME
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let runs = concat!(env!("CARGO_MANIFEST_DIR"), "/target/scenarios");
        let path = PathBuf::from(runs).join(std::process::id().to_string());
        fs::create_dir_all(&path)?;
        std::env::set_current_dir(&path)?;
        Ok(Self { path, _turn: turn })
    }
}

impl Drop for Workdir {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"));
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_answers_each_snapshot_and_names_the_line_of_an_error() {
        let shown = run("size 6 2\ntype \"ab\"\n--- typed\n\n---\n").unwrap();
        assert_eq!(shown[0].caption, "typed");
        assert_eq!(shown[0].frame, "0|1 ab\n |::--^-\n1| …1:3\n");
        assert_eq!(shown[1].frame, "same\n");

        let error = run("type \"a\"\nbogus\n").unwrap_err();
        assert_eq!(error.to_string(), "line 2");
    }

    /// Two runs in one process take turns: each has the directory to itself
    /// while it lasts, and what one writes the other never sees.
    #[test]
    fn runs_in_one_process_do_not_share_the_directory() {
        let first = run("file a.txt \"a\"\n").unwrap();
        assert!(first.is_empty());
        assert!(!PathBuf::from("a.txt").exists());

        let sandbox = Sandbox::enter().unwrap();
        fs::write("b.txt", "b").unwrap();
        assert!(fs::read_dir(".").unwrap().count() == 1);
        drop(sandbox);
    }

    #[test]
    fn a_frame_the_terminal_cannot_show_whole_is_refused() {
        let mut sandbox = Sandbox::enter().unwrap();
        let mut editor = Editor::new(Buffer::default());
        assert_eq!(sandbox.frame_size((80, 24)), DEFAULT_SIZE);

        let error = sandbox
            .apply(&mut editor, Instruction::Size(100, 5), Some((80, 24)))
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with("the terminal is 80×24, too small"),
            "{error}"
        );
        assert_eq!(sandbox.frame_size((80, 24)), DEFAULT_SIZE);

        // The terminal shrank after the size was set: the snapshot says so.
        assert!(
            sandbox
                .apply(
                    &mut editor,
                    Instruction::Snapshot(String::new()),
                    Some((80, 8))
                )
                .is_err()
        );
        // Headless there is no terminal to be too small.
        assert!(
            sandbox
                .apply(&mut editor, Instruction::Size(100, 5), None)
                .is_ok()
        );
    }
}
