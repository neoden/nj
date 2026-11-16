//! The headless run of a scenario, for the tests and the drive example:
//! an editor under the conditions the scenario assumes — a fake
//! clipboard, a directory of its own under `target`, a frame of the size
//! the scenario says, 40×10 until it says — started, driven and ended by
//! the scenario's lines. Test machinery, so debug builds only. A
//! live editor gets the same conditions from the sandbox binary, which
//! runs it as a child process and shares `Workdir` and `Frames`
//! from here.

use std::cell::RefCell;
use std::collections::HashMap;
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
use super::scenario::{self, Expect, Growth, Instruction, Line, Measure};
use super::screen::{Frames, Layer, Screen};
use super::{app, growth, meter, render};
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
    /// The counts that `%measure` lines began, by the name each gave.
    counts: HashMap<String, Count>,
}

/// What the editor had used at a `%measure` line, and the most memory it
/// has held since.
struct Count {
    /// The bytes it had asked for by then, freed or not.
    requested: u64,
    /// The bytes it held then.
    held: i64,
    /// The most bytes it has held at any moment since.
    highest: i64,
    /// The processor time it had taken by then.
    spent: Duration,
    /// The part of it the editor's own thread had taken.
    blocked: Duration,
}

impl Sandbox {
    /// Brings the most each count has held up to now. `meter` keeps one
    /// such mark for the process and starts it afresh when asked, so every
    /// count that is open takes it before another begins or one is read.
    fn note_highest(&mut self) {
        let highest = meter::highest();
        for count in self.counts.values_mut() {
            count.highest = count.highest.max(highest);
        }
    }

    fn new() -> Self {
        Self {
            editor: None,
            counts: HashMap::new(),
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

    /// Starts the editor as `args` would from the shell:`stdin`
    /// is the pipe, the sandbox keeps the other end of the clipboard. The
    /// socket is the process's, not the editor's, and has no meaning here.
    /// An editor started after another begins as the first did, with the
    /// default frame and no frame before it, as a new process would.
    fn start(&mut self, args: Args, stdin: Option<String>) -> Result<()> {
        if args.socket.is_some() {
            bail!("--socket: a run has no socket");
        }
        let start = launch::start(&args, stdin.map(Cursor::new))?;
        self.size = DEFAULT_SIZE;
        self.frames = Frames::default();
        self.editor = Some(start.editor(self.size).with_desktop(self.pasteboard.clone()));
        Ok(())
    }

    /// One of the editor's own lines: input, `wait` or `size`. The outcome
    /// once the line has made the editor exit.
    ///
    /// The only way from the run to a running editor, and so the one place
    /// where its work is metered. The frame is composed after the line as
    /// the loop composes it after every event (`app::run`) and dropped:
    /// what a line costs is handling it and drawing what came of it,
    /// whether or not the scenario looks at the screen next.
    fn send(&mut self, instruction: Instruction) -> Result<Option<Outcome>> {
        meter::editor(|| {
            match instruction {
                Instruction::Input(events) => {
                    let editor = self.editor();
                    app::handle_all(events, editor)?;
                    if editor.exit().is_some() {
                        return Ok(Some(Outcome::of(editor)?));
                    }
                }
                Instruction::Wait => self.editor().finish_jobs(),
                // The editor is told with the frame, below.
                Instruction::Size(width, height) => self.size = (width, height),
                other => unreachable!("not the editor's own: {other:?}"),
            }
            let size = self.size;
            grid(self.editor(), size);
            Ok(None)
        })
    }

    /// The frame of `layer`: what changed on the screen since the last
    /// snapshot of that layer.
    fn frame(&mut self, layer: Layer) -> String {
        let editor = self
            .editor
            .as_mut()
            .expect("the run starts the editor first");
        // The run looking at the screen, and not metered: the frame that a
        // line costs was composed when the line was sent.
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

/// One measure that a `?` line about a count read in one pass. `?memory`
/// reads three.
struct Reading {
    /// The line of the `?`, from 1.
    line: usize,
    name: String,
    measure: Measure,
    /// How fast the line lets the cost grow.
    growth: Growth,
    /// The count after `over`, when the cost is what is over that
    /// count's.
    over: Option<String>,
    /// Bytes or nanoseconds.
    cost: u64,
}

/// How many times the scenario is run for each value of `%for`. The least
/// cost of them is the value's: the editor does the same work every time,
/// and whatever a pass took over the least was the machine's, a cold cache
/// or a core shared with another process. How far the passes are apart is
/// the noise of the value, which `growth` allows for.
const PASSES: usize = 5;

/// Runs `script` against a fresh headless editor. An error names the line
/// it happened on.
///
/// A scenario with `%for` is run for each value in turn, and its `?`
/// lines about a count are checked after the last: each
/// compares what one pair measured over all the values.
pub fn run(script: &str) -> Result<Run> {
    // This thread reads the scenario. From here its allocations are counted
    // only inside `meter::editor`, which `start` and `send` are run in.
    let _harness = meter::harness();
    let workdir = Workdir::enter()?;
    let Some((variable, values)) = values(script)? else {
        return pass(script, &workdir, None, &mut Vec::new());
    };

    // For each value, the readings of each of its passes: the same lines in
    // the same order every pass.
    let mut measured: Vec<Vec<Vec<Reading>>> = Vec::new();
    for (written, _) in &values {
        let variable = (variable.as_str(), written.as_str());
        let mut passes = Vec::new();
        for _ in 0..PASSES {
            workdir.renew()?;
            let mut readings = Vec::new();
            if let Run::Skipped(name) = pass(script, &workdir, Some(variable), &mut readings)? {
                return Ok(Run::Skipped(name));
            }
            passes.push(readings);
        }
        measured.push(passes);
    }

    // Every line that fails, not the first: they are measured already, and
    // which of them fail together says where the cost is.
    let mut failed = Vec::new();
    for (index, reading) in measured[0][0].iter().enumerate() {
        let points: Vec<growth::Point> = values
            .iter()
            .zip(&measured)
            .map(|((_, value), passes)| {
                let mut costs: Vec<u64> = passes.iter().map(|pass| pass[index].cost).collect();
                costs.sort();
                growth::Point {
                    value: *value as f64,
                    cost: costs[0] as f64,
                    // Half of the passes were within this of the least.
                    noise: (costs[costs.len() / 2] - costs[0]) as f64,
                }
            })
            .collect();
        let fit = growth::fit(&points);
        if !fit.shape.within(reading.growth) {
            let amount = |amount: f64| match reading.measure {
                Measure::Time | Measure::Blocked => {
                    format!("{:?}", Duration::from_nanos(amount as u64))
                }
                _ => format!("{amount} bytes"),
            };
            let over = match &reading.over {
                Some(other) => format!(" over {other}"),
                None => String::new(),
            };
            let mut said = format!(
                "line {}: {} of {}{over} is {} in {variable}, not {}",
                reading.line,
                reading.measure.name(),
                reading.name,
                fit.shape.name(),
                reading.growth.name()
            );
            for ((written, _), point) in values.iter().zip(&points) {
                said.push_str(&format!(
                    "\n  {variable}={written}  {}, within {} from pass to pass",
                    amount(point.cost),
                    amount(point.noise)
                ));
            }
            let off: Vec<String> = fit
                .off
                .iter()
                .map(|(shape, off)| format!("{} {:.1}%", shape.name(), off * 100.0))
                .collect();
            said.push_str(&format!(
                "\n  the furthest point from each formula: {}",
                off.join(", ")
            ));
            failed.push(said);
        }
    }
    if !failed.is_empty() {
        bail!(failed.join("\n"));
    }
    // A scenario with `%for` has no snapshots.
    Ok(Run::Done(Vec::new()))
}

/// The values of a `%for`, each as it is written and as a number.
type Values = Vec<(String, u64)>;

/// The variable and the values of the scenario's `%for`, if it has one.
fn values(script: &str) -> Result<Option<(String, Values)>> {
    let mut found = None;
    for (index, line) in scenario::lines(script).enumerate() {
        let Line::Text(text) = line else { continue };
        // A line that cannot be read is the error of the pass that gets
        // to it.
        if let Ok(Some(Instruction::For(variable, values))) = scenario::parse(text) {
            if found.is_some() {
                bail!("line {}: a scenario has one `%for`", index + 1);
            }
            found = Some((variable, values));
        }
    }
    Ok(found)
}

/// Runs `script` once, from an empty directory to its last line. `value`
/// is the variable of `%for` and its value in this pass, for the `%shell`
/// commands. What the `?` lines about a count measured is added to
/// `readings`.
fn pass(
    script: &str,
    workdir: &Workdir,
    value: Option<(&str, &str)>,
    readings: &mut Vec<Reading>,
) -> Result<Run> {
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
                workdir.shell(&command, value).with_context(at)?;
                continue;
            }
            Instruction::Clipboard(text) => {
                sandbox.pasteboard.set(text);
                continue;
            }
            // Read by `run` before the first pass.
            Instruction::For(..) => continue,
            Instruction::Count(name) => {
                let requested = meter::allocated()
                    .context("nothing counts allocations in this binary")
                    .with_context(at)?;
                sandbox.note_highest();
                let held = meter::held();
                let count = Count {
                    requested,
                    held,
                    highest: held,
                    spent: meter::spent(),
                    blocked: meter::blocked(),
                };
                sandbox.counts.insert(name, count);
                continue;
            }
            Instruction::Grows(measures, name, growth, over) => {
                sandbox.note_highest();
                let since = sandbox
                    .counts
                    .get(&name)
                    .with_context(|| format!("no `%measure {name}` before this"))
                    .with_context(at)?;
                if value.is_none() {
                    bail!(
                        "{}: a growth is over the values of a `%for`, and the scenario has none",
                        at()
                    );
                }
                for &measure in measures {
                    let cost = match measure {
                        // The name is stored only when the allocator is
                        // installed, so the count is there now too.
                        Measure::Requested => meter::allocated().unwrap_or(0) - since.requested,
                        // Less held than at the start is nothing retained.
                        Measure::Retained => (meter::held() - since.held).max(0) as u64,
                        Measure::Peak => (since.highest - since.held) as u64,
                        Measure::Time => (meter::spent() - since.spent).as_nanos() as u64,
                        Measure::Blocked => {
                            (meter::blocked() - since.blocked).as_nanos() as u64
                        }
                    };
                    // What the other count's line read is taken off, in
                    // this pass, where the two were measured one after
                    // the other. Not what that count has come to by now:
                    // it goes on counting, this count's work with it. A
                    // cost below the other's is nothing over it.
                    let cost = match &over {
                        None => cost,
                        Some(other) => {
                            let base = readings
                                .iter()
                                .rev()
                                .find(|read| read.name == *other && read.measure == measure)
                                .with_context(|| {
                                    format!("no `?` line has read this of `{other}` before")
                                })
                                .with_context(at)?;
                            cost.saturating_sub(base.cost)
                        }
                    };
                    readings.push(Reading {
                        line: index + 1,
                        name: name.clone(),
                        measure,
                        growth,
                        over: over.clone(),
                        cost,
                    });
                }
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
            let stdin = stdin.take();
            meter::editor(|| sandbox.start(args, stdin)).with_context(at)?;
            stage = Stage::Running;
            match rest {
                Some(rest) => instruction = rest,
                None => continue,
            }
        }

        match instruction {
            Instruction::Input(_) | Instruction::Wait | Instruction::Size(..) => {
                if let Some(outcome) = sandbox.send(instruction).with_context(at)? {
                    stage = Stage::Exited(outcome);
                }
            }
            // One frame cannot be the screen of every pass.
            Instruction::Snapshot(..) if value.is_some() => {
                bail!("{}: a scenario with `%for` has no snapshots", at())
            }
            Instruction::Snapshot(layer, caption) => snapshots.push(Snapshot {
                frame: sandbox.frame(layer),
                caption,
            }),
            Instruction::Stdin(_) | Instruction::Start(_) => {
                bail!("{}: the editor is already running", at())
            }
            Instruction::File(..)
            | Instruction::Shell(_)
            | Instruction::For(..)
            | Instruction::Count(_)
            | Instruction::Grows(..)
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
    /// errors go where the run's own do. `variable` is a name and a value
    /// put into its environment: the value of `%for` in this pass.
    pub fn shell(&self, command: &str, variable: Option<(&str, &str)>) -> Result<()> {
        self.shell_within(command, variable, SHELL_LIMIT)
    }

    /// `shell` with the limit given: the tests do not wait five seconds.
    fn shell_within(
        &self,
        command: &str,
        variable: Option<(&str, &str)>,
        limit: Duration,
    ) -> Result<()> {
        let mut shell = Command::new("sh")
            .args(["-c", command])
            .envs(variable)
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
                None,
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
        workdir.shell("printf made > a.txt", None).unwrap();
        assert!(workdir.shells.borrow().is_empty());

        workdir.shell("(sleep 600 &)", None).unwrap();
        assert_eq!(workdir.shells.borrow().len(), 1);
    }

    /// Every pass starts in an empty directory and finds its own value in
    /// the shell's environment: a file left by the pass before would fail
    /// the `?nopath`, a value other than the three the `test`.
    #[test]
    fn for_runs_the_scenario_with_each_value_in_the_shells_environment() {
        run(concat!(
            "%for SIZE 1K 2K 4K\n",
            "?nopath seen\n",
            "%shell test $SIZE = 1K -o $SIZE = 2K -o $SIZE = 4K\n",
            "%shell touch seen\n",
        ))
        .unwrap();
    }

    #[test]
    fn a_scenario_with_for_takes_no_snapshots() {
        let error = run("%for SIZE 1K 2K 4K\ntype \"a\"\n---\n").unwrap_err();
        assert!(error.to_string().starts_with("line 3"), "{error:#}");
    }

    /// The text opened is the value's size, and copying all of it
    /// allocates as much: a cost that grows as the value, which is over
    /// `constant` and within `linear`. The error has the line, the power
    /// and the cost at each value.
    ///
    /// Only a growth this plain is tested here. The other tests of this
    /// binary run on their own threads meanwhile: what they allocate is
    /// counted with the editor's, and their time is the process's too. A
    /// megabyte copied stands out of that in what is requested and in the
    /// peak; a finer growth would not, and neither does what is retained,
    /// which the other threads take up and down by megabytes. Those are in
    /// the scenarios, which run alone.
    #[test]
    fn a_cost_that_grows_faster_than_the_line_says_fails_it() {
        let copy = |growth: &str| {
            format!(
                concat!(
                    "%for SIZE 10K 100K 1M\n",
                    "%shell head -c $SIZE /dev/zero | tr '\\0' x > a.txt\n",
                    "%start a.txt\n",
                    "%measure copy\n",
                    "key Ctrl+A\n",
                    "key Ctrl+C\n",
                    "?requested copy {0}\n",
                    "?peak copy {0}\n",
                ),
                growth
            )
        };
        let error = run(&copy("constant")).unwrap_err();
        let said = error.to_string();
        assert!(
            said.starts_with(
                "line 7: the memory requested of copy is linear in SIZE, not constant"
            ),
            "{said}"
        );
        assert!(said.contains("\n  SIZE=1M  "), "{said}");
        // Every failed line is in the error.
        assert!(
            said.contains("\nline 8: the peak of memory of copy is"),
            "{said}"
        );

        run(&copy("linear")).unwrap();
    }

    #[test]
    fn a_growth_needs_a_for_and_a_count_begun() {
        let error = run("%measure a\ntype \"a\"\n?memory a constant\n").unwrap_err();
        assert!(error.to_string().starts_with("line 3"), "{error:#}");

        let error = run("%for SIZE 1K 2K 4K\ntype \"a\"\n?time never constant\n").unwrap_err();
        assert!(error.to_string().starts_with("line 3"), "{error:#}");

        let error = run("%for SIZE 1K 2K 4K\n%for SIZE 1K 2K 4K\n").unwrap_err();
        assert!(error.to_string().starts_with("line 2"), "{error:#}");
    }

    /// What a command left running does not outlive the run: here a writer
    /// at a pipe that nobody ever opens.
    #[test]
    fn what_shell_left_running_ends_with_the_run() {
        let workdir = Workdir::enter().unwrap();
        workdir
            .shell(
                "mkfifo p; (echo late > p &) ; (sleep 600 & echo $! > pid)",
                None,
            )
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
