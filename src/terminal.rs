use std::fs::File;
use std::io::{self, IsTerminal, Write};
use std::os::fd::AsFd;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{bail, Context, Result};
use crossterm::cursor;
use crossterm::event::{
    DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
    LeaveAlternateScreen,
};

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
            cursor::Hide
        )?;

        let keyboard_enhanced = matches!(supports_keyboard_enhancement(), Ok(true));
        if keyboard_enhanced {
            execute!(
                out,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )?;
            KEYBOARD_FLAGS_PUSHED.store(true, Ordering::SeqCst);
        }

        Ok(Self { keyboard_enhanced })
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
        cursor::Show,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    );
    let _ = disable_raw_mode();
    let _ = out.flush();
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
