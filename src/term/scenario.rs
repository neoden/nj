//! The scenario language: what a line says, read but not done.
//! The doing is elsewhere — `sandbox` runs a scenario against a headless
//! editor or carries its lines out in a live one, and `socket` takes the
//! lines from outside.
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
//!   wait           let every background job finish
//!   %file NAME "TEXT"   a file in the scenario's directory
//!   %shell COMMAND    COMMAND, the rest of the line, run by `sh` in the
//!                  scenario's directory and waited for, five seconds at
//!                  most; what it leaves running behind it is ended with
//!                  the run
//!   %clipboard "TEXT"   what the desktop clipboard holds, as if copied elsewhere
//!   %stdin "TEXT"  what is piped into the editor; a terminal otherwise
//!   %start ARGS    the editor, started with ARGS as its command line;
//!                  without this line it starts, as `nj` alone
//!                  would, on the first line that is not a `%` line
//!   %reset         the run as it was before anything: an editor still
//!                  running is ended, the directory emptied
//!   %grammar NAME  the scenario needs the grammar of the language NAME,
//!                  which a build may leave out; without it the run stops
//!                  there, and the headless run says it was skipped
//!   %allocated NAME     from here the bytes the editor allocates are
//!                  counted under NAME; said again, counted afresh
//!   ?exit N        the editor has exited, with code N
//!   ?stdout "TEXT" and, a pipeline, handed TEXT downstream
//!   ?file NAME "TEXT"   the file in the scenario's directory holds TEXT
//!   ?nopath NAME   nothing is there, neither a file nor a directory
//!   ?clipboard "TEXT"   the desktop clipboard holds TEXT
//!   ?allocated NAME SIZE   the editor has allocated no more than SIZE
//!                  since `%allocated NAME`: bytes, or `4K`, `2M`, `1G`
//!   --- CAPTION    a snapshot: the text on the screen and the marks under
//!                  it. The first is whole, each later one is the rows that
//!                  changed since the snapshot before, or `same` when none
//!                  did. CAPTION says what should be seen. In a file the
//!                  frame follows the line, up to a blank line, and is
//!                  generated.
//!   ---ink CAPTION the same for the inks, the colours of the text: what
//!                  changed since the last `---ink`. Two snapshots in a
//!                  row when both matter.
//!
//! A line beginning with `%` sets the run up around the editor, and one
//! beginning with `?` checks what the run can see and the screen cannot:
//! how the editor ended, what it handed on, what it wrote, what it put on
//! the clipboard. A snapshot is the run's too: it asks the editor for the
//! screen and works out what changed from the one before. None of these
//! are the editor's: the headless run and the sandbox binary carry them
//! out, the editor's socket refuses them unread. A `?` line that does not
//! hold is an error naming the line, like any other. The other lines are
//! the editor's own.
//!
//! Once the editor has exited there is no frame, so a snapshot is an
//! error, as are the editor's own lines, there being no editor to take
//! them. `?exit` and `?stdout` say what became of it; the `%` lines go
//! on, and `%start` starts the next editor.
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
use super::launch::{self, Args, Outcome};
use super::screen::Layer;
use crate::buffer::language::{self, Language};
use crate::editor::key::Key;

/// A line of a scenario, as the file is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Line<'a> {
    /// The author's: an instruction, a comment or a blank line.
    Text(&'a str),
    /// `---` or `---ink` and its caption: a frame of that layer is taken
    /// here.
    Snapshot(Layer, &'a str),
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
        match snapshot(line) {
            Some((layer, caption)) => {
                in_frame = true;
                Line::Snapshot(layer, caption)
            }
            None => Line::Text(line),
        }
    })
}

/// The layer and caption of a snapshot line, if this is one: `---` for the
/// marks, `---ink` for the inks, the caption after a space.
fn snapshot(line: &str) -> Option<(Layer, &str)> {
    let rest = line.strip_prefix("---")?;
    match rest.strip_prefix("ink") {
        Some(caption) if caption.is_empty() || caption.starts_with(' ') => {
            Some((Layer::Inks, caption.trim()))
        }
        _ => Some((Layer::Marks, rest.trim())),
    }
}

/// A snapshot line as it is written: the inverse of `snapshot`, for the
/// test that rewrites a scenario and the replay that shows one.
pub fn snapshot_line(layer: Layer, caption: &str) -> String {
    let marker = match layer {
        Layer::Marks => "---",
        Layer::Inks => "---ink",
    };
    match caption {
        "" => marker.to_owned(),
        caption => format!("{marker} {caption}"),
    }
}

/// One instruction, read but not yet carried out. Reading is separate from
/// doing because the same lines come from a file in a test and from a
/// socket in a live editor, and only the doing differs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    /// What the terminal would have sent: the events of a key, of typed
    /// text one keystroke each, of a paste or of a mouse action.
    Input(Vec<Event>),
    Size(usize, usize),
    /// A file in the run's directory, with its text.
    File(String, String),
    /// A command for the shell, run in the run's directory.
    Shell(String),
    /// The start of a count of the editor's allocations, under this name.
    Allocated(String),
    /// Since the count of this name began the editor has allocated no more
    /// than this many bytes. A `?` line, but not an `Expect`: only the
    /// headless run can tell, from the counts it keeps.
    AllocatedWithin(String, u64),
    /// What the desktop clipboard holds.
    Clipboard(String),
    /// What is piped into the editor, to be read when it starts.
    Stdin(String),
    /// The editor, started with this command line.
    Start(Args),
    /// The run taken back to before anything: the editor gone, the
    /// directory empty.
    Reset,
    /// The grammar of this language, which the scenario's frames assume.
    Grammar(&'static Language),
    Wait,
    /// A frame of this layer to take, under this caption.
    Snapshot(Layer, String),
    /// A `?` line: what the run is to check.
    Expect(Expect),
}

/// What a `?` line claims. Checked by the run, since none of it is on the
/// screen: the editor's end and its standard output are seen only from
/// outside, and a file only from the directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expect {
    /// The editor has exited, with this code.
    Exit(u8),
    /// The editor has exited and, a pipeline, handed this downstream.
    Stdout(String),
    /// The file in the run's directory holds this text.
    File(String, String),
    /// Nothing is at this name in the run's directory: a save that was
    /// declined or refused made no file.
    NoPath(String),
    /// The desktop clipboard holds this text.
    Clipboard(String),
}

/// Whether `line` is the run's rather than the editor's: a `%` line sets
/// the run up, a `?` line checks it, a `---` line takes a frame of what
/// the run remembers. The socket refuses all three without reading further.
pub fn belongs_to_run(line: &str) -> bool {
    line.starts_with('%') || line.starts_with('?') || line.starts_with("---")
}

/// Reads one line of the language. `None` for a blank line or a comment,
/// which say nothing to do.
pub fn parse(line: &str) -> Result<Option<Instruction>> {
    let line = line.trim_end();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    if let Some((layer, caption)) = snapshot(line) {
        return Ok(Some(Instruction::Snapshot(layer, caption.to_owned())));
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
        "wait" => Instruction::Wait,
        "%file" => {
            let (name, text) = rest
                .split_once(' ')
                .context("%file takes a name and its text")?;
            Instruction::File(name.to_owned(), quoted(text)?)
        }
        "%shell" if rest.is_empty() => bail!("%shell takes a command"),
        "%shell" => Instruction::Shell(rest.to_owned()),
        "%clipboard" => Instruction::Clipboard(quoted(rest)?),
        "%stdin" => Instruction::Stdin(quoted(rest)?),
        "%start" => Instruction::Start(launch::args(rest.split_whitespace().map(str::to_owned))?),
        "%reset" => Instruction::Reset,
        // A misspelt name is an error, or it would quietly stop the run.
        "%grammar" => Instruction::Grammar(
            language::by_name(rest).with_context(|| format!("no language is called `{rest}`"))?,
        ),
        "?exit" => Instruction::Expect(Expect::Exit(
            rest.parse().context("?exit takes a code, 0 to 255")?,
        )),
        "?stdout" => Instruction::Expect(Expect::Stdout(quoted(rest)?)),
        "?file" => {
            let (name, text) = rest
                .split_once(' ')
                .context("?file takes a name and its text")?;
            Instruction::Expect(Expect::File(name.to_owned(), quoted(text)?))
        }
        "?nopath" => match rest.split_whitespace().collect::<Vec<_>>()[..] {
            [name] => Instruction::Expect(Expect::NoPath(name.to_owned())),
            _ => bail!("?nopath takes one name"),
        },
        "?clipboard" => Instruction::Expect(Expect::Clipboard(quoted(rest)?)),
        "%allocated" => match rest.split_whitespace().collect::<Vec<_>>()[..] {
            [name] => Instruction::Allocated(name.to_owned()),
            _ => bail!("%allocated takes one name"),
        },
        "?allocated" => match rest.split_whitespace().collect::<Vec<_>>()[..] {
            [name, size] => Instruction::AllocatedWithin(name.to_owned(), bytes(size)?),
            _ => bail!("?allocated takes a name and a size"),
        },
        other => bail!("unknown instruction `{other}`"),
    };
    Ok(Some(instruction))
}

/// A size: bytes when it is a bare number, and `K`, `M` or `G` after it
/// for units of 1024.
fn bytes(size: &str) -> Result<u64> {
    let (number, unit) = match size.strip_suffix(['K', 'M', 'G']) {
        Some(number) => (
            number,
            1 << (10 * (1 + "KMG".find(&size[number.len()..]).unwrap())),
        ),
        None => (size, 1),
    };
    // A number too large for the unit is as much not a size as a word is.
    number
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(unit))
        .with_context(|| format!("a size is a number of bytes, or `4K`, `2M`, `1G`: {size}"))
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

/// What the socket answers to the line that made the editor exit, in place
/// of `ok`: the code, and for a pipeline what went downstream.
/// The sandbox binary reads the `exit` off it to know the editor is gone.
pub fn outcome(outcome: &Outcome) -> String {
    let mut out = format!("exit {}\n", outcome.code);
    if let Some(stdout) = &outcome.stdout {
        out.push_str(&format!("stdout {}\n", quote(stdout)));
    }
    out
}

/// `text` as the language writes it: in double quotes, with the escapes
/// that `quoted` reads back.
pub fn quote(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
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
    fn quote_writes_what_quoted_reads() {
        for text in ["", "a b ", "x\ny\"z\\", "a\tb"] {
            assert_eq!(quoted(&quote(text)).unwrap(), text, "{text:?}");
        }
        assert_eq!(quote("a\nb"), r#""a\nb""#);
    }

    #[test]
    fn an_outcome_is_the_code_and_what_a_pipeline_hands_on() {
        let quit = Outcome {
            code: 0,
            stdout: None,
        };
        assert_eq!(outcome(&quit), "exit 0\n");
        let piped = Outcome {
            code: 1,
            stdout: Some("x\n".to_owned()),
        };
        assert_eq!(outcome(&piped), "exit 1\nstdout \"x\\n\"\n");
    }

    #[test]
    fn a_count_of_allocations_is_named_and_its_limit_is_a_size() {
        assert_eq!(
            parse("%allocated step").unwrap(),
            Some(Instruction::Allocated("step".to_owned()))
        );
        let limit = |line: &str| match parse(line).unwrap() {
            Some(Instruction::AllocatedWithin(_, limit)) => limit,
            other => panic!("{other:?}"),
        };
        assert_eq!(limit("?allocated step 300"), 300);
        assert_eq!(limit("?allocated step 4K"), 4096);
        assert_eq!(limit("?allocated step 2M"), 2 << 20);
        assert_eq!(limit("?allocated step 1G"), 1 << 30);
        for bad in [
            "%allocated",
            "%allocated a b",
            "?allocated step",
            "?allocated step 4k",
            "?allocated step K",
            "?allocated step 1.5M",
            "?allocated step 17179869184G",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_shell_line_is_its_command_as_written() {
        assert_eq!(
            parse(r#"%shell mkfifo p; (printf "a b" > p &)"#).unwrap(),
            Some(Instruction::Shell(
                r#"mkfifo p; (printf "a b" > p &)"#.to_owned()
            ))
        );
        assert!(parse("%shell").is_err());
    }

    #[test]
    fn a_check_is_the_runs_and_names_what_it_claims() {
        assert_eq!(
            parse("?exit 1").unwrap(),
            Some(Instruction::Expect(Expect::Exit(1)))
        );
        assert_eq!(
            parse(r#"?stdout "a\nb""#).unwrap(),
            Some(Instruction::Expect(Expect::Stdout("a\nb".to_owned())))
        );
        assert_eq!(
            parse(r#"?file a.txt "kept""#).unwrap(),
            Some(Instruction::Expect(Expect::File(
                "a.txt".to_owned(),
                "kept".to_owned()
            )))
        );
        assert_eq!(
            parse(r#"?clipboard "cut""#).unwrap(),
            Some(Instruction::Expect(Expect::Clipboard("cut".to_owned())))
        );
        assert_eq!(
            parse("?nopath a.txt").unwrap(),
            Some(Instruction::Expect(Expect::NoPath("a.txt".to_owned())))
        );
        for bad in [
            "?exit",
            "?exit x",
            "?exit 256",
            "?stdout bare",
            "?file a.txt",
            "?nopath",
            "?nopath a b",
            "?clipboard",
            "?saved",
        ] {
            assert!(parse(bad).is_err(), "{bad}");
        }
        assert!(belongs_to_run("?exit 0"));
        assert!(belongs_to_run("%reset"));
        assert!(!belongs_to_run("key Esc"));
    }

    #[test]
    fn a_grammar_is_asked_for_by_the_languages_name() {
        let rust = Some(Instruction::Grammar(language::by_name("Rust").unwrap()));
        assert_eq!(parse("%grammar Rust").unwrap(), rust);
        assert_eq!(parse("%grammar rust").unwrap(), rust);
        for bad in ["%grammar", "%grammar Rsut"] {
            assert!(parse(bad).is_err(), "{bad}");
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
        let script =
            "type a\n--- one letter\n0|1 a\n |--^\n\nkey Esc\n---\n\n---ink coloured\n\n---inky\n";
        let read: Vec<Line> = lines(script).collect();
        assert_eq!(
            read,
            [
                Line::Text("type a"),
                Line::Snapshot(Layer::Marks, "one letter"),
                Line::Frame("0|1 a"),
                Line::Frame(" |--^"),
                Line::Frame(""),
                Line::Text("key Esc"),
                Line::Snapshot(Layer::Marks, ""),
                Line::Frame(""),
                Line::Snapshot(Layer::Inks, "coloured"),
                Line::Frame(""),
                // Not the word `ink`, so a caption.
                Line::Snapshot(Layer::Marks, "inky"),
            ]
        );
        assert_eq!(snapshot_line(Layer::Marks, "one letter"), "--- one letter");
        assert_eq!(snapshot_line(Layer::Inks, ""), "---ink");
    }
}
