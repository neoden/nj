use std::io::{self, IsTerminal};

use anyhow::Result;

use ned::app;
use ned::buffer::Buffer;
use ned::editor::Editor;
use ned::terminal::{self, Terminal};

fn main() -> Result<()> {
    terminal::install_panic_hook();

    // A path wins over a pipe: `ned file < input` edits the file.
    let buffer = match std::env::args().nth(1) {
        Some(path) => Buffer::open(path)?,
        None if io::stdin().is_terminal() => Buffer::default(),
        None => Buffer::from_stdin()?,
    };

    let downstream = terminal::claim_stdout()?;
    let term = Terminal::enter()?;
    let mut editor = Editor::new(buffer).with_system_clipboard();
    if !term.keyboard_enhanced {
        editor.message = Some("Legacy keyboard: some combinations are unavailable".to_owned());
    }

    // The terminal is restored before anything else is printed, or it would
    // land on the alternate screen and vanish with it.
    let result = app::run(&mut editor, &mut io::stdout());
    drop(term);
    result?;

    if let Some(mut downstream) = downstream {
        editor.buffer.finish(&mut downstream)?;
    }
    Ok(())
}
