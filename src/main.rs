use std::io::{self, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};

use ned::buffer::Buffer;
use ned::editor::{Editor, Exit};
use ned::term::app;
#[cfg(debug_assertions)]
use ned::term::sandbox::Sandbox;
use ned::term::socket::{self, Server};
use ned::term::system_clipboard::SystemClipboard;
use ned::term::terminal::{self, Terminal};

/// `ned [--socket PATH] [--sandbox] [FILE]`; `--sandbox` in a debug build
/// only (ADR 27).
struct Args {
    file: Option<String>,
    socket: Option<PathBuf>,
    sandbox: bool,
}

fn args() -> Result<Args> {
    let mut args = Args {
        file: None,
        socket: None,
        sandbox: false,
    };
    let mut rest = std::env::args().skip(1);
    while let Some(argument) = rest.next() {
        match argument.as_str() {
            "--socket" => {
                let path = rest.next().context("--socket takes a path")?;
                args.socket = Some(PathBuf::from(path));
            }
            "--sandbox" if cfg!(debug_assertions) => args.sandbox = true,
            flag if flag.starts_with("--") => bail!("unknown option `{flag}`"),
            path => match args.file {
                None => args.file = Some(path.to_owned()),
                Some(_) => bail!("one file at a time"),
            },
        }
    }
    Ok(args)
}

fn main() -> Result<ExitCode> {
    terminal::install_panic_hook();
    let args = args()?;

    // A path wins over a pipe: `ned file < input` edits the file.
    let buffer = match args.file {
        Some(path) => Buffer::open(path)?,
        None if io::stdin().is_terminal() => Buffer::default(),
        None => Buffer::from_stdin()?,
    };

    let downstream = terminal::claim_stdout()?;
    let term = Terminal::enter()?;
    let (mut editor, mut server) = match args.sandbox {
        #[cfg(debug_assertions)]
        true => {
            let sandbox = Sandbox::enter()?;
            let editor = Editor::new(buffer).with_desktop(sandbox.desktop());
            (editor, Server::sandboxed(sandbox))
        }
        _ => (
            Editor::new(buffer).with_desktop(SystemClipboard::new()),
            Server::new(),
        ),
    };
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
