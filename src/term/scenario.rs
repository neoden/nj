//! Drives the editor with a scenario and returns what the screen showed
//! (ADR 25). No terminal is opened: events go in where the loop would hand
//! them in, and frames come out as text.
//!
//! One instruction per line; blank lines and `#` comments are skipped.
//!
//!   size W H       the screen, in cells (40×10 until said otherwise)
//!   type TEXT      TEXT, keystroke by keystroke, as it is written
//!   key NAME       one key by its name, `Ctrl+Shift+L`, `Enter`, `F3`
//!   paste TEXT     TEXT as one bracketed paste; `\n` stands for a newline
//!   click X Y [MODS]   the left button going down at column X, row Y;
//!   drag X Y [MODS]    the pointer moving there with the button held;
//!   scroll up|down X Y [MODS]   the wheel. MODS is `Ctrl+Shift` and so on
//!   file NAME TEXT  a file in the scenario's directory, with `\n` as above
//!   clipboard TEXT  what the desktop clipboard holds, as if copied elsewhere
//!   wait           let every background job finish
//!   --- CAPTION    a snapshot: the frame as the screen shows it here. The
//!                  first is whole, each later one is the rows that changed
//!                  since the one before, or `same` when none did. CAPTION
//!                  says what should be seen. In a file the frame follows
//!                  the line, up to a blank line, and is generated.
//!
//! Resizing is `size`: the loop takes the terminal's size before every
//! frame, and a resize event only wakes it up.
//!
//! The scenario runs in a directory of its own under `target`, made for the
//! run and removed after it, so a path typed into a prompt is relative to
//! that and nothing outside is touched. The clipboard is a fake: what the
//! scenario copies or puts there is all it ever holds.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result, bail};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use crate::buffer::Buffer;
use crate::editor::Editor;
use crate::editor::clipboard::Desktop;
use crate::editor::key::Key;

use super::grid::Grid;
use super::{app, dump, input, render};

/// A line of a scenario, as the file is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line<'a> {
    /// The author's: an instruction, a comment or a blank line.
    Text(&'a str),
    /// `---` and its caption: a frame is taken here.
    Snapshot(&'a str),
    /// Generated: a line of the frame under a snapshot, and the blank line
    /// that ends it.
    Frame(&'a str),
}

/// Tells the author's lines from the frames, which is position, not shape:
/// a frame is whatever follows a snapshot line up to and including the
/// first blank line.
pub fn lines(script: &str) -> impl Iterator<Item = Line<'_>> {
    let mut in_frame = false;
    script.lines().map(move |line| {
        let line = line.trim_end();
        if in_frame {
            in_frame = !line.is_empty();
            return Line::Frame(line);
        }
        match line.strip_prefix("---") {
            Some(caption) => {
                in_frame = true;
                Line::Snapshot(caption.trim())
            }
            None => Line::Text(line),
        }
    })
}

/// What a snapshot showed, with the caption it was taken under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub caption: String,
    pub frame: String,
}

/// Runs `script` against a fresh editor and returns its snapshots, in
/// order. An error names the line it happened on.
pub fn run(script: &str) -> Result<Vec<Snapshot>> {
    let _workdir = Workdir::enter()?;
    let mut run = Run::new();
    for (index, line) in lines(script).enumerate() {
        match line {
            Line::Text(line) if line.is_empty() || line.starts_with('#') => {}
            Line::Text(line) => {
                let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
                run.step(verb, rest)
                    .with_context(|| format!("line {}", index + 1))?;
            }
            Line::Snapshot(caption) => run.snapshot(caption),
            Line::Frame(_) => {}
        }
    }
    Ok(run.snapshots)
}

/// The fake desktop clipboard. The scenario shares it with the editor to
/// set what is on it from outside.
#[derive(Clone, Default)]
struct Pasteboard(Rc<RefCell<Option<String>>>);

impl Desktop for Pasteboard {
    fn read(&mut self) -> Option<String> {
        self.0.borrow().clone()
    }

    fn write(&mut self, text: &str) {
        *self.0.borrow_mut() = Some(text.to_owned());
    }
}

/// The scenario's directory: made on entry, left on exit, taken down with
/// everything in it when the run ends, however it ends.
struct Workdir(PathBuf);

impl Workdir {
    fn enter() -> Result<Self> {
        let runs = concat!(env!("CARGO_MANIFEST_DIR"), "/target/scenarios");
        let path = PathBuf::from(runs).join(std::process::id().to_string());
        fs::create_dir_all(&path)?;
        std::env::set_current_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for Workdir {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(env!("CARGO_MANIFEST_DIR"));
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Run {
    editor: Editor,
    size: (usize, usize),
    previous: Option<Grid>,
    pasteboard: Pasteboard,
    snapshots: Vec<Snapshot>,
}

impl Run {
    fn new() -> Self {
        let pasteboard = Pasteboard::default();
        let mut editor = Editor::new(Buffer::default()).with_desktop(pasteboard.clone());
        let size = (40, 10);
        editor.resize(size.0, size.1);
        Self {
            editor,
            size,
            previous: None,
            pasteboard,
            snapshots: Vec::new(),
        }
    }

    fn step(&mut self, verb: &str, rest: &str) -> Result<()> {
        match verb {
            "size" => {
                let (width, height) = rest
                    .split_once(' ')
                    .context("size takes a width and a height")?;
                self.size = (width.parse()?, height.parse()?);
                self.editor.resize(self.size.0, self.size.1);
            }
            "type" => {
                for character in rest.chars() {
                    let key = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
                    app::handle(Event::Key(key), &mut self.editor);
                }
            }
            "key" => {
                let key: Key = rest.parse()?;
                app::handle(input::event(key), &mut self.editor);
            }
            "paste" => app::handle(Event::Paste(unescaped(rest)), &mut self.editor),
            "click" => self.mouse(MouseEventKind::Down(MouseButton::Left), rest)?,
            "drag" => self.mouse(MouseEventKind::Drag(MouseButton::Left), rest)?,
            "scroll" => {
                let (direction, rest) = rest.split_once(' ').context("scroll takes up or down")?;
                let kind = match direction {
                    "up" => MouseEventKind::ScrollUp,
                    "down" => MouseEventKind::ScrollDown,
                    other => bail!("scroll goes up or down, not `{other}`"),
                };
                self.mouse(kind, rest)?;
            }
            "file" => {
                let (name, text) = rest.split_once(' ').unwrap_or((rest, ""));
                fs::write(name, unescaped(text))?;
            }
            "clipboard" => *self.pasteboard.0.borrow_mut() = Some(unescaped(rest)),
            "wait" => self.editor.finish_jobs(),
            other => bail!("unknown instruction `{other}`"),
        }
        Ok(())
    }

    fn snapshot(&mut self, caption: &str) {
        // As the loop does before every frame (`app::run`): the screen is
        // divided afresh, since a panel may have opened or closed.
        self.editor.resize(self.size.0, self.size.1);
        let mut grid = Grid::new(self.size.0, self.size.1);
        render::compose(&mut grid, &self.editor);
        let frame = match dump::dump(&grid, self.previous.as_ref()) {
            text if text.is_empty() => "same\n".to_owned(),
            text => text,
        };
        self.snapshots.push(Snapshot {
            caption: caption.to_owned(),
            frame,
        });
        self.previous = Some(grid);
    }

    /// `X Y` and, after them, modifiers if any.
    fn mouse(&mut self, kind: MouseEventKind, rest: &str) -> Result<()> {
        let mut words = rest.split_whitespace();
        let column = words.next().context("a column")?.parse()?;
        let row = words.next().context("a row")?.parse()?;
        let modifiers = match words.next() {
            Some(names) => mods(names)?,
            None => KeyModifiers::NONE,
        };
        let event = MouseEvent {
            kind,
            column,
            row,
            modifiers,
        };
        app::handle(Event::Mouse(event), &mut self.editor);
        Ok(())
    }
}

fn mods(names: &str) -> Result<KeyModifiers> {
    let mut modifiers = KeyModifiers::NONE;
    for name in names.split('+') {
        modifiers |= match name.to_ascii_lowercase().as_str() {
            "ctrl" => KeyModifiers::CONTROL,
            "alt" => KeyModifiers::ALT,
            "shift" => KeyModifiers::SHIFT,
            other => bail!("unknown modifier `{other}`"),
        };
    }
    Ok(modifiers)
}

/// A line of the scenario can hold several lines of text.
fn unescaped(text: &str) -> String {
    text.replace("\\n", "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_runs_from_its_snapshot_to_a_blank_line() {
        let script = "type a\n--- one letter\n0|1 a\n |::^\n\nkey Esc\n---\n";
        let read: Vec<Line> = lines(script).collect();
        assert_eq!(
            read,
            [
                Line::Text("type a"),
                Line::Snapshot("one letter"),
                Line::Frame("0|1 a"),
                Line::Frame(" |::^"),
                Line::Frame(""),
                Line::Text("key Esc"),
                Line::Snapshot(""),
            ]
        );
    }
}
