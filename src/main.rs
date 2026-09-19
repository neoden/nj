use std::io::{self, IsTerminal};
use std::process::ExitCode;

use anyhow::Result;

use ned::buffer::Buffer;
use ned::editor::{Editor, Exit};
use ned::term::app;
use ned::term::system_clipboard::SystemClipboard;
use ned::term::terminal::{self, Terminal};

fn main() -> Result<ExitCode> {
    terminal::install_panic_hook();

    // A path wins over a pipe: `ned file < input` edits the file.
    let buffer = match std::env::args().nth(1) {
        Some(path) => Buffer::open(path)?,
        None if io::stdin().is_terminal() => Buffer::default(),
        None => Buffer::from_stdin()?,
    };

    let downstream = terminal::claim_stdout()?;
    let term = Terminal::enter()?;
    let mut editor = Editor::new(buffer).with_desktop(SystemClipboard::new());
    if !term.keyboard_enhanced {
        editor.say("Legacy keyboard: some combinations are unavailable");
    }

    // The terminal is restored before anything else is printed, or it would
    // land on the alternate screen and vanish with it.
    let result = app::run(&mut editor, &mut io::stdout());
    drop(term);
    result?;

    // An abort hands nothing on and fails, so that a pipeline or a commit
    // waiting on the editor stops here.
    if editor.exit() == Some(Exit::Abort) {
        return Ok(ExitCode::FAILURE);
    }
    if let Some(mut downstream) = downstream {
        editor.buffer().finish(&mut downstream)?;
    }
    Ok(ExitCode::SUCCESS)
}
