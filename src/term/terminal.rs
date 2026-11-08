use std::fmt;
use std::fs::File;
use std::io::{self, IsTerminal, Write};
use std::os::fd::AsFd;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result, bail};
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    supports_keyboard_enhancement,
};
use crossterm::{Command, cursor, execute};

use super::theme;

static MODIFIED: AtomicBool = AtomicBool::new(false);
static KEYBOARD_FLAGS_PUSHED: AtomicBool = AtomicBool::new(false);

/// When standard output is a pipe carrying the edited text downstream, takes
/// it away and puts the controlling terminal on descriptor 1 for the rest of
/// the session. Everything that assumes stdout is the screen — our drawing,
/// crossterm's fallbacks, a panic message — is then right, and the pipe
/// receives nothing but what is written to the handle returned here.
///
/// `None` when standard output is the terminal already.
pub fn claim_stdout() -> Result<Option<File>> {
    if io::stdout().is_terminal() {
        return Ok(None);
    }
    let downstream = io::stdout().as_fd().try_clone_to_owned()?;
    let tty = File::options()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .context("not running in a terminal")?;
    rustix::stdio::dup2_stdout(&tty)?;
    Ok(Some(File::from(downstream)))
}

/// Owns the terminal's raw mode and alternate screen for as long as it lives.
pub struct Terminal {
    /// Whether the kitty keyboard protocol is active, making `Ctrl+Shift+K`,
    /// `Ctrl+/` and friends distinguishable.
    pub keyboard_enhanced: bool,
}

impl Terminal {
    pub fn enter() -> Result<Self> {
        if !io::stdout().is_terminal() {
            bail!("not running in a terminal");
        }

        enable_raw_mode()?;
        MODIFIED.store(true, Ordering::SeqCst);
        // From here on a failure has something to undo, which is what
        // dropping the guard does — so the guard exists before anything
        // else can fail.
        let mut terminal = Self {
            keyboard_enhanced: false,
        };

        let mut out = io::stdout();
        // Capturing the mouse takes the terminal's own selection away; holding
        // Shift while dragging bypasses us and gets it back.
        //
        // Bracketed paste makes the terminal hand over pasted text in one
        // event; without it every pasted character arrives as a keystroke.
        execute!(
            out,
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            cursor::Hide,
            SetDefaultColors
        )?;

        if matches!(supports_keyboard_enhancement(), Ok(true)) {
            execute!(
                out,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )?;
            KEYBOARD_FLAGS_PUSHED.store(true, Ordering::SeqCst);
            terminal.keyboard_enhanced = true;
        }

        Ok(terminal)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        restore();
    }
}

/// Idempotent and panic-free: also runs while unwinding.
fn restore() {
    if !MODIFIED.swap(false, Ordering::SeqCst) {
        return;
    }

    let mut out = io::stdout();
    // Popping a stack entry we did not push would break the next application.
    if KEYBOARD_FLAGS_PUSHED.swap(false, Ordering::SeqCst) {
        let _ = execute!(out, PopKeyboardEnhancementFlags);
    }
    let _ = execute!(
        out,
        ResetDefaultColors,
        cursor::Show,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
    let _ = out.flush();
}

/// Makes the theme's colours the terminal's own default ones (OSC 10 and
/// 11), rather than painting them on every cell: a cell with no colour of
/// its own, a cleared screen and the padding around the cells, which no
/// cell covers, all take them then.
struct SetDefaultColors;

impl Command for SetDefaultColors {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        write!(
            f,
            "\x1b]10;{}\x1b\\\x1b]11;{}\x1b\\",
            theme::FOREGROUND,
            theme::BACKGROUND
        )
    }
}

/// Gives the terminal back the default colours its own settings say
/// (OSC 110 and 111).
struct ResetDefaultColors;

impl Command for ResetDefaultColors {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str("\x1b]110\x1b\\\x1b]111\x1b\\")
    }
}

/// The default hook prints before unwinding, so `Drop` alone would leave the
/// panic message on the alternate screen in raw mode.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        default_hook(info);
    }));
}
