//! The loop and the dispatch around the editor: what the process does with
//! events, as opposed to what the editor does with commands.

use std::io::Write;
use std::thread;

use anyhow::Result;
use crossterm::event::{self, Event, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};

use super::render::Renderer;
use crate::editor::command::{Command, Edit, Select};
use crate::editor::job::Message;
use crate::editor::key::Keystroke;
use crate::editor::{keymap, Editor};

const MOUSE_SCROLL_LINES: isize = 3;

/// One queue, one thread that owns the editor (ADR 21). The terminal is read
/// on a thread of its own, because `event::read` blocks and the queue is
/// what the loop has to wait on.
pub fn run(editor: &mut Editor, out: &mut impl Write) -> Result<()> {
    let mut renderer = Renderer::new();

    // Started only now: the terminal has been set up, and its answer to the
    // keyboard-protocol query has been read by whoever asked.
    let input = editor.sender();
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            if input.send(Message::Input(event)).is_err() {
                return;
            }
        }
    });

    loop {
        let (width, height) = crossterm::terminal::size()?;
        let size = (width as usize, height as usize);
        editor.resize(size.0, size.1);
        renderer.draw(out, editor, size)?;

        // Whatever has arrived by the time the loop wakes is handled before
        // the next frame, input first: a keystroke never queues behind the
        // reports that happened to come with it.
        let Some(first) = editor.wait() else {
            return Ok(());
        };
        let mut inputs = Vec::new();
        let mut reports = Vec::new();
        let mut sort = |message| match message {
            Message::Input(event) => inputs.push(event),
            Message::Report(report) => reports.push(report),
        };
        sort(first);
        while let Some(message) = editor.pending() {
            sort(message);
        }

        for event in inputs {
            handle(event, editor);
            if editor.should_quit() {
                return Ok(());
            }
        }
        for report in reports {
            editor.receive(report);
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
