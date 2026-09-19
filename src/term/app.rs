//! The loop and the dispatch around the editor: what the process does with
//! events, as opposed to what the editor does with commands.

use std::io::Write;
use std::thread;

use anyhow::Result;
use crossterm::event::{self, Event};

use super::input;
use super::render::Renderer;
use crate::editor::job::Message;
use crate::editor::Editor;

/// One queue, one thread that owns the editor (ADR 21). The terminal is read
/// on a thread of its own, because `event::read` blocks and the queue is
/// what the loop has to wait on.
pub fn run(editor: &mut Editor, out: &mut impl Write) -> Result<()> {
    let mut renderer = Renderer::new();

    // Started only now: the terminal has been set up, and its answer to the
    // keyboard-protocol query has been read by whoever asked.
    let queue = editor.sender();
    thread::spawn(move || {
        while let Ok(event) = event::read() {
            if let Some(input) = input::translate(event)
                && queue.send(Message::Input(input)).is_err()
            {
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

        for input in inputs {
            if let Some(command) = input.into_command(editor) {
                editor.execute(command);
            }
            if editor.should_quit() {
                return Ok(());
            }
        }
        for report in reports {
            editor.receive(report);
        }
    }
}

/// What the loop does with one event, for a test that drives the editor with
/// events and never opens a terminal.
pub fn handle(event: Event, editor: &mut Editor) {
    let command = input::translate(event).and_then(|input| input.into_command(editor));
    if let Some(command) = command {
        editor.execute(command);
    }
}
