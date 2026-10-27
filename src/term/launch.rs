//! What comes before there is an editor and what comes after it: the
//! command line, the buffer the editor starts with, and how the process
//! ends once the editor has exited. In the library rather than in `main.rs`
//! so that a scenario starts and ends the editor the way a launch does
//! (ADR 28).

use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::buffer::Buffer;
use crate::editor::{Editor, Exit};

/// `ned [--socket PATH] [FILE]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args {
    pub file: Option<String>,
    pub socket: Option<PathBuf>,
}

/// Reads the command line, given without the program's own name.
pub fn args(arguments: impl IntoIterator<Item = String>) -> Result<Args> {
    let mut args = Args::default();
    let mut rest = arguments.into_iter();
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--socket" => {
                let path = rest.next().context("--socket takes a path")?;
                args.socket = Some(PathBuf::from(path));
            }
            flag if flag.starts_with("--") => bail!("unknown option `{flag}`"),
            path => match args.file {
                None => args.file = Some(path.to_owned()),
                Some(_) => bail!("one file at a time"),
            },
        }
    }
    Ok(args)
}

/// The buffer the editor starts with: the file named, else what is piped
/// in, else nothing. A path wins over a pipe: `ned file < input` edits the
/// file.
pub fn buffer(file: Option<&str>, pipe: Option<impl Read>) -> Result<Buffer> {
    match (file, pipe) {
        (Some(path), _) => Buffer::open_or_new(path),
        (None, Some(pipe)) => Buffer::from_pipe(pipe),
        (None, None) => Ok(Buffer::default()),
    }
}

/// How the process ends once the editor has exited: its exit code, with
/// what a pipeline downstream receives written to `downstream`. An abort
/// hands nothing on and fails, so that a pipeline or a commit waiting on
/// the editor stops there.
pub fn conclude(editor: &Editor, mut downstream: &mut dyn Write) -> Result<u8> {
    if editor.exit() == Some(Exit::Abort) {
        return Ok(1);
    }
    editor.buffer().finish(&mut downstream)?;
    Ok(0)
}

/// The end of the process, held rather than carried out: for a scenario to
/// show, or a socket to answer with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub code: u8,
    /// What the downstream receives, for a pipeline; `None` where there is
    /// none to receive anything.
    pub stdout: Option<String>,
}

impl Outcome {
    pub fn of(editor: &Editor) -> Result<Self> {
        let mut downstream = Vec::new();
        let code = conclude(editor, &mut downstream)?;
        let stdout = editor
            .buffer()
            .is_pipeline()
            .then(|| String::from_utf8_lossy(&downstream).into_owned());
        Ok(Self { code, stdout })
    }
}
