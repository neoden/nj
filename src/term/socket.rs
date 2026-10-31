//! A Unix socket that takes lines of the scenario
//! language and answers each. The threads here own the socket and nothing
//! else; a line goes to the loop as a message and the answer comes back on
//! a channel, so the editor is touched by its own thread only.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;

use anyhow::{Context, Result, bail};
use crossterm::event::Event;

use super::app;
use super::launch::Outcome;
#[cfg(debug_assertions)]
use super::sandbox;
use super::scenario::{self, Instruction};
#[cfg(debug_assertions)]
use super::screen::Screen;
use crate::editor::Editor;
use crate::editor::job::{Message, Remote};

/// Listens at `path` and serves each connection on a thread of its own.
/// A socket left there by an earlier run is replaced; anything else at the
/// path is somebody's file, and the editor refuses rather than take its
/// place. The socket's file goes when the listener is dropped.
pub fn listen(path: &Path, queue: Sender<Message>) -> Result<Listener> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if !metadata.file_type().is_socket() {
            bail!("{} exists and is not a socket", path.display());
        }
        fs::remove_file(path)
            .with_context(|| format!("cannot replace the socket at {}", path.display()))?;
    }
    let listener =
        UnixListener::bind(path).with_context(|| format!("cannot listen at {}", path.display()))?;
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                break;
            };
            let queue = queue.clone();
            thread::spawn(move || serve(stream, queue));
        }
    });
    Ok(Listener(path.to_owned()))
}

/// One connection: a line in, an answer out, in order. The answer is lines
/// ended by a blank one, whatever the line asked, so the client reads to
/// the blank line and need not remember what it sent.
fn serve(stream: UnixStream, queue: Sender<Message>) {
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else {
            break;
        };
        let (reply, answer) = mpsc::channel();
        if queue.send(Message::Remote(Remote { line, reply })).is_err() {
            break;
        }
        // The loop is gone when there is no answer: nothing left to serve.
        let Ok(answer) = answer.recv() else {
            break;
        };
        // The answer ends with a newline of its own; this adds the blank line.
        if writeln!(out, "{answer}").is_err() {
            break;
        }
    }
}

/// The socket's file, removed when the listener goes.
#[derive(Debug)]
pub struct Listener(PathBuf);

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// What the socket takes: the editor's own lines of the scenario language
/// — input, as the keyboard's would be; `wait`, for its own jobs; `size`,
/// the frame to draw — and `screen`, a verb of the socket alone, answered
/// with the whole screen as `screen::Screen` writes it, whole every time.
/// The `%`, `?` and `---` lines are the run's, carried out by whoever
/// drives the editor, and are refused here.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Input(Vec<Event>),
    Wait,
    Size(usize, usize),
    Screen,
}

impl Command {
    /// The one command that is not a line of the language, as it is sent.
    pub const SCREEN: &'static str = "screen";

    /// `None` for a blank line or a comment, which say nothing to do.
    pub fn parse(line: &str) -> Result<Option<Self>> {
        if line.trim_end() == Self::SCREEN {
            return Ok(Some(Command::Screen));
        }
        // The prefix says so before the verb does.
        if scenario::belongs_to_run(line) {
            bail!("a `%`, `?` or `---` line is the run's and is not taken over the socket");
        }
        Ok(match scenario::parse(line)? {
            None => None,
            Some(Instruction::Input(events)) => Some(Command::Input(events)),
            Some(Instruction::Wait) => Some(Command::Wait),
            Some(Instruction::Size(width, height)) => Some(Command::Size(width, height)),
            Some(other) => unreachable!("the run's lines are refused by their prefix: {other:?}"),
        })
    }
}

/// What the loop does with a command: carries it out on the editor and
/// says what came of it, each line of the answer ended by a newline.
#[derive(Default)]
pub struct Server {
    /// The frame to draw instead of the whole terminal, once `size` said.
    size: Option<(usize, usize)>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    /// The frame to draw in a terminal of this size: the whole of it, or
    /// what `size` said, drawn in the top-left corner.
    pub fn frame_size(&self, terminal: (usize, usize)) -> (usize, usize) {
        match self.size {
            Some((width, height)) => (terminal.0.min(width), terminal.1.min(height)),
            None => terminal,
        }
    }

    /// `terminal` is the terminal's size: a frame it cannot show whole is
    /// refused rather than shown in part.
    pub fn answer(&mut self, editor: &mut Editor, line: &str, terminal: (usize, usize)) -> String {
        match Command::parse(line) {
            Err(error) => format!("error: {error:#}\n"),
            Ok(None) => "ok\n".to_owned(),
            // The screen as it is, whole, with nothing remembered from the
            // time before. Refused when the terminal has shrunk under
            // `size`: then the screen and the frame would disagree, and it
            // is not shown in part.
            #[cfg(debug_assertions)]
            Ok(Some(Command::Screen)) => {
                if let Some(error) = self.size.and_then(|wanted| too_small(terminal, wanted)) {
                    return error;
                }
                let grid = sandbox::grid(editor, self.frame_size(terminal));
                Screen::of(&grid).write()
            }
            #[cfg(not(debug_assertions))]
            Ok(Some(Command::Screen)) => {
                "error: the screen is read in the debug build only\n".to_owned()
            }
            Ok(Some(Command::Input(events))) => {
                if let Err(error) = app::handle_all(events, editor) {
                    return format!("error: {error:#}\n");
                }
                // The editor leaving is the answer: the loop ends after this
                // and the connection goes with it.
                if editor.exit().is_some() {
                    return match Outcome::of(editor) {
                        Ok(outcome) => scenario::outcome(&outcome),
                        Err(error) => format!("error: {error:#}\n"),
                    };
                }
                "ok\n".to_owned()
            }
            Ok(Some(Command::Wait)) => {
                editor.finish_jobs();
                "ok\n".to_owned()
            }
            Ok(Some(Command::Size(width, height))) => {
                if let Some(error) = too_small(terminal, (width, height)) {
                    return error;
                }
                self.size = Some((width, height));
                "ok\n".to_owned()
            }
        }
    }
}

/// The error for a frame the terminal cannot show whole, if it cannot.
fn too_small(terminal: (usize, usize), frame: (usize, usize)) -> Option<String> {
    let (columns, rows) = terminal;
    let (width, height) = frame;
    (columns < width || rows < height).then(|| {
        format!(
            "error: the terminal is {columns}×{rows}, too small for a frame of {width}×{height}: enlarge it\n"
        )
    })
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::buffer::Buffer;

    fn scratch(name: &str) -> PathBuf {
        let directory = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target/sockets"));
        fs::create_dir_all(&directory).unwrap();
        directory.join(format!("{name}-{}", std::process::id()))
    }

    /// The server takes what is about the editor and refuses the rest. A
    /// `size` the terminal cannot show whole is refused rather than shown
    /// in part; headless tests have no terminal and are not here.
    #[test]
    fn the_server_takes_input_wait_and_size_and_refuses_the_rest() {
        let mut server = Server::new();
        let mut editor = Editor::new(Buffer::default());
        let terminal = (80, 24);
        assert_eq!(server.frame_size(terminal), terminal);
        // As the loop does before the first line arrives.
        editor.resize(terminal.0, terminal.1);

        assert_eq!(server.answer(&mut editor, "type \"ab\"", terminal), "ok\n");
        assert_eq!(server.answer(&mut editor, "wait", terminal), "ok\n");
        assert_eq!(server.answer(&mut editor, "size 40 6", terminal), "ok\n");
        assert_eq!(server.frame_size(terminal), (40, 6));
        assert_eq!(server.frame_size((30, 30)), (30, 6));

        let answer = server.answer(&mut editor, "size 100 5", terminal);
        assert!(
            answer.starts_with("error: the terminal is 80×24, too small"),
            "{answer}"
        );
        assert_eq!(server.frame_size(terminal), (40, 6));

        // The screen comes whole every time, whatever was asked before; a
        // terminal that shrank since `size` is refused as `size` would
        // have been.
        let answer = server.answer(&mut editor, "screen", terminal);
        assert!(
            answer.starts_with("screen 40 6\n0 text|1 ab\n0 mark|"),
            "{answer}"
        );
        assert!(answer.ends_with("end\n"), "{answer}");
        assert_eq!(server.answer(&mut editor, "screen", terminal), answer);
        let answer = server.answer(&mut editor, "screen", (30, 30));
        assert!(
            answer.starts_with("error: the terminal is 30×30"),
            "{answer}"
        );
        // The socket's own vocabulary, read before anything is done.
        assert_eq!(Command::parse("screen").unwrap(), Some(Command::Screen));
        assert_eq!(Command::parse("wait").unwrap(), Some(Command::Wait));
        assert_eq!(
            Command::parse("size 10 4").unwrap(),
            Some(Command::Size(10, 4))
        );
        assert_eq!(Command::parse("# a comment").unwrap(), None);
        assert!(Command::parse("bogus").is_err());
        // A `%`, `?` or `---` line is refused by its prefix, whatever
        // follows it.
        for line in [
            "--- typed",
            "---ink",
            "%file a.txt \"a\"",
            "%clipboard \"c\"",
            "%stdin \"s\"",
            "%start",
            "%bogus",
            "?exit 0",
            "?bogus",
        ] {
            let answer = server.answer(&mut editor, line, terminal);
            assert!(
                answer.starts_with("error: a `%`, `?` or `---` line"),
                "{line}: {answer}"
            );
        }
        // The text is unsaved, so quitting asks first; the answer that
        // quits is answered with the outcome. Input on the same line after
        // the exit is refused, as the headless run refuses it.
        assert_eq!(server.answer(&mut editor, "key Ctrl+Q", terminal), "ok\n");
        assert_eq!(
            server.answer(&mut editor, "type \"qx\"", terminal),
            "error: input after the editor has exited\n"
        );
        let mut editor = Editor::new(Buffer::default());
        editor.resize(terminal.0, terminal.1);
        assert_eq!(server.answer(&mut editor, "type \"ab\"", terminal), "ok\n");
        assert_eq!(server.answer(&mut editor, "key Ctrl+Q", terminal), "ok\n");
        assert_eq!(
            server.answer(&mut editor, "type \"q\"", terminal),
            "exit 0\n"
        );
    }

    #[test]
    fn a_file_at_the_path_is_left_alone_and_refused() {
        let path = scratch("notes.txt");
        fs::write(&path, "notes").unwrap();
        let (queue, _) = mpsc::channel();

        let error = listen(&path, queue).unwrap_err().to_string();
        assert!(error.ends_with("exists and is not a socket"), "{error}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "notes");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_stale_socket_is_replaced_and_the_file_goes_with_the_listener() {
        let path = scratch("stale.sock");
        let (queue, _) = mpsc::channel();
        // Bound and never dropped: what a run that was killed leaves behind.
        let stale = UnixListener::bind(&path).unwrap();

        let listener = listen(&path, queue).unwrap();
        assert!(fs::symlink_metadata(&path).unwrap().file_type().is_socket());
        drop(listener);
        assert!(!path.exists());
        drop(stale);
    }
}
