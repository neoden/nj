//! What comes before there is an editor and what comes after it: the
//! command line, the buffer the editor starts with, and how the process
//! ends once the editor has exited. In the library rather than in `main.rs`
//! so that a scenario starts and ends the editor the way a launch does.

use std::fs;
use std::io::{ErrorKind, Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

use crate::buffer::Buffer;
use crate::buffer::language::{self, Language};
use crate::editor::{self, Editor, Exit};

/// `nj [--socket PATH] [--language NAME] [FILE]`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args {
    /// The file as it was written, which may be with a place in it:
    /// `FILE:LINE`, `FILE:LINE:COLUMN` or `FILE:LINE-LINE`. Which part is
    /// the name depends on what files there are, and is settled by `start`.
    pub file: Option<String>,
    pub socket: Option<PathBuf>,
    /// The language to edit in, over what the file's name says: a pipe
    /// has no name to say it.
    pub language: Option<&'static Language>,
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
            "--language" => {
                let name = rest.next().context("--language takes a language's name")?;
                let language = language::named(&name)
                    .with_context(|| format!("no language is called `{name}`"))?;
                args.language = Some(language);
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

/// What the editor starts with: the buffer, read already, and the place in
/// it that the file was named with.
pub struct Start {
    buffer: Buffer,
    place: Option<Place>,
}

/// The buffer the editor starts with: the file named, else what is piped
/// in, else nothing. A path wins over a pipe: `nj file < input` edits the
/// file. A file is opened in the language asked for, if any; a pipe and
/// nothing have no name to tell one from, and are plain text unless asked.
pub fn start(args: &Args, pipe: Option<impl Read>) -> Result<Start> {
    if let Some(named) = args.file.as_deref() {
        let (path, place) = target(named);
        let buffer = Buffer::open_or_new(path, args.language)?;
        return Ok(Start { buffer, place });
    }
    let mut buffer = match pipe {
        Some(pipe) => Buffer::from_pipe(pipe)?,
        None => Buffer::default(),
    };
    if let Some(language) = args.language {
        buffer.set_language(language);
    }
    Ok(Start { buffer, place: None })
}

impl Start {
    /// The editor, on a screen of `size`, columns and rows. Sized before
    /// the cursor is put in its place: placing it brings it on screen, and
    /// a view that has no size yet has no screen. The place is then shown
    /// in the middle of the screen, with what is around it on both sides.
    pub fn editor(self, size: (usize, usize)) -> Editor {
        let mut editor = Editor::new(self.buffer);
        editor.resize(size.0, size.1);
        let Some(place) = self.place else {
            return editor;
        };
        match place {
            Place::At { line, column } => editor.go_to(line, column),
            Place::Lines { first, last } => editor.select_lines(first, last),
        }
        editor.center();
        editor
    }
}

/// Where in a file to start, counted from zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    /// One position: `file:42` is column zero of the line, `file:42:7` is
    /// the column a compiler names.
    At { line: usize, column: usize },
    /// Whole lines, both ends included: `file:42-50`.
    Lines { first: usize, last: usize },
}

/// The file an argument names, and the place in it if it names one.
///
/// `a:1:42` can be read three ways: a file of that name, line 42 of `a:1`,
/// column 42 of line 1 of `a`. The file that is there says which, the
/// longest name first. With none of them there it is a new file, under the
/// shortest name.
fn target(named: &str) -> (&str, Option<Place>) {
    if is_there(named) {
        return (named, None);
    }
    let readings = readings(named);
    let found = readings.iter().find(|(path, _)| is_there(path));
    match found.or(readings.last()) {
        Some(&(path, place)) => (path, Some(place)),
        None => (named, None),
    }
}

/// Whether something has this name. What cannot be looked at counts as
/// there, so that opening it says why, rather than the name being read
/// another way.
fn is_there(path: &str) -> bool {
    !matches!(fs::symlink_metadata(path), Err(error) if error.kind() == ErrorKind::NotFound)
}

/// The ways to read a name as a path with a place after it, the longest
/// path first; none when it does not end as a place does. The place is
/// written counting from one: `file:42`, `file:42:7`, `file:42-50`. Which
/// files are there is not looked at here.
fn readings(named: &str) -> Vec<(&str, Place)> {
    // A compiler ends a place with a colon, before what it has to say.
    let named = named.strip_suffix(':').unwrap_or(named);
    let Some((path, last)) = named.rsplit_once(':') else {
        return Vec::new();
    };
    if path.is_empty() {
        return Vec::new();
    }
    let before = path
        .rsplit_once(':')
        .filter(|(shorter, _)| !shorter.is_empty())
        .and_then(|(shorter, line)| Some((shorter, editor::line_number(line)?)));

    if let Some((from, to)) = last.split_once('-') {
        let (Some(from), Some(to)) = (editor::line_number(from), editor::line_number(to)) else {
            return Vec::new();
        };
        let (first, last) = (from.min(to), from.max(to));
        let mut readings = vec![(path, Place::Lines { first, last })];
        // `file:42:7-50` is columns of one line: the cursor goes to the
        // first of them.
        if let Some((shorter, line)) = before {
            readings.push((shorter, Place::At { line, column: first }));
        }
        return readings;
    }

    let Some(last) = editor::line_number(last) else {
        return Vec::new();
    };
    let mut readings = vec![(path, Place::At { line: last, column: 0 })];
    // Two numbers are a line and a column.
    if let Some((shorter, line)) = before {
        readings.push((shorter, Place::At { line, column: last }));
    }
    readings
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

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: usize, column: usize) -> Place {
        Place::At { line, column }
    }

    fn lines(first: usize, last: usize) -> Place {
        Place::Lines { first, last }
    }

    #[test]
    fn a_name_may_end_with_a_place() {
        assert_eq!(readings("main.rs:42"), [("main.rs", at(41, 0))]);
        assert_eq!(readings("main.rs:42-50"), [("main.rs", lines(41, 49))]);
        // Written backwards, it is the same lines.
        assert_eq!(readings("main.rs:50-42"), [("main.rs", lines(41, 49))]);
        // One line as a range is the whole line, not a position in it.
        assert_eq!(readings("main.rs:42-42"), [("main.rs", lines(41, 41))]);
        // A colon further left belongs to the path.
        assert_eq!(readings("a:b:7"), [("a:b", at(6, 0))]);
        // There is no line zero; it is the first one, as in the line number field.
        assert_eq!(readings("main.rs:0"), [("main.rs", at(0, 0))]);
        // Too large to hold is past the end of any file, not a name.
        assert_eq!(
            readings("main.rs:99999999999999999999"),
            [("main.rs", at(usize::MAX - 1, 0))]
        );
    }

    #[test]
    fn two_numbers_are_a_line_of_a_longer_name_or_a_line_and_a_column() {
        assert_eq!(
            readings("main.rs:42:7"),
            [("main.rs:42", at(6, 0)), ("main.rs", at(41, 6))]
        );
        // No more than two numbers are taken from the end.
        assert_eq!(
            readings("a:1:42:7"),
            [("a:1:42", at(6, 0)), ("a:1", at(41, 6))]
        );
        // Columns of a line: the first of them.
        assert_eq!(
            readings("main.rs:42:7-50"),
            [("main.rs:42", lines(6, 49)), ("main.rs", at(41, 6))]
        );
    }

    #[test]
    fn a_colon_may_end_the_place() {
        // As grep and gcc write it, before the line or the message.
        assert_eq!(readings("main.rs:42:"), [("main.rs", at(41, 0))]);
        assert_eq!(
            readings("main.c:42:7:"),
            [("main.c:42", at(6, 0)), ("main.c", at(41, 6))]
        );
        // One colon, not two.
        assert_eq!(readings("main.rs:42::"), []);
    }

    #[test]
    fn anything_else_after_the_colon_is_part_of_the_path() {
        for name in [
            "main.rs",
            "main.rs:",
            "notes:draft",
            "main.rs:42-",
            "main.rs:-50",
            "main.rs:+42",
            "main.rs:1-2-3",
            ":42",
        ] {
            assert_eq!(readings(name), [], "{name}");
        }
    }
}
