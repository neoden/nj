//! The sandbox (ADR 29): a process in the terminal that runs the editor as
//! its child under the conditions a scenario assumes, and answers for
//! everything around the editor. It listens on a socket for lines of the
//! scenario language. The `%` lines it carries out itself: `%file` in the
//! run's directory, `%stdin` kept for the start, `%start ARGS` the editor
//! started with those arguments, in this terminal. The rest it hands on to
//! the editor's own socket and answers as the editor did. When the editor
//! exits, the answer is the process's exit code and, for a pipeline, what
//! it wrote; the next `%start` starts the next one.
//!
//!   cargo run -q --bin sandbox              in a terminal of its own
//!   cargo run -q --bin replay -- FILE.sc    in another
//!
//! Test machinery, so the debug build only, like the headless run.

#[cfg(debug_assertions)]
fn main() -> anyhow::Result<()> {
    live::main()
}

#[cfg(not(debug_assertions))]
fn main() {
    eprintln!("the sandbox is test machinery and exists in the debug build only");
    std::process::exit(2);
}

#[cfg(debug_assertions)]
mod live {
    use std::fs::{self, File};
    use std::io::{BufRead, BufReader, ErrorKind, Write};
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    use anyhow::{Context, Result, bail};
    use signal_hook::consts::{SIGINT, SIGTERM};

    use ned::term::launch::{Args, Outcome};
    use ned::term::sandbox::{SOCKET, Workdir};
    use ned::term::scenario::{self, Instruction};

    /// In the run's directory: the editor's socket, its pipe and its output.
    const EDITOR_SOCKET: &str = ".editor.sock";
    const STDIN: &str = ".stdin";
    const STDOUT: &str = ".stdout";
    /// The fake desktop clipboard: one file, and the helpers the editor
    /// finds on its `PATH` in place of the desktop's tools
    /// (`system_clipboard.rs`), each reading or writing that file.
    const CLIPBOARD: &str = ".clipboard";
    const HELPERS: &str = ".bin";
    const HELPER_SCRIPTS: &[(&str, &str)] = &[
        ("wl-copy", "cat > \"$CLIPBOARD\""),
        ("wl-paste", "cat \"$CLIPBOARD\" 2>/dev/null"),
        ("xclip", "case \"$*\" in *-o*) cat \"$CLIPBOARD\" 2>/dev/null;; *) cat > \"$CLIPBOARD\";; esac"),
        ("xsel", "case \"$*\" in *-o*) cat \"$CLIPBOARD\" 2>/dev/null;; *) cat > \"$CLIPBOARD\";; esac"),
    ];

    pub fn main() -> Result<()> {
        let mut socket = PathBuf::from(SOCKET);
        let mut arguments = std::env::args().skip(1);
        while let Some(argument) = arguments.next() {
            match argument.as_str() {
                "--socket" => socket = PathBuf::from(arguments.next().context("--socket takes a path")?),
                other => bail!("unknown argument `{other}`"),
            }
        }

        // Ctrl+C, or a `kill`, only raises the flag: the loop sees it at
        // its next turn and returns, so that the directory and the socket
        // are taken down by their owners rather than left behind.
        let stop = Arc::new(AtomicBool::new(false));
        signal_hook::flag::register(SIGINT, stop.clone())?;
        signal_hook::flag::register(SIGTERM, stop.clone())?;

        let run = std::env::temp_dir().join(format!("ned-sandbox.{}", std::process::id()));
        let _workdir = Workdir::enter_at(run)?;
        let listener = listen(&socket)?;
        listener.set_nonblocking(true)?;
        let mut sandbox = Sandbox::new()?;
        // One client at a time, each served to the end of its connection.
        // A client is a scenario: where the last one left the editor
        // exited, the next begins before an editor, as a scenario does.
        while !stop.load(Ordering::Relaxed) {
            let stream = match listener.accept() {
                Ok((stream, _)) => stream,
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                    continue;
                }
                Err(error) => return Err(error.into()),
            };
            if let Stage::Exited(_) = sandbox.stage {
                sandbox = Sandbox::new()?;
            }
            serve(stream, &mut sandbox, &stop)?;
        }
        sandbox.leave();
        Ok(())
    }

    /// Answers a client's lines until it hangs up, looking up between
    /// lines for the flag.
    fn serve(stream: UnixStream, sandbox: &mut Sandbox, stop: &AtomicBool) -> Result<()> {
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;
        let mut out = stream.try_clone()?;
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        while !stop.load(Ordering::Relaxed) {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {}
                Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    continue;
                }
                Err(error) => return Err(error.into()),
            }
            let answer = match sandbox.carry_out(line.trim_end_matches('\n')) {
                Ok(answer) => answer,
                Err(error) => format!("error: {error:#}\n"),
            };
            // The answer ends with a newline of its own; this adds the
            // blank line that ends every answer (ADR 26).
            if writeln!(out, "{answer}").is_err() {
                break;
            }
        }
        Ok(())
    }

    /// Listens at `path`, replacing a socket an earlier sandbox left there
    /// and refusing anything else at the path. The file goes with the
    /// listener.
    fn listen(path: &Path) -> Result<Listener> {
        if let Ok(metadata) = fs::symlink_metadata(path) {
            if !metadata.file_type().is_socket() {
                bail!("{} exists and is not a socket", path.display());
            }
            fs::remove_file(path)?;
        }
        let listener = UnixListener::bind(path)
            .with_context(|| format!("cannot listen at {}", path.display()))?;
        Ok(Listener {
            listener,
            path: path.to_owned(),
        })
    }

    struct Listener {
        listener: UnixListener,
        path: PathBuf,
    }

    impl std::ops::Deref for Listener {
        type Target = UnixListener;
        fn deref(&self) -> &UnixListener {
            &self.listener
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    /// Where the run is: before an editor, with one, or after it has exited.
    enum Stage {
        Before,
        Running(Running),
        Exited(Outcome),
    }

    /// The editor as a child process, reached over its socket.
    struct Running {
        child: Child,
        editor: Connection,
        /// Whether the editor was given a pipe: then what it wrote is part
        /// of the outcome.
        piped: bool,
    }

    struct Sandbox {
        stage: Stage,
        /// What `%stdin` gave, until the editor starts and reads it.
        stdin: Option<String>,
    }

    impl Sandbox {
        /// A run before its editor, with nothing on the clipboard and the
        /// helpers in place for the editor to find.
        fn new() -> Result<Self> {
            let _ = fs::remove_file(CLIPBOARD);
            fs::create_dir_all(HELPERS)?;
            let clipboard = std::env::current_dir()?.join(CLIPBOARD);
            for (name, body) in HELPER_SCRIPTS {
                let path = Path::new(HELPERS).join(name);
                let script = format!("#!/bin/sh\nCLIPBOARD={}\n{body}\n", shell_quote(&clipboard));
                fs::write(&path, script)?;
                fs::set_permissions(&path, fs::Permissions::from_mode(0o755))?;
            }
            Ok(Self {
                stage: Stage::Before,
                stdin: None,
            })
        }

        /// Carries out one line and says what came of it, each line of the
        /// answer ended by a newline.
        fn carry_out(&mut self, line: &str) -> Result<String> {
            let Some(instruction) = scenario::parse(line)? else {
                return Ok("ok\n".to_owned());
            };
            // An editor the person quit by hand is gone without a line
            // having said so.
            if let Stage::Running(running) = &mut self.stage
                && running.child.try_wait()?.is_some()
            {
                self.finish()?;
            }
            match instruction {
                Instruction::File(name, text) => {
                    fs::write(&name, text).with_context(|| format!("cannot write {name}"))?;
                }
                Instruction::Clipboard(text) => {
                    fs::write(CLIPBOARD, text).context("cannot write the clipboard")?;
                }
                Instruction::Stdin(text) => {
                    if let Stage::Running(_) = self.stage {
                        bail!("the editor is already running");
                    }
                    self.stdin = Some(text);
                }
                Instruction::Start(args) => {
                    if let Stage::Running(_) = self.stage {
                        bail!("the editor is already running");
                    }
                    self.start(args)?;
                }
                Instruction::Size(..)
                | Instruction::Snapshot(_)
                | Instruction::Input(_)
                | Instruction::Wait => {
                    return self.forward(line);
                }
            }
            Ok("ok\n".to_owned())
        }

        /// An editor's own line: to the editor, started first if the run
        /// has none yet, as `ned` alone would be. After the exit a snapshot
        /// shows the outcome and nothing else is taken (ADR 28).
        fn forward(&mut self, line: &str) -> Result<String> {
            match &mut self.stage {
                Stage::Before => self.start(Args::default())?,
                Stage::Running(_) => {}
                Stage::Exited(outcome) => {
                    if line.starts_with("---") {
                        return Ok(scenario::outcome(outcome));
                    }
                    bail!("the editor has exited");
                }
            }
            let Stage::Running(running) = &mut self.stage else {
                unreachable!("started above");
            };
            let answer = running.editor.ask(line)?;
            // The editor leaving is the answer (ADR 28); the process's own
            // end is the one reported.
            if answer.starts_with("exit ") {
                return self.finish();
            }
            Ok(answer)
        }

        /// Starts the editor as `args` would from the shell, in this
        /// terminal: `%stdin` on a pipe, standard output kept for the
        /// outcome, the socket the sandbox's to name.
        fn start(&mut self, args: Args) -> Result<()> {
            if args.socket.is_some() {
                bail!("--socket: the sandbox gives the editor its socket");
            }
            let _ = fs::remove_file(EDITOR_SOCKET);
            let mut command = Command::new(editor_binary()?);
            command.arg("--socket").arg(EDITOR_SOCKET);
            if let Some(file) = &args.file {
                command.arg(file);
            }
            let piped = match self.stdin.take() {
                Some(text) => {
                    fs::write(STDIN, text)?;
                    command.stdin(File::open(STDIN)?);
                    true
                }
                None => {
                    command.stdin(Stdio::inherit());
                    false
                }
            };
            command.stdout(File::create(STDOUT)?);
            // The helpers come first on the editor's `PATH`, so its
            // clipboard is the run's and the desktop's is left alone.
            let helpers = std::env::current_dir()?.join(HELPERS);
            let path = match std::env::var_os("PATH") {
                Some(rest) => std::env::join_paths(std::iter::once(helpers).chain(std::env::split_paths(&rest)))?,
                None => helpers.into_os_string(),
            };
            command.env("PATH", path);
            let mut child = command.spawn().context("cannot start the editor")?;
            let editor = connect(Path::new(EDITOR_SOCKET), &mut child)?;
            self.stage = Stage::Running(Running {
                child,
                editor,
                piped,
            });
            Ok(())
        }

        /// The sandbox is leaving: an editor still running goes with it,
        /// so that the terminal is not left to a child nobody drives.
        fn leave(&mut self) {
            if let Stage::Running(running) = &mut self.stage {
                let _ = running.child.kill();
                let _ = running.child.wait();
            }
        }

        /// Waits for the editor's process to end and keeps its outcome:
        /// the exit code and, for a pipeline, what it wrote.
        fn finish(&mut self) -> Result<String> {
            let Stage::Running(running) = &mut self.stage else {
                unreachable!("finished while running");
            };
            let status = running.child.wait()?;
            let code = status.code().unwrap_or(1).clamp(0, 255) as u8;
            let stdout = match running.piped {
                true => Some(fs::read_to_string(STDOUT)?),
                false => None,
            };
            let outcome = Outcome { code, stdout };
            let answer = scenario::outcome(&outcome);
            self.stage = Stage::Exited(outcome);
            Ok(answer)
        }
    }

    /// `path` as a word of a shell script: in single quotes, with a quote
    /// inside written as `'\''`.
    fn shell_quote(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }

    /// The editor next to this binary: both are built into the same
    /// directory.
    fn editor_binary() -> Result<PathBuf> {
        let path = std::env::current_exe()?
            .parent()
            .context("the sandbox has no directory")?
            .join("ned");
        if !path.exists() {
            bail!("{} is not built: `cargo build`", path.display());
        }
        Ok(path)
    }

    /// Waits for the editor to listen at `socket`, unless it ends first.
    fn connect(socket: &Path, child: &mut Child) -> Result<Connection> {
        let patience = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait()? {
                bail!("the editor ended before it listened: {status}");
            }
            match UnixStream::connect(socket) {
                Ok(stream) => return Connection::over(stream),
                Err(_) if Instant::now() < patience => thread::sleep(Duration::from_millis(50)),
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("the editor does not listen at {}", socket.display()));
                }
            }
        }
    }

    /// A line in, the answer out, over the editor's socket.
    struct Connection {
        stream: UnixStream,
        reader: BufReader<UnixStream>,
    }

    impl Connection {
        fn over(stream: UnixStream) -> Result<Self> {
            Ok(Self {
                reader: BufReader::new(stream.try_clone()?),
                stream,
            })
        }

        /// The answer's lines up to the blank one that ends it, each with
        /// its newline, as the sandbox passes it on.
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
                bail!("the editor went away without answering");
            }
            Ok(answer)
        }
    }
}
