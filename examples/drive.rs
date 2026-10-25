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
//!   wait           let every background job finish
//!   screen         print the frame: the first one whole, each later one
//!                  as the rows that changed since the one before, or
//!                  `same` when none did
//!
//! Resizing is `size`: the loop takes the terminal's size before every
//! frame, and a resize event only wakes it up.

use std::io::{self, BufRead};

use anyhow::{Context, Result, bail};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use ned::buffer::Buffer;
use ned::editor::Editor;
use ned::editor::key::Key;
use ned::term::grid::Grid;
use ned::term::{app, dump, input, render};

fn main() -> Result<()> {
    let mut editor = Editor::new(Buffer::default());
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
        step(verb, rest, &mut editor, &mut size, &mut previous)
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
        "paste" => {
            let text = rest.replace("\\n", "\n");
            app::handle(Event::Paste(text), editor);
        }
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
