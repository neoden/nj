//! The scenario language (ADR 25): what a line says, read but not done.
//! The doing is elsewhere — `sandbox` runs a scenario against a headless
//! editor or carries its lines out in a live one, and `socket` takes the
//! lines from outside (ADR 26).
//!
//! One instruction per line; blank lines and `#` comments are skipped.
//!
//!   size W H       the screen, in cells (40×10 until said otherwise)
//!   type "TEXT"    TEXT, keystroke by keystroke
//!   key NAME       one key by its name, `Ctrl+Shift+L`, `Enter`, `F3`
//!   paste "TEXT"   TEXT as one bracketed paste
//!   click X Y [MODS]   the left button going down at column X, row Y;
//!   drag X Y [MODS]    the pointer moving there with the button held;
//!   scroll up|down X Y [MODS]   the wheel. MODS is `Ctrl+Shift` and so on
//!   file NAME "TEXT"   a file in the scenario's directory
//!   clipboard "TEXT"   what the desktop clipboard holds, as if copied elsewhere
//!   wait           let every background job finish
//!   --- CAPTION    a snapshot: the frame as the screen shows it here. The
//!                  first is whole, each later one is the rows that changed
//!                  since the one before, or `same` when none did. CAPTION
//!                  says what should be seen. In a file the frame follows
//!                  the line, up to a blank line, and is generated.
//!
//! Text is written in double quotes, so that a space at either end is
//! visible and a file name is told from its contents; inside them `\n` is a
//! newline, `\t` a tab, `\"` a quote and `\\` a backslash. Names are bare.
//! `type` is keystrokes and takes no control characters: Tab and Enter are
//! keys, and a newline in a text is `paste`.
//!
//! Resizing is `size`: the loop takes the terminal's size before every
//! frame, and a resize event only wakes it up.
//!
//! The scenario runs in a directory of its own under `target`, made for the
//! run and removed after it, so a path typed into a prompt is relative to
//! that and nothing outside is touched. The clipboard is a fake: what the
//! scenario copies or puts there is all it ever holds.

use anyhow::{Context, Result, bail};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

use super::input;
use crate::editor::key::Key;

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
        match snapshot_caption(line) {
            Some(caption) => {
                in_frame = true;
                Line::Snapshot(caption)
            }
            None => Line::Text(line),
        }
    })
}

/// The caption of a snapshot line, if this is one.
fn snapshot_caption(line: &str) -> Option<&str> {
    line.strip_prefix("---").map(str::trim)
}

/// One instruction, read but not yet carried out. Reading is separate from
/// doing because the same lines come from a file in a test and from a
/// socket in a live editor (ADR 26), and only the doing differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    /// What the terminal would have sent: the events of a key, of typed
    /// text one keystroke each, of a paste or of a mouse action.
    Input(Vec<Event>),
    Size(usize, usize),
    /// A file in the run's directory, with its text.
    File(String, String),
    /// What the desktop clipboard holds.
    Clipboard(String),
    Wait,
    /// A frame to take, under this caption.
    Snapshot(String),
}

/// Reads one line of the language. `None` for a blank line or a comment,
/// which say nothing to do.
pub fn parse(line: &str) -> Result<Option<Instruction>> {
    let line = line.trim_end();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    if let Some(caption) = snapshot_caption(line) {
        return Ok(Some(Instruction::Snapshot(caption.to_owned())));
    }
    let (verb, rest) = line.split_once(' ').unwrap_or((line, ""));
    let instruction = match verb {
        "size" => {
            let (width, height) = rest
                .split_once(' ')
                .context("size takes a width and a height")?;
            Instruction::Size(width.parse()?, height.parse()?)
        }
        "type" => {
            let text = quoted(rest)?;
            // A keyboard yields no control characters as text: Tab and
            // Enter arrive as keys of their own, and so they are typed
            // here, or the scenario would describe an event that never
            // happens.
            if let Some(control) = text.chars().find(|c| c.is_control()) {
                bail!("`type` takes text, not {control:?}: use `key Tab`, `key Enter` or `paste`");
            }
            Instruction::Input(
                text.chars()
                    .map(|c| Event::Key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)))
                    .collect(),
            )
        }
        "key" => Instruction::Input(vec![input::event(rest.parse::<Key>()?)]),
        "paste" => Instruction::Input(vec![Event::Paste(quoted(rest)?)]),
        "click" => mouse(MouseEventKind::Down(MouseButton::Left), rest)?,
        "drag" => mouse(MouseEventKind::Drag(MouseButton::Left), rest)?,
        "scroll" => {
            let (direction, rest) = rest.split_once(' ').context("scroll takes up or down")?;
            let kind = match direction {
                "up" => MouseEventKind::ScrollUp,
                "down" => MouseEventKind::ScrollDown,
                other => bail!("scroll goes up or down, not `{other}`"),
            };
            mouse(kind, rest)?
        }
        "file" => {
            let (name, text) = rest
                .split_once(' ')
                .context("file takes a name and its text")?;
            Instruction::File(name.to_owned(), quoted(text)?)
        }
        "clipboard" => Instruction::Clipboard(quoted(rest)?),
        "wait" => Instruction::Wait,
        other => bail!("unknown instruction `{other}`"),
    };
    Ok(Some(instruction))
}

/// `X Y` and, after them, modifiers if any.
fn mouse(kind: MouseEventKind, rest: &str) -> Result<Instruction> {
    let mut words = rest.split_whitespace();
    let column = words.next().context("a column")?.parse()?;
    let row = words.next().context("a row")?.parse()?;
    let modifiers = match words.next() {
        Some(names) => mods(names)?,
        None => KeyModifiers::NONE,
    };
    Ok(Instruction::Input(vec![Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers,
    })]))
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

/// The text between double quotes, with `\n`, `\t`, `\"` and `\\` read as
/// a newline, a tab, a quote and a backslash. Nothing may follow the
/// closing quote.
fn quoted(argument: &str) -> Result<String> {
    let Some(inside) = argument.strip_prefix('"') else {
        bail!("text goes in double quotes: {argument}");
    };
    let mut text = String::new();
    let mut chars = inside.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => match chars.as_str() {
                "" => return Ok(text),
                rest => bail!("nothing may follow the closing quote: {rest}"),
            },
            '\\' => text.push(match chars.next() {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('"') => '"',
                Some('\\') => '\\',
                other => bail!("unknown escape `\\{}`", other.unwrap_or(' ')),
            }),
            c => text.push(c),
        }
    }
    bail!("the closing quote is missing: {argument}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_comes_quoted_and_escaped() {
        assert_eq!(quoted(r#""a b ""#).unwrap(), "a b ");
        assert_eq!(quoted(r#""x\ny\"z\\""#).unwrap(), "x\ny\"z\\");
        assert_eq!(quoted(r#""a\tb""#).unwrap(), "a\tb");
        assert_eq!(quoted(r#""""#).unwrap(), "");
        for bad in ["bare", r#""open"#, r#""a" b"#, r#""\q""#] {
            assert!(quoted(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn type_refuses_what_a_keyboard_cannot_type() {
        assert!(parse(r#"type "a\nb""#).is_err());
        assert!(parse(r#"type "a\tb""#).is_err());
        assert!(parse(r#"paste "a\tb""#).is_ok());
    }

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
