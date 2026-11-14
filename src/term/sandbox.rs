//! The headless run of a scenario, for the tests and the drive example:
//! an editor under the conditions the scenario assumes — a fake
//! clipboard, a directory of its own under `target`, a frame of the size
//! the scenario says, 40×10 until it says — started, driven and ended by
//! the scenario's lines. Test machinery, so debug builds only. A
//! live editor gets the same conditions from the sandbox binary, which
//! runs it as a child process and shares `Workdir` and `Frames`
//! from here.

use std::cell::RefCell;
use std::fs;
use std::io::{self, Cursor};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use rustix::process::{Pid, Signal, kill_process_group, test_kill_process_group};

use super::grid::Grid;
use super::launch::{self, Args, Outcome};
use super::scenario::{self, Expect, Instruction, Line};
use super::screen::{Frames, Layer, Screen};
use super::{app, render};
use crate::buffer::syntax;
use crate::editor::Editor;
use crate::editor::clipboard::Desktop;

/// The frame a scenario gets until it says `size`.
const DEFAULT_SIZE: (usize, usize) = (40, 10);

/// Where the sandbox binary listens and the replay binary looks, unless
/// `--socket` says otherwise: under the project's `target`.
pub const SOCKET: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/sandbox.sock");

/// The screen at `size`, as the loop would draw it (`app::run`): divided
/// afresh first, since a panel may have opened or closed.
pub fn grid(editor: &mut Editor, size: (usize, usize)) -> Grid {
    editor.resize(size.0, size.1);
    let mut grid = Grid::new(size.0, size.1);
    render::compose(&mut grid, editor);
    grid
}

/// Whether a `?` line holds. `ended` is how the editor exited, `None`
/// while it runs; `clipboard` is what the desktop clipboard holds; a file
/// is read from the run's directory, which is the working directory. The
/// error says what was found and what was claimed, in the language's own
/// quoting. Shared with the sandbox binary, whose run has the same things
/// to check and only keeps them elsewhere.
pub fn check(expect: &Expect, ended: Option<&Outcome>, clipboard: Option<String>) -> Result<()> {
    use scenario::quote;
    match expect {
        Expect::Exit(code) => match ended {
            None => bail!("the editor is still running"),
            Some(outcome) if outcome.code != *code => bail!("exit {}, not {code}", outcome.code),
            Some(_) => Ok(()),
        },
        Expect::Stdout(text) => match ended.map(|outcome| outcome.stdout.as_ref()) {
            None => bail!("the editor is still running"),
            Some(None) => bail!("not a pipeline: nothing is handed downstream"),
            Some(Some(stdout)) if stdout != text => {
                bail!("stdout {}, not {}", quote(stdout), quote(text))
            }
            Some(Some(_)) => Ok(()),
        },
        Expect::File(name, text) => match fs::read_to_string(name) {
            Ok(held) if &held == text => Ok(()),
            Ok(held) => bail!("{name} holds {}, not {}", quote(&held), quote(text)),
            Err(error) => bail!("{name}: {error}"),
        },
        Expect::NoPath(name) => match fs::symlink_metadata(name) {
            Ok(_) => bail!("{name} is there"),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => bail!("{name}: {error}"),
        },
        Expect::Clipboard(text) => match clipboard {
            Some(held) if &held == text => Ok(()),
            Some(held) => bail!("the clipboard holds {}, not {}", quote(&held), quote(text)),
            None => bail!("the clipboard holds nothing, not {}", quote(text)),
        },
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
        self.editor
            .as_mut()
            .expect("the run starts the editor first")
    }

    /// `size`: the terminal is now this big, and the editor, which the
    /// run has started, is told so.
    fn size(&mut self, width: usize, height: usize) {
        self.size = (width, height);
        self.editor().resize(width, height);
    }

    /// Starts the editor as `args` would from the shell:`stdin`
    /// is the pipe, the sandbox keeps the other end of the clipboard. The
    /// socket is the process's, not the editor's, and has no meaning here.
    /// An editor started after another begins as the first did, with the
    /// default frame and no frame before it, as a new process would.
    fn start(&mut self, args: Args, stdin: Option<String>) -> Result<()> {
        if args.socket.is_some() {
            bail!("--socket: a run has no socket");
        }
        let buffer = launch::buffer(&args, stdin.map(Cursor::new))?;
        let mut editor = Editor::new(buffer).with_desktop(self.pasteboard.clone());
        self.size = DEFAULT_SIZE;
        self.frames = Frames::default();
        editor.resize(self.size.0, self.size.1);
        self.editor = Some(editor);
        Ok(())
    }

    /// Input or `wait`, the editor's own lines. The outcome once the line
    /// has made the editor exit.
    fn send(&mut self, instruction: Instruction) -> Result<Option<Outcome>> {
        let editor = self.editor();
        match instruction {
            Instruction::Input(events) => {
                app::handle_all(events, editor)?;
                match editor.exit() {
                    None => Ok(None),
                    Some(_) => Ok(Some(Outcome::of(editor)?)),
                }
            }
            Instruction::Wait => {
                editor.finish_jobs();
                Ok(None)
            }
            other => unreachable!("not the editor's own: {other:?}"),
        }
    }

    /// The frame of `layer`: what changed on the screen since the last
    /// snapshot of that layer.
    fn frame(&mut self, layer: Layer) -> String {
        let editor = self
            .editor
            .as_mut()
            .expect("the run starts the editor first");
        let screen = Screen::of(&grid(editor, self.size));
        self.frames.take(screen, layer)
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

/// How a run ended.
#[derive(Debug)]
pub enum Run {
    /// Through to the end, with its snapshots in order.
    Done(Vec<Snapshot>),
    /// Stopped at a `%grammar` line: this build has no grammar of the
    /// language named, and what the frames show depends on it.
    Skipped(&'static str),
}

/// Runs `script` against a fresh headless editor. An error names the line
/// it happened on.
pub fn run(script: &str) -> Result<Run> {
    let workdir = Workdir::enter()?;
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
            Line::Snapshot(layer, caption) => Instruction::Snapshot(layer, caption.to_owned()),
            Line::Frame(_) => continue,
        };

        // A condition of the run — a file, the clipboard — is set whenever
        // the line says so, before the editor, beside it or after it, and
        // starts nothing.
        let mut instruction = match instruction {
            Instruction::File(name, text) => {
                fs::write(&name, text)
                    .with_context(|| format!("cannot write {name}"))
                    .with_context(at)?;
                continue;
            }
            Instruction::Shell(command) => {
                workdir.shell(&command).with_context(at)?;
                continue;
            }
            Instruction::Clipboard(text) => {
                sandbox.pasteboard.set(text);
                continue;
            }
            // Checked when read, whatever the stage: a `?exit` needs the
            // editor gone, a `?file` does not care.
            Instruction::Expect(expect) => {
                let ended = match &stage {
                    Stage::Exited(outcome) => Some(outcome),
                    _ => None,
                };
                let clipboard = sandbox.pasteboard.read();
                check(&expect, ended, clipboard).with_context(at)?;
                continue;
            }
            // Whatever the stage: the editor, if any, is dropped with the
            // rest of the run, and the next line finds the run as the
            // first line did.
            Instruction::Reset => {
                workdir.renew().with_context(at)?;
                sandbox = Sandbox::new();
                stage = Stage::Before;
                stdin = None;
                continue;
            }
            Instruction::Grammar(language) => {
                if !syntax::has_grammar(language) {
                    return Ok(Run::Skipped(language.name));
                }
                continue;
            }
            other => other,
        };

        // After the exit the editor's own lines have no editor to go to,
        // and a snapshot has no screen to show: `?exit` and `?stdout` say
        // what became of it. `%stdin` and `%start` make the next one, below.
        if let Stage::Exited(_) = &stage {
            match &instruction {
                Instruction::Stdin(_) | Instruction::Start(_) => {}
                _ => bail!("{}: the editor has exited", at()),
            }
        }

        // While there is no editor, `%stdin` waits for one. `%start` starts
        // it; before the first, any other line starts it as `nj` alone
        // would, and is then carried out.
        if !matches!(stage, Stage::Running) {
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
            Instruction::Snapshot(layer, caption) => snapshots.push(Snapshot {
                frame: sandbox.frame(layer),
                caption,
            }),
            Instruction::Stdin(_) | Instruction::Start(_) => {
                bail!("{}: the editor is already running", at())
            }
            Instruction::File(..)
            | Instruction::Shell(_)
            | Instruction::Clipboard(..)
            | Instruction::Expect(..)
            | Instruction::Reset
            | Instruction::Grammar(_) => unreachable!("the run's own lines are done above"),
        }
    }
    Ok(Run::Done(snapshots))
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
    /// The process groups of the `%shell` lines that left something
    /// running in the background: it is in the line's group.
    shells: RefCell<Vec<Pid>>,
    _turn: MutexGuard<'static, ()>,
}

static ONE_RUN_AT_A_TIME: Mutex<()> = Mutex::new(());

/// How long a `%shell` command may take. One that waits for what the
/// scenario never gives, a reader at a named pipe say, is ended here
/// instead of holding the run forever.
const SHELL_LIMIT: Duration = Duration::from_secs(5);

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
        Ok(Self {
            path,
            shells: RefCell::default(),
            _turn: turn,
        })
    }

    /// `%shell`: `command` run by `sh` in the directory and waited for, up
    /// to `SHELL_LIMIT`. It gets a process group of its own, so that what
    /// it starts in the background, a writer waiting at a named pipe say,
    /// can be ended with the run. Its output is not read: a background
    /// process would hold the pipe open and the wait would never end. Its
    /// errors go where the run's own do.
    pub fn shell(&self, command: &str) -> Result<()> {
        self.shell_within(command, SHELL_LIMIT)
    }

    /// `shell` with the limit given: the tests do not wait five seconds.
    fn shell_within(&self, command: &str, limit: Duration) -> Result<()> {
        let mut shell = Command::new("sh")
            .args(["-c", command])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .process_group(0)
            .spawn()
            .context("cannot run sh")?;
        let group = Pid::from_child(&shell);
        let deadline = Instant::now() + limit;
        let status = loop {
            if let Some(status) = shell.try_wait().context("cannot wait for sh")? {
                break status;
            }
            if Instant::now() >= deadline {
                // The whole group: what `sh` waits for is in it.
                let _ = kill_process_group(group, Signal::KILL);
                let _ = shell.wait();
                bail!("the command did not end in {limit:?}");
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        // An empty group is gone and its number is free for another process
        // to get: remembered, it would be killed later in place of ours.
        if test_kill_process_group(group).is_ok() {
            self.shells.borrow_mut().push(group);
        }
        if !status.success() {
            bail!("the command failed: {status}");
        }
        Ok(())
    }

    /// Ends whatever the `%shell` lines left running.
    fn end_shells(&self) {
        for group in self.shells.borrow_mut().drain(..) {
            // Nothing left in the group is the usual case and no error.
            let _ = kill_process_group(group, Signal::KILL);
        }
    }

    /// The directory taken down and made again, empty: for the next run in
    /// the same process to start where nothing has been, as a fresh
    /// `enter` would, without giving the turn up.
    pub fn renew(&self) -> Result<()> {
        self.end_shells();
        std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"))?;
        fs::remove_dir_all(&self.path)?;
        fs::create_dir_all(&self.path)?;
        std::env::set_current_dir(&self.path)?;
        Ok(())
    }
}

impl Drop for Workdir {
    fn drop(&mut self) {
        self.end_shells();
        let _ = std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"));
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The snapshots of a run that has no `%grammar` to stop at.
    fn snapshots(script: &str) -> Vec<Snapshot> {
        match run(script).unwrap() {
            Run::Done(snapshots) => snapshots,
            Run::Skipped(name) => panic!("skipped for want of the {name} grammar"),
        }
    }

    #[test]
    fn a_run_answers_each_snapshot_and_names_the_line_of_an_error() {
        let shown = snapshots("size 6 2\ntype \"ab\"\n--- typed\n\n---\n");
        assert_eq!(shown[0].caption, "typed");
        assert_eq!(shown[0].frame, "0|1 ab\n |----^-\n1| …1:3\n");
        assert_eq!(shown[1].frame, "same\n");

        let error = run("type \"a\"\nbogus\n").unwrap_err();
        assert_eq!(error.to_string(), "line 2");
    }

    /// Two runs in one process take turns: each has the directory to itself
    /// while it lasts, and what one writes the other never sees.
    #[test]
    fn runs_in_one_process_do_not_share_the_directory() {
        let first = snapshots("%file a.txt \"a\"\n");
        assert!(first.is_empty());
        assert!(!PathBuf::from("a.txt").exists());

        let workdir = Workdir::enter().unwrap();
        fs::write("b.txt", "b").unwrap();
        assert!(fs::read_dir(".").unwrap().count() == 1);
        drop(workdir);
    }

    #[test]
    fn shell_runs_a_command_in_the_directory() {
        run("%shell printf made > a.txt\n?file a.txt \"made\"\n").unwrap();
    }

    #[test]
    fn a_shell_command_that_fails_is_the_lines_error() {
        let error = run("%shell exit 3\n").unwrap_err();
        assert!(error.to_string().starts_with("line 1"), "{error:#}");
    }

    /// A named pipe with a writer left behind it opens as a file does:
    /// text for the editor that is never on the disk.
    #[test]
    fn what_shell_left_at_a_pipe_is_there_for_the_editor() {
        let shown = snapshots(
            "%shell mkfifo p.txt; (printf piped > p.txt &)\n%start p.txt\nsize 30 2\n---\n",
        );
        assert!(shown[0].frame.contains("piped"), "{}", shown[0].frame);
    }

    /// A command that does not end is ended at the limit, with whatever it
    /// was waiting for: here a writer in the foreground at a pipe that
    /// nobody reads.
    #[test]
    fn a_shell_command_that_does_not_end_is_ended_at_the_limit() {
        let workdir = Workdir::enter().unwrap();
        let error = workdir
            .shell_within(
                "mkfifo p; echo $$ > pid; printf x > p",
                Duration::from_millis(100),
            )
            .unwrap_err();
        assert!(error.to_string().contains("did not end"), "{error:#}");

        // `sh` was collected by the wait, so it is gone at once.
        let shell: i32 = fs::read_to_string("pid").unwrap().trim().parse().unwrap();
        let shell = Pid::from_raw(shell).unwrap();
        assert!(rustix::process::test_kill_process(shell).is_err());
    }

    /// Only a group with something left in it is remembered: the number of
    /// an empty one may be another process's by the time the run ends.
    #[test]
    fn a_shell_group_is_remembered_only_while_it_has_members() {
        let workdir = Workdir::enter().unwrap();
        workdir.shell("printf made > a.txt").unwrap();
        assert!(workdir.shells.borrow().is_empty());

        workdir.shell("(sleep 600 &)").unwrap();
        assert_eq!(workdir.shells.borrow().len(), 1);
    }

    /// What a command left running does not outlive the run: here a writer
    /// at a pipe that nobody ever opens.
    #[test]
    fn what_shell_left_running_ends_with_the_run() {
        let workdir = Workdir::enter().unwrap();
        workdir
            .shell("mkfifo p; (echo late > p &) ; (sleep 600 & echo $! > pid)")
            .unwrap();
        let sleeper: i32 = fs::read_to_string("pid").unwrap().trim().parse().unwrap();
        let sleeper = Pid::from_raw(sleeper).unwrap();
        assert!(rustix::process::test_kill_process(sleeper).is_ok());

        drop(workdir);
        // Killed at once, gone when whoever inherited it has collected it.
        let gone = (0..200).any(|_| {
            std::thread::sleep(std::time::Duration::from_millis(10));
            rustix::process::test_kill_process(sleeper).is_err()
        });
        assert!(gone);
    }

    /// `start` is the command line, read by the same parser; what `stdin`
    /// gave before it is the pipe. Afterwards neither has a place.
    #[test]
    fn start_takes_the_command_line_and_stdin_the_pipe() {
        let shown = snapshots("%stdin \"piped\"\n%start\nsize 36 2\n---\n");
        assert_eq!(
            shown[0].frame,
            "0|1 piped\n |--^---------------------------------\n1|     [stdin] · Text · 1 lines · 1:1\n"
        );

        let shown =
            snapshots("%file a.txt \"filed\"\n%stdin \"piped\"\n%start a.txt\nsize 30 2\n---\n");
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

    /// A `?` line that holds is silent; one that does not is an error that
    /// names the line and says what was found instead.
    #[test]
    fn a_check_passes_silently_and_a_failed_one_names_the_line() {
        let script = concat!(
            "%file a.txt \"kept\"\n",
            "?file a.txt \"kept\"\n",
            "?nopath b.txt\n",
            "type \"x\"\n",
            "key Ctrl+A\n",
            "key Ctrl+C\n",
            "?clipboard \"x\"\n",
            "key Ctrl+Shift+Q\n",
            "type \"a\"\n",
            "?exit 1\n",
        );
        assert!(snapshots(script).is_empty());

        let piped = "%stdin \"in\"\ntype \"x\"\nkey Ctrl+S\nkey Ctrl+Q\n?exit 0\n?stdout \"xin\"\n";
        assert!(snapshots(piped).is_empty());

        for (script, error) in [
            (
                "%file a.txt \"kept\"\n?file a.txt \"lost\"\n",
                "line 2: a.txt holds \"kept\", not \"lost\"",
            ),
            ("?file a.txt \"kept\"\n", "line 1: a.txt: No such file"),
            (
                "%file a.txt \"kept\"\n?nopath a.txt\n",
                "line 2: a.txt is there",
            ),
            ("?nopath .\n", "line 1: . is there"),
            (
                "type \"x\"\n?clipboard \"x\"\n",
                "line 2: the clipboard holds nothing, not \"x\"",
            ),
            (
                "type \"x\"\n?exit 0\n",
                "line 2: the editor is still running",
            ),
            ("key Ctrl+Q\n?exit 1\n", "line 2: exit 0, not 1"),
            ("key Ctrl+Q\n?stdout \"\"\n", "line 2: not a pipeline"),
            (
                "%stdin \"in\"\nkey Ctrl+Q\n?stdout \"x\"\n",
                "line 3: stdout \"in\", not \"x\"",
            ),
        ] {
            let shown = format!("{:#}", run(script).unwrap_err());
            assert!(shown.starts_with(error), "{script:?}: {shown}");
        }
    }

    /// A quit ends the run: there is no screen to snapshot and no editor
    /// to take a line, only `?exit` and `?stdout` to say how it ended.
    #[test]
    fn an_exit_ends_the_run() {
        snapshots("%stdin \"in\"\nkey Ctrl+Shift+Q\n?exit 1\n?stdout \"\"\n");

        for script in [
            "key Ctrl+Q\n--- gone\n",
            "key Ctrl+Q\ntype \"a\"\n",
            "key Ctrl+Q\nwait\n",
            "key Ctrl+Q\nsize 6 2\n",
        ] {
            let error = run(script).unwrap_err().to_string();
            assert!(error.starts_with("line 2: "), "{script}: {error}");
        }

        // The run goes on without an editor: a file is written, and
        // `%start` starts another, which begins as the first did, with the
        // default frame and a whole first snapshot.
        let shown = snapshots(
            "%file a.txt \"kept\"\nsize 6 2\ntype \"x\"\n---\n\nkey Ctrl+Shift+Q\ntype \"a\"\n?exit 1\n%file b.txt \"b\"\n%start a.txt\n---\n",
        );
        // Whole, and the default ten rows: the eight blank ones are listed.
        assert!(
            shown[1].frame.starts_with("0|1 kept\n"),
            "{}",
            shown[1].frame
        );
        assert!(shown[1].frame.contains("\n1-8|\n"), "{}", shown[1].frame);
    }

    /// `%reset` is the same whether the editor runs, has exited or was
    /// never started: afterwards the directory is empty, a `%stdin` given
    /// before is forgotten, and the next line starts an editor as the
    /// first line of a run would.
    #[test]
    fn reset_takes_the_run_back_to_before_anything() {
        let shown = snapshots(
            "%file a.txt \"a\"\n%stdin \"piped\"\ntype \"x\"\n%reset\nsize 60 2\nkey Ctrl+O\ntype \"a.txt\"\nkey Enter\n---\n\nkey Ctrl+Q\n%reset\n%reset\ntype \"y\"\n---\n",
        );
        assert!(shown[0].frame.starts_with("0|1\n"), "{}", shown[0].frame);
        assert!(
            shown[0].frame.contains("cannot read a.txt"),
            "{}",
            shown[0].frame
        );
        // The second editor begins afresh: the default frame, whole.
        assert!(shown[1].frame.starts_with("0|1 y\n"), "{}", shown[1].frame);
        assert!(shown[1].frame.contains("\n1-8|\n"), "{}", shown[1].frame);
    }
}
