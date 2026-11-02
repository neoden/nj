//! Replays a scenario in a live editor, one line at a time. The
//! editor runs in the sandbox, in a terminal of its own; this side sends
//! the sandbox every line of the file over its socket and shows what came
//! back.
//!
//!   cargo run -q --bin sandbox              in a terminal of its own
//!   cargo run -q --bin replay -- FILE.sc    in another
//!
//! The sandbox is reset first, so the scenario begins as the file does,
//! whatever the last replay left there — an editor still open included.
//! Each line is shown and sent on Enter; `a` sends the rest without
//! stopping, `q` stops. At `---` the sandbox is asked for the screen and
//! the frame is worked out here, as the test works it out, then compared
//! with the frame in the file; both are printed only where they differ.
//! The instructions are described in `term::scenario`.

#[cfg(debug_assertions)]
fn main() -> anyhow::Result<std::process::ExitCode> {
    live::main()
}

#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("the replay is test machinery and exists in the debug build only");
    std::process::exit(2);
}

#[cfg(debug_assertions)]
mod live {
    use std::fs;
    use std::io::{self, BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::process::ExitCode;

    use anyhow::{Context, Result, bail};

    use nj::term::sandbox::SOCKET;
    use nj::term::scenario::{self, Instruction, Line};
    use nj::term::screen::{Frames, Screen};
    use nj::term::socket::Command;

    pub fn main() -> Result<ExitCode> {
        let mut socket = PathBuf::from(SOCKET);
        let mut path = None;
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--socket" => {
                    socket = PathBuf::from(arguments.next().context("--socket takes a path")?)
                }
                flag if flag.starts_with("--") => bail!("unknown option `{flag}`"),
                file => match path {
                    None => path = Some(file.to_owned()),
                    Some(_) => bail!("one scenario at a time"),
                },
            }
        }
        let path = path.context("a scenario file to replay")?;
        let script = fs::read_to_string(&path).with_context(|| format!("cannot read {path}"))?;

        let stream = UnixStream::connect(&socket)
            .with_context(|| format!("no sandbox listens at {}", socket.display()))?;
        let mut sandbox = Sandbox {
            reader: BufReader::new(stream.try_clone()?),
            stream,
        };
        // A replay is the scenario from its first line, so the run begins
        // where the file does: no editor from last time, an empty directory.
        let answer = sandbox.ask("%reset")?;
        if answer != "ok\n" {
            bail!("the sandbox would not reset: {}", answer.trim_end());
        }

        let mut expected = expected_frames(&script).into_iter();
        let mut frames = Frames::default();
        let mut differ = 0;
        let mut unattended = false;
        for line in scenario::lines(&script) {
            let (text, snapshot) = match line {
                Line::Text(text) if text.is_empty() || text.starts_with('#') => continue,
                Line::Text(text) => (text.to_owned(), None),
                Line::Snapshot(layer, caption) => {
                    (scenario::snapshot_line(layer, caption), Some(layer))
                }
                Line::Frame(_) => continue,
            };
            if unattended {
                println!("{text}");
            } else {
                print!("{text}  ");
                io::stdout().flush()?;
                let mut command = String::new();
                if io::stdin().read_line(&mut command)? == 0 || command.trim() == "q" {
                    return Ok(ExitCode::SUCCESS);
                }
                unattended = command.trim() == "a";
            }

            let Some(layer) = snapshot else {
                let answer = sandbox.ask(&text)?;
                if answer != "ok\n" {
                    println!("    {}", answer.trim_end().replace('\n', "\n    "));
                }
                // A reset starts the run over, the frames taken with it.
                if let Ok(Some(Instruction::Reset)) = scenario::parse(&text) {
                    frames = Frames::default();
                }
                continue;
            };
            // A snapshot is the run's: the sandbox answers with the screen,
            // and what changed since the last snapshot is worked out here.
            let (number, wanted) = expected.next().expect("a frame per snapshot");
            let answer = sandbox.ask(Command::SCREEN)?;
            let screen = Screen::read(&answer)
                .with_context(|| format!("line {number}: {}", answer.trim_end()))?;
            let frame = frames.take(screen, layer);
            if frame != wanted {
                differ += 1;
                print!("{frame}");
                println!("the file has (line {number}):");
                print!("{wanted}");
            }
        }
        match differ {
            0 => println!("done"),
            n => println!("done, {n} snapshot(s) differ"),
        }
        Ok(match differ {
            0 => ExitCode::SUCCESS,
            _ => ExitCode::FAILURE,
        })
    }

    /// The sandbox at the other end of the socket.
    struct Sandbox {
        stream: UnixStream,
        reader: BufReader<UnixStream>,
    }

    impl Sandbox {
        /// One line in, the answer out: the lines up to the blank one that
        /// ends every answer, each with its newline.
        fn ask(&mut self, line: &str) -> Result<String> {
            writeln!(self.stream, "{line}")?;
            let mut answer = String::new();
            loop {
                let mut received = String::new();
                if self.reader.read_line(&mut received)? == 0 || received == "\n" {
                    break;
                }
                answer.push_str(&received);
            }
            if answer.is_empty() {
                bail!("the sandbox went away");
            }
            Ok(answer)
        }
    }

    /// The frame under each snapshot of the file, with the snapshot's line
    /// number: the lines that follow `---` up to the blank one, as the test
    /// writes them, ended by a newline each.
    fn expected_frames(script: &str) -> Vec<(usize, String)> {
        let mut frames: Vec<(usize, String)> = Vec::new();
        for (index, line) in scenario::lines(script).enumerate() {
            match line {
                Line::Snapshot(..) => frames.push((index + 1, String::new())),
                Line::Frame("") | Line::Text(_) => {}
                Line::Frame(row) => {
                    let (_, frame) = frames.last_mut().expect("a frame follows its snapshot");
                    frame.push_str(row);
                    frame.push('\n');
                }
            }
        }
        frames
    }
}
