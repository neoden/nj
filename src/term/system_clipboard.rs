//! The desktop clipboard, reached through external helpers (ADR 11).

use std::env;
use std::io::{self, IsTerminal, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::clipboard::CopyToClipboard;
use crossterm::execute;

use crate::editor::clipboard::Desktop;

const READ_TIMEOUT: Duration = Duration::from_millis(250);

struct Tool {
    display: &'static str,
    copy: &'static [&'static str],
    paste: &'static [&'static str],
}

const TOOLS: &[Tool] = &[
    Tool {
        display: "WAYLAND_DISPLAY",
        copy: &["wl-copy"],
        // Without this it appends a newline that was never copied.
        paste: &["wl-paste", "--no-newline"],
    },
    Tool {
        display: "DISPLAY",
        copy: &["xclip", "-selection", "clipboard"],
        paste: &["xclip", "-selection", "clipboard", "-out"],
    },
    Tool {
        display: "DISPLAY",
        copy: &["xsel", "--input", "--clipboard"],
        paste: &["xsel", "--output", "--clipboard"],
    },
];

/// The desktop clipboard, reached through whichever helper is installed.
/// Owning a selection means staying alive to serve it, which these helpers do
/// by forking; doing it in-process would cost a wayland and an X11 client
/// library and would lose the clipboard when the editor exits.
pub struct SystemClipboard {
    tool: Option<&'static Tool>,
}

impl SystemClipboard {
    pub fn new() -> Self {
        Self {
            tool: TOOLS
                .iter()
                .find(|tool| env::var_os(tool.display).is_some() && in_path(tool.copy[0])),
        }
    }

    pub fn read(&mut self) -> Option<String> {
        let tool = self.tool?;
        let mut child = Command::new(tool.paste[0])
            .args(&tool.paste[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        // `output()` would wait on the helper however long it took.
        let mut stdout = child.stdout.take()?;
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = sender.send(stdout.read_to_end(&mut bytes).map(|_| bytes));
        });

        let Ok(Ok(bytes)) = receiver.recv_timeout(READ_TIMEOUT) else {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        };

        match child.wait() {
            Ok(status) if status.success() => String::from_utf8(bytes).ok(),
            _ => None,
        }
    }

    /// OSC 52 carries the copy to the machine the user is sitting at when
    /// there is no local display to talk to, as over ssh.
    pub fn write(&mut self, text: &str) {
        if self.write_with_tool(text).is_some() {
            return;
        }
        if io::stdout().is_terminal() {
            let _ = execute!(io::stdout(), CopyToClipboard::to_clipboard_from(text));
        }
    }

    fn write_with_tool(&self, text: &str) -> Option<()> {
        let tool = self.tool?;
        let mut child = Command::new(tool.copy[0])
            .args(&tool.copy[1..])
            .stdin(Stdio::piped())
            // The helper forks to serve the selection; handing it a pipe to
            // inherit would leave us waiting on it forever.
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        child.stdin.take()?.write_all(text.as_bytes()).ok()?;
        child.wait().ok()?.success().then_some(())
    }
}

impl Default for SystemClipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Desktop for SystemClipboard {
    fn read(&mut self) -> Option<String> {
        SystemClipboard::read(self)
    }

    fn write(&mut self, text: &str) {
        SystemClipboard::write(self, text);
    }
}

fn in_path(program: &str) -> bool {
    env::var_os("PATH").is_some_and(|path| {
        env::split_paths(&path).any(|directory| Path::new(&directory).join(program).is_file())
    })
}
