use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{bail, Result};
use crossterm::cursor;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, supports_keyboard_enhancement, EnterAlternateScreen,
    LeaveAlternateScreen,
};

static MODIFIED: AtomicBool = AtomicBool::new(false);
static KEYBOARD_FLAGS_PUSHED: AtomicBool = AtomicBool::new(false);

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
        execute!(out, EnterAlternateScreen, EnableMouseCapture, cursor::Hide)?;

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
    let _ = execute!(out, cursor::Show, DisableMouseCapture, LeaveAlternateScreen);
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
