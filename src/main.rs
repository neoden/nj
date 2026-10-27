use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anyhow::Result;

use ned::editor::Editor;
use ned::term::socket::{self, Server};
use ned::term::system_clipboard::SystemClipboard;
use ned::term::terminal::{self, Terminal};
use ned::term::{app, launch};

fn main() -> Result<ExitCode> {
    terminal::install_panic_hook();
    let args = launch::args(std::env::args().skip(1))?;
    let pipe = (!io::stdin().is_terminal()).then(|| io::stdin().lock());
    let buffer = launch::buffer(args.file.as_deref(), pipe)?;

    let mut downstream: Box<dyn Write> = match terminal::claim_stdout()? {
        Some(pipe) => Box::new(pipe),
        None => Box::new(io::sink()),
    };
    let term = Terminal::enter()?;
    let mut editor = Editor::new(buffer).with_desktop(SystemClipboard::new());
    let mut server = Server::new();
    if !term.keyboard_enhanced {
        editor.say("Legacy keyboard: some combinations are unavailable");
    }
    // Held until the editor is done, so the socket's file goes with it.
    let _listener = match &args.socket {
        Some(path) => Some(socket::listen(path, editor.sender())?),
        None => None,
    };

    // The terminal is restored before anything else is printed, or it would
    // land on the alternate screen and vanish with it.
    let result = app::run(&mut editor, &mut io::stdout(), &mut server);
    drop(term);
    result?;

    Ok(ExitCode::from(launch::conclude(&editor, &mut downstream)?))
}
