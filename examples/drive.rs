//! Drives the editor with a scenario read from standard input and prints
//! what the screen shows (ADR 25). No terminal is opened: events go in
//! where the loop would hand them in, and frames come out as text.
//!
//! One instruction per line; blank lines and `#` comments are skipped.
//!
//!   size W H       the screen, in cells (40×10 until said otherwise)
//!   type TEXT      TEXT, keystroke by keystroke, as it is written
//!   key NAME       one key by its name, `Ctrl+Shift+L`, `Enter`, `F3`
//!   screen         print the frame

use std::io::{self, BufRead};

use anyhow::{Context, Result, bail};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use ned::buffer::Buffer;
use ned::editor::Editor;
use ned::editor::key::Key;
use ned::term::grid::Grid;
use ned::term::{app, dump, input, render};

fn main() -> Result<()> {
    let mut editor = Editor::new(Buffer::default());
    let mut size = (40, 10);
    editor.resize(size.0, size.1);

    for (index, line) in io::stdin().lock().lines().enumerate() {
        let line = line?;
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
        step(verb, rest, &mut editor, &mut size).with_context(|| format!("line {}", index + 1))?;
    }
    Ok(())
}

fn step(verb: &str, rest: &str, editor: &mut Editor, size: &mut (usize, usize)) -> Result<()> {
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
        "screen" => {
            let mut grid = Grid::new(size.0, size.1);
            render::compose(&mut grid, editor);
            print!("{}", dump::dump(&grid));
        }
        other => bail!("unknown instruction `{other}`"),
    }
    Ok(())
}
