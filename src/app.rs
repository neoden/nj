//! The loop and the dispatch around the editor: what the process does with
//! events, as opposed to what the editor does with commands.

use std::io::Write;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use crate::command::{Command, Edit, Select};
use crate::editor::Editor;
use crate::key::Keystroke;
use crate::keymap;
use crate::render;

const MOUSE_SCROLL_LINES: isize = 3;

pub fn run(editor: &mut Editor, out: &mut impl Write) -> Result<()> {
    loop {
        let (width, height) = crossterm::terminal::size()?;
        editor.resize(width as usize, height as usize);

        let (left, right) = status(editor);
        render::draw(out, editor, &left, &right)?;

        // A panel still reading its candidates must not let the loop go back
        // to sleep, or the list would stop filling until the next keystroke.
        if editor.is_loading() && !event::poll(Duration::ZERO)? {
            editor.load_more();
            continue;
        }

        handle(event::read()?, editor);
        // Whatever else already arrived is processed before drawing again, so
        // a burst of input costs one frame rather than one frame per event.
        while !editor.should_quit() && event::poll(Duration::ZERO)? {
            handle(event::read()?, editor);
        }

        if editor.should_quit() {
            return Ok(());
        }
    }
}

/// Turns an event into a command and hands it over. Public so that a test
/// can drive the editor with events and never open a terminal.
pub fn handle(event: Event, editor: &mut Editor) {
    let command = match event {
        Event::Key(event) => {
            let stroke = Keystroke::from_event(&event);
            stroke.and_then(|stroke| keymap::resolve(stroke, editor))
        }
        Event::Mouse(mouse) => mouse_command(mouse),
        // Bracketed paste: the terminal hands over the whole text at once
        // instead of replaying it as keystrokes.
        Event::Paste(text) => Some(Command::Edit(Edit::Insert(text))),
        _ => None,
    };

    if let Some(command) = command {
        editor.execute(command);
    }
}

/// The mouse speaks in screen cells; the editor knows which view they fall in.
fn mouse_command(mouse: MouseEvent) -> Option<Command> {
    let (row, column) = (mouse.row as usize, mouse.column as usize);
    Some(match mouse.kind {
        MouseEventKind::ScrollUp => Command::Scroll(-MOUSE_SCROLL_LINES),
        MouseEventKind::ScrollDown => Command::Scroll(MOUSE_SCROLL_LINES),
        MouseEventKind::Down(MouseButton::Left) => Command::Select(Select::Click {
            row,
            column,
            add: mouse.modifiers.contains(KeyModifiers::ALT),
        }),
        MouseEventKind::Drag(MouseButton::Left) => Command::Select(Select::Drag { row, column }),
        _ => return None,
    })
}

/// The right side is the document, always in the same place. The left side is
/// whatever is going on right now, so the eye only goes there when something
/// is happening.
fn status(editor: &Editor) -> (String, String) {
    let (view, buffer) = (editor.view(), editor.buffer());
    let line = buffer.text.line_of(view.selections.primary().head);
    let column = editor.cursor_column();

    let right = format!(
        "{}{} · {} · {} lines · {}:{}",
        buffer.name(),
        if buffer.is_modified() { "*" } else { "" },
        buffer.language.name,
        buffer.text.line_count(),
        line + 1,
        column + 1,
    );

    let left = match editor.message() {
        Some(message) => message.to_owned(),
        None => {
            let cursors = view.selections.len();
            [
                editor.panel_status(),
                match cursors {
                    1 => String::new(),
                    count => format!("{count} cursors"),
                },
            ]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
        }
    };

    (format!(" {left}"), right)
}
