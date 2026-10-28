//! The loop and the dispatch around the editor: what the process does with
//! events, as opposed to what the editor does with commands.

use std::io::Write;
use std::thread;

use anyhow::{Result, bail};
use crossterm::event::{self, Event};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use super::input;
use super::render::Renderer;
use super::socket::Server;
use crate::editor::Editor;
use crate::editor::job::Message;

/// The signals that ask the process to end, caught from here on: a signal
/// arriving before `run` forwards them is kept, not acted on by default.
/// Caught before the terminal is touched, so that none can leave it in raw
/// mode. Ctrl+C in raw mode is a key, not SIGINT; `kill -INT` still is.
pub fn signals() -> Result<Signals> {
    Ok(Signals::new([SIGTERM, SIGHUP, SIGINT])?)
}

/// One queue, one thread that owns the editor. The terminal is read
/// on a thread of its own, because `event::read` blocks and the queue is
/// what the loop has to wait on.
pub fn run(
    editor: &mut Editor,
    out: &mut impl Write,
    server: &mut Server,
    mut signals: Signals,
) -> Result<()> {
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

    // A signal asking the process to end is one more sender on the same
    // queue: the loop wakes for it as for a key.
    let queue = editor.sender();
    thread::spawn(move || {
        for _ in signals.forever() {
            if queue.send(Message::Terminate).is_err() {
                return;
            }
        }
    });

    loop {
        let (width, height) = crossterm::terminal::size()?;
        let terminal = (width as usize, height as usize);
        let size = server.frame_size(terminal);
        editor.resize(size.0, size.1);
        renderer.draw(out, editor, size)?;

        // Whatever has arrived by the time the loop wakes is handled before
        // the next frame, input first: a keystroke never queues behind the
        // reports that happened to come with it.
        let Some(first) = editor.wait() else {
            return Ok(());
        };
        let mut inputs = Vec::new();
        let mut remotes = Vec::new();
        let mut reports = Vec::new();
        let mut terminate = false;
        let mut sort = |message| match message {
            Message::Input(event) => inputs.push(event),
            Message::Remote(remote) => remotes.push(remote),
            Message::Report(report) => reports.push(report),
            Message::Terminate => terminate = true,
        };
        sort(first);
        while let Some(message) = editor.pending() {
            sort(message);
        }

        // Before anything that came with it: the process is ending, and a
        // keystroke in the same burst would act on an editor about to go.
        if terminate {
            editor.terminate();
            return Ok(());
        }

        for input in inputs {
            if let Some(command) = input.into_command(editor) {
                editor.execute(command);
            }
            if editor.exit().is_some() {
                return Ok(());
            }
        }
        for report in reports {
            editor.receive(report);
        }
        // A line from the socket is input too, and is answered once it has
        // been carried out, so that a client sending a key and asking for
        // the frame gets the frame after the key. It comes after the
        // reports: `wait` finishes the jobs by reading the queue, and a
        // report that arrived with the line is in `reports` by now, not in
        // the queue, so handled first it is seen and left for later it
        // would be waited for forever.
        for remote in remotes {
            let answer = server.answer(editor, &remote.line, terminal);
            let _ = remote.reply.send(answer);
            if editor.exit().is_some() {
                return Ok(());
            }
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

/// The events of one line of a scenario, handled one at a time, so that an
/// exit in the middle of a `type` is seen: the events after it would go to
/// an editor that has left, and are refused instead.
pub fn handle_all(events: Vec<Event>, editor: &mut Editor) -> Result<()> {
    let mut events = events.into_iter();
    for event in events.by_ref() {
        handle(event, editor);
        if editor.exit().is_some() {
            break;
        }
    }
    if events.next().is_some() {
        bail!("input after the editor has exited");
    }
    Ok(())
}
