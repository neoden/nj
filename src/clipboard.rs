use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crossterm::clipboard::CopyToClipboard;
use crossterm::execute;

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
        let output = Command::new(tool.paste[0])
            .args(&tool.paste[1..])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;

        match output.status.success() {
            true => String::from_utf8(output.stdout).ok(),
            false => None,
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

fn in_path(program: &str) -> bool {
    env::var_os("PATH").is_some_and(|path| {
        env::split_paths(&path).any(|directory| Path::new(&directory).join(program).is_file())
    })
}

/// What was copied, kept as one piece per cursor so that a multi-cursor copy
/// can be pasted back into the same number of cursors.
#[derive(Debug, Default)]
pub struct Clipboard {
    pieces: Vec<String>,
    /// Copied with nothing selected, meaning whole lines were taken and whole
    /// lines should be put back.
    lines: bool,
}

impl Clipboard {
    pub fn set(&mut self, pieces: Vec<String>, lines: bool) {
        self.pieces = pieces;
        self.lines = lines;
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    pub fn holds_lines(&self) -> bool {
        self.lines
    }

    pub fn text(&self) -> String {
        match self.lines {
            true => self.pieces.concat(),
            false => self.pieces.join("\n"),
        }
    }

    /// Cursor for cursor when the counts agree; otherwise every cursor gets
    /// the whole thing.
    pub fn pieces_for(&self, cursors: usize) -> Vec<String> {
        match self.pieces.len() == cursors {
            true => self.pieces.clone(),
            false => vec![self.text(); cursors],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clipboard(pieces: &[&str], lines: bool) -> Clipboard {
        let mut clipboard = Clipboard::default();
        clipboard.set(pieces.iter().map(|&piece| piece.to_owned()).collect(), lines);
        clipboard
    }

    #[test]
    fn matching_cursor_counts_get_a_piece_each() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(2), ["one", "two"]);
    }

    #[test]
    fn a_lone_cursor_gets_everything() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(1), ["one\ntwo"]);
    }

    #[test]
    fn mismatched_counts_get_everything_each() {
        let clipboard = clipboard(&["one", "two"], false);
        assert_eq!(clipboard.pieces_for(3), ["one\ntwo", "one\ntwo", "one\ntwo"]);
    }

    #[test]
    fn copied_lines_already_carry_their_breaks() {
        let clipboard = clipboard(&["one\n", "two\n"], true);
        assert_eq!(clipboard.text(), "one\ntwo\n");
    }
}
