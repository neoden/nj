//! The headless run of a scenario, for the tests and the drive example
//! (ADR 25): an editor under the conditions the scenario assumes — a fake
//! clipboard, a directory of its own under `target`, a frame of the size
//! the scenario says, 40×10 until it says — started, driven and ended by
//! the scenario's lines (ADR 28). Test machinery, so debug builds only. A
//! live editor gets the same conditions from the sandbox binary, which
//! runs it as a child process (ADR 29) and shares `Workdir` and `Frames`
//! from here.

use std::cell::RefCell;
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use anyhow::{Context, Result, bail};

use super::grid::Grid;
use super::launch::{self, Args, Outcome};
use super::scenario::{self, Instruction, Line};
use super::{app, dump, render};
use crate::editor::Editor;
use crate::editor::clipboard::Desktop;

/// The frame a scenario gets until it says `size`.
const DEFAULT_SIZE: (usize, usize) = (40, 10);

/// Where the sandbox binary listens and the replay binary looks, unless
/// `--socket` says otherwise: under the project's `target`.
pub const SOCKET: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/sandbox.sock");

/// The frames a run has taken, as far as a snapshot needs them: the one
/// before, for the next to differ from. Taken headless by the run and, in
/// the debug build, over the socket by the live editor (ADR 29).
#[derive(Default)]
pub struct Frames {
    previous: Option<Grid>,
}

impl Frames {
    /// The frame as the screen shows it at `size`, dumped against the
    /// frame before.
    pub fn take(&mut self, editor: &mut Editor, size: (usize, usize)) -> String {
        // As the loop does before every frame (`app::run`): the screen is
        // divided afresh, since a panel may have opened or closed.
        editor.resize(size.0, size.1);
        let mut grid = Grid::new(size.0, size.1);
        render::compose(&mut grid, editor);
        let frame = match dump::dump(&grid, self.previous.as_ref()) {
            text if text.is_empty() => "same\n".to_owned(),
            text => text,
        };
        self.previous = Some(grid);
        frame
    }
}

/// The conditions of the run and the editor under them, once started.
struct Sandbox {
    editor: Option<Editor>,
    pasteboard: Pasteboard,
    size: (usize, usize),
    frames: Frames,
}

impl Sandbox {
    fn new() -> Self {
        Self {
            editor: None,
            pasteboard: Pasteboard::default(),
            size: DEFAULT_SIZE,
            frames: Frames::default(),
        }
    }

    /// The editor, which the run has started before it sends anything.
    fn editor(&mut self) -> &mut Editor {
        self.editor.as_mut().expect("the run starts the editor first")
    }

    /// `size`: the terminal is now this big, and the editor, which the
    /// run has started, is told so.
    fn size(&mut self, width: usize, height: usize) {
        self.size = (width, height);
        self.editor().resize(width, height);
    }

    /// Starts the editor as `args` would from the shell (ADR 28): `stdin`
    /// is the pipe, the sandbox keeps the other end of the clipboard. The
    /// socket is the process's, not the editor's, and has no meaning here.
    fn start(&mut self, args: Args, stdin: Option<String>) -> Result<()> {
        if args.socket.is_some() {
            bail!("--socket: a run has no socket");
        }
        let buffer = launch::buffer(args.file.as_deref(), stdin.map(Cursor::new))?;
        let mut editor = Editor::new(buffer).with_desktop(self.pasteboard.clone());
        editor.resize(self.size.0, self.size.1);
        self.editor = Some(editor);
        Ok(())
    }

    /// Input or `wait`, the editor's own lines. The events go through
    /// `app::handle` one at a time, so that an exit in the middle of a
    /// `type` is seen and what follows it refused. The outcome once the
    /// line has made the editor exit.
    fn send(&mut self, instruction: Instruction) -> Result<Option<Outcome>> {
        let editor = self.editor();
        match instruction {
            Instruction::Input(events) => {
                let mut events = events.into_iter();
                for event in events.by_ref() {
                    app::handle(event, editor);
                    if editor.exit().is_some() {
                        break;
                    }
                }
                if editor.exit().is_none() {
                    return Ok(None);
                }
                if events.next().is_some() {
                    bail!("input after the editor has exited");
                }
                Ok(Some(Outcome::of(editor)?))
            }
            Instruction::Wait => {
                editor.finish_jobs();
                Ok(None)
            }
            other => unreachable!("not the editor's own: {other:?}"),
        }
    }

    /// The frame as the screen shows it, dumped against the frame before.
    fn frame(&mut self) -> String {
        let editor = self.editor.as_mut().expect("the run starts the editor first");
        self.frames.take(editor, self.size)
    }
}

/// What a snapshot showed, with the caption it was taken under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub caption: String,
    pub frame: String,
}

/// Where a run is: before the editor, with it, or after it has exited.
enum Stage {
    Before,
    Running,
    Exited(Outcome),
}

/// Runs `script` against a fresh headless editor and returns its snapshots,
/// in order. An error names the line it happened on.
pub fn run(script: &str) -> Result<Vec<Snapshot>> {
    let _workdir = Workdir::enter()?;
    let mut sandbox = Sandbox::new();
    let mut stage = Stage::Before;
    // What `%stdin` gave, until the editor starts and reads it.
    let mut stdin = None;
    let mut snapshots = Vec::new();

    for (index, line) in scenario::lines(script).enumerate() {
        let at = || format!("line {}", index + 1);
        let instruction = match line {
            Line::Text(text) => match scenario::parse(text).with_context(at)? {
                Some(instruction) => instruction,
                None => continue,
            },
            Line::Snapshot(caption) => Instruction::Snapshot(caption.to_owned()),
            Line::Frame(_) => continue,
        };

        // After the exit there is only the outcome to show.
        if let Stage::Exited(outcome) = &stage {
            match instruction {
                Instruction::Snapshot(caption) => {
                    snapshots.push(Snapshot {
                        frame: scenario::outcome(outcome),
                        caption,
                    });
                    continue;
                }
                _ => bail!("{}: the editor has exited", at()),
            }
        }

        // A condition of the run — a file, the clipboard — is set before
        // the editor starts or changed while it runs, and starts nothing.
        let mut instruction = match instruction {
            Instruction::File(name, text) => {
                fs::write(&name, text)
                    .with_context(|| format!("cannot write {name}"))
                    .with_context(at)?;
                continue;
            }
            Instruction::Clipboard(text) => {
                sandbox.pasteboard.set(text);
                continue;
            }
            other => other,
        };

        // Until the editor has started, `%stdin` waits for it. `%start`
        // starts it; any other line starts it first, as `ned` alone would,
        // and is then carried out.
        if let Stage::Before = stage {
            let (args, rest) = match instruction {
                Instruction::Stdin(text) => {
                    stdin = Some(text);
                    continue;
                }
                Instruction::Start(args) => (args, None),
                other => (Args::default(), Some(other)),
            };
            sandbox.start(args, stdin.take()).with_context(at)?;
            stage = Stage::Running;
            match rest {
                Some(rest) => instruction = rest,
                None => continue,
            }
        }

        match instruction {
            Instruction::Input(_) | Instruction::Wait => {
                if let Some(outcome) = sandbox.send(instruction).with_context(at)? {
                    stage = Stage::Exited(outcome);
                }
            }
            Instruction::Size(width, height) => sandbox.size(width, height),
            Instruction::Snapshot(caption) => snapshots.push(Snapshot {
                frame: sandbox.frame(),
                caption,
            }),
            Instruction::Stdin(_) | Instruction::Start(_) => {
                bail!("{}: the editor is already running", at())
            }
            Instruction::File(..) | Instruction::Clipboard(..) => {
                unreachable!("a condition is set above")
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
pub struct Workdir {
    path: PathBuf,
    _turn: MutexGuard<'static, ()>,
}

static ONE_RUN_AT_A_TIME: Mutex<()> = Mutex::new(());

impl Workdir {
    /// The headless run's directory, under the project's `target`.
    pub fn enter() -> Result<Self> {
        let runs = concat!(env!("CARGO_MANIFEST_DIR"), "/target/scenarios");
        Self::enter_at(PathBuf::from(runs).join(std::process::id().to_string()))
    }

    /// Makes `path` and enters it; the sandbox binary puts its run under
    /// the system's temporary directory.
    pub fn enter_at(path: PathBuf) -> Result<Self> {
        // A run that panicked poisons the lock; its directory is still ours
        // to replace.
        let turn = ONE_RUN_AT_A_TIME
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
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
        let first = run("%file a.txt \"a\"\n").unwrap();
        assert!(first.is_empty());
        assert!(!PathBuf::from("a.txt").exists());

        let workdir = Workdir::enter().unwrap();
        fs::write("b.txt", "b").unwrap();
        assert!(fs::read_dir(".").unwrap().count() == 1);
        drop(workdir);
    }

    /// `start` is the command line, read by the same parser; what `stdin`
    /// gave before it is the pipe. Afterwards neither has a place.
    #[test]
    fn start_takes_the_command_line_and_stdin_the_pipe() {
        let shown = run("%stdin \"piped\"\n%start\nsize 36 2\n---\n").unwrap();
        assert_eq!(
            shown[0].frame,
            "0|1 piped\n |::^---------------------------------\n1|     [stdin] · Text · 1 lines · 1:1\n"
        );

        let shown =
            run("%file a.txt \"filed\"\n%stdin \"piped\"\n%start a.txt\nsize 30 2\n---\n").unwrap();
        assert!(shown[0].frame.contains("filed"), "{}", shown[0].frame);

        // `size` is the editor's line, so it starts the editor like any
        // other, and `%start` after it is too late.
        for script in [
            "type \"a\"\n%start\n",
            "size 30 2\n%start\n",
            "%start\n%stdin \"late\"\n",
            "%start --socket s\n",
            "%start --bogus\n",
        ] {
            let error = run(script).unwrap_err();
            assert!(
                error.to_string().starts_with("line "),
                "{script}: {error:#}"
            );
        }
    }

    /// A quit ends the run: the snapshot after it is the outcome, with what
    /// a pipeline hands on, and nothing else may follow.
    #[test]
    fn an_exit_is_shown_as_the_outcome_and_ends_the_run() {
        let shown = run("key Ctrl+Q\n--- gone\n").unwrap();
        assert_eq!(shown[0].frame, "exit 0\n");

        let shown = run("%stdin \"in\"\ntype \"x\"\nkey Ctrl+S\nkey Ctrl+Q\n---\n").unwrap();
        assert_eq!(shown[0].frame, "exit 0\nstdout \"xin\"\n");

        let shown = run("%stdin \"in\"\nkey Ctrl+Shift+Q\n---\n").unwrap();
        assert_eq!(shown[0].frame, "exit 1\nstdout \"\"\n");

        for script in ["key Ctrl+Q\ntype \"a\"\n", "key Ctrl+Q\nwait\n"] {
            let error = run(script).unwrap_err().to_string();
            assert!(error.starts_with("line 2: "), "{script}: {error}");
        }
    }
}
