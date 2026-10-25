//! The socket of ADR 26: a Unix socket that takes lines of the scenario
//! language and answers each. The threads here own the socket and nothing
//! else; a line goes to the loop as a message and the answer comes back on
//! a channel, so the editor is touched by its own thread only (ADR 21).

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread;

use anyhow::{Context, Result, bail};

use super::app;
#[cfg(debug_assertions)]
use super::sandbox::Sandbox;
use super::scenario::{self, Instruction};
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

/// What the loop does with a line: carries it out on the editor and says
/// what came of it, each line of the answer ended by a newline. Input is
/// taken as the keyboard's would be; the rest of the language is taken in
/// the sandbox (ADR 27) and refused outside it. The server also knows how
/// big a frame to draw, since the sandbox may cap it.
#[derive(Default)]
pub struct Server {
    #[cfg(debug_assertions)]
    sandbox: Option<Sandbox>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(debug_assertions)]
    pub fn sandboxed(sandbox: Sandbox) -> Self {
        Self {
            sandbox: Some(sandbox),
        }
    }

    /// The frame to draw in a terminal of this size.
    pub fn frame_size(&self, terminal: (usize, usize)) -> (usize, usize) {
        #[cfg(debug_assertions)]
        if let Some(sandbox) = &self.sandbox {
            return sandbox.frame_size(terminal);
        }
        terminal
    }

    /// `terminal` is the terminal's size, which the sandbox holds a frame
    /// against.
    pub fn answer(&mut self, editor: &mut Editor, line: &str, terminal: (usize, usize)) -> String {
        match scenario::parse(line) {
            Err(error) => format!("error: {error:#}\n"),
            Ok(None) => "ok\n".to_owned(),
            Ok(Some(Instruction::Input(events))) => {
                for event in events {
                    app::handle(event, editor);
                }
                "ok\n".to_owned()
            }
            Ok(Some(instruction)) => self.sandboxed_answer(editor, instruction, terminal),
        }
    }

    #[cfg(debug_assertions)]
    fn sandboxed_answer(
        &mut self,
        editor: &mut Editor,
        instruction: Instruction,
        terminal: (usize, usize),
    ) -> String {
        let Some(sandbox) = &mut self.sandbox else {
            return refused();
        };
        match sandbox.apply(editor, instruction, Some(terminal)) {
            Ok(answer) => answer,
            Err(error) => format!("error: {error:#}\n"),
        }
    }

    #[cfg(not(debug_assertions))]
    fn sandboxed_answer(&mut self, _: &mut Editor, _: Instruction, _: (usize, usize)) -> String {
        refused()
    }
}

fn refused() -> String {
    "error: only input is taken outside the sandbox\n".to_owned()
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let directory = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target/sockets"));
        fs::create_dir_all(&directory).unwrap();
        directory.join(format!("{name}-{}", std::process::id()))
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
