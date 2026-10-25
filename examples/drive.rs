//! Drives the editor with a scenario read from standard input and prints
//! what the screen shows (ADR 25). No terminal is opened: events go in
//! where the loop would hand them in, and frames come out as text.
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
//!   screen         print the frame: the first one whole, each later one
//!                  as the rows that changed since the one before, or
//!                  `same` when none did
//!
//! Resizing is `size`: the loop takes the terminal's size before every
//! frame, and a resize event only wakes it up.
//!
//! The scenario runs in a directory of its own, made for the run and removed
//! after it, so a path typed into a prompt is relative to that and nothing
//! outside is touched. The clipboard is a fake: what the scenario copies or
//! puts there is all it ever holds.

use std::cell::RefCell;
use std::fs;
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result, bail};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use ned::buffer::Buffer;
use ned::editor::Editor;
use ned::editor::clipboard::Desktop;
use ned::editor::key::Key;
use ned::term::grid::Grid;
use ned::term::{app, dump, input, render};

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
        let path = std::env::temp_dir().join(format!("ned-drive-{}", std::process::id()));
        fs::create_dir(&path)?;
        std::env::set_current_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for Workdir {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(std::env::temp_dir());
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() -> Result<()> {
    let _workdir = Workdir::enter()?;
    let pasteboard = Pasteboard::default();
    let mut editor = Editor::new(Buffer::default()).with_desktop(pasteboard.clone());
    let mut size = (40, 10);
    editor.resize(size.0, size.1);
    let mut previous: Option<Grid> = None;

    for (index, line) in io::stdin().lock().lines().enumerate() {
        let line = line?;
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
        step(
            verb,
            rest,
            &mut editor,
            &mut size,
            &mut previous,
            &pasteboard,
        )
        .with_context(|| format!("line {}", index + 1))?;
    }
    Ok(())
}

fn step(
    verb: &str,
    rest: &str,
    editor: &mut Editor,
    size: &mut (usize, usize),
    previous: &mut Option<Grid>,
    pasteboard: &Pasteboard,
) -> Result<()> {
    match verb {
        "size" => {
            let (width, height) = rest
                .split_once(' ')
                .context("size takes a width and a height")?;
            *size = (width.parse()?, height.parse()?);
            editor.resize(size.0, size.1);
        }
        "type" => {
            for character in rest.chars() {
                let key = KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE);
                app::handle(Event::Key(key), editor);
            }
        }
        "key" => {
            let key: Key = rest.parse()?;
            app::handle(input::event(key), editor);
        }
        "paste" => app::handle(Event::Paste(unescaped(rest)), editor),
        "click" => mouse(editor, MouseEventKind::Down(MouseButton::Left), rest)?,
        "drag" => mouse(editor, MouseEventKind::Drag(MouseButton::Left), rest)?,
        "scroll" => {
            let (direction, rest) = rest.split_once(' ').context("scroll takes up or down")?;
            let kind = match direction {
                "up" => MouseEventKind::ScrollUp,
                "down" => MouseEventKind::ScrollDown,
                other => bail!("scroll goes up or down, not `{other}`"),
            };
            mouse(editor, kind, rest)?;
        }
        "file" => {
            let (name, text) = rest.split_once(' ').unwrap_or((rest, ""));
            fs::write(name, unescaped(text))?;
        }
        "clipboard" => *pasteboard.0.borrow_mut() = Some(unescaped(rest)),
        "wait" => editor.finish_jobs(),
        "screen" => {
            // As the loop does before every frame (`app::run`): the screen
            // is divided afresh, since a panel may have opened or closed.
            editor.resize(size.0, size.1);
            let mut grid = Grid::new(size.0, size.1);
            render::compose(&mut grid, editor);
            if previous.is_some() {
                println!();
            }
            match dump::dump(&grid, previous.as_ref()) {
                text if text.is_empty() => println!("same"),
                text => print!("{text}"),
            }
            *previous = Some(grid);
        }
        other => bail!("unknown instruction `{other}`"),
    }
    Ok(())
}

/// `X Y` and, after them, modifiers if any.
fn mouse(editor: &mut Editor, kind: MouseEventKind, rest: &str) -> Result<()> {
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
    app::handle(Event::Mouse(event), editor);
    Ok(())
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
