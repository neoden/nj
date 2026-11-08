//! The colours the editor is drawn in: one theme, built in, after the
//! `dwarf` scheme for micro. Every colour is given exactly, never as a slot
//! of the terminal's palette, so the editor looks the same in any terminal.

use std::fmt;

use crossterm::style::Color;

/// A colour in 24 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// Written as X11 spells a colour, `rgb:rr/gg/bb`, which is what the
/// sequences that set the terminal's own colours take.
impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Rgb(r, g, b) = self;
        write!(f, "rgb:{r:02x}/{g:02x}/{b:02x}")
    }
}

impl From<Rgb> for Color {
    fn from(Rgb(r, g, b): Rgb) -> Self {
        Color::Rgb { r, g, b }
    }
}

/// The text where nothing else colours it, and what it lies on.
pub const FOREGROUND: Rgb = Rgb(0xcd, 0xce, 0xd2);
pub const BACKGROUND: Rgb = Rgb(0x14, 0x18, 0x1d);

/// The numbers of the lines other than the current one, and the rule
/// above a panel; the current line's number is in the foreground. Dimmer
/// than a comment, though not as dim as dwarf's `#3d4248`, which was near
/// to the background.
pub const DIM: Rgb = Rgb(0x5a, 0x62, 0x70);

pub const COMMENT: Rgb = Rgb(0x6d, 0x77, 0x86);
pub const KEYWORD: Rgb = Rgb(0x8a, 0xa7, 0xd6);
/// The keyword's blue before dwarf brightened it: of its family, and told
/// from it.
pub const LABEL: Rgb = Rgb(0x6c, 0x88, 0xaf);
/// Where a function is defined; a call to it is plain. Dwarf's orange,
/// which it gave constants: a hue of the scheme's own, where every new
/// one was either drowned by the text or stood out of the scheme.
pub const FUNCTION: Rgb = Rgb(0xd6, 0x8f, 0x61);
pub const STRING: Rgb = Rgb(0xa3, 0xb8, 0x84);
/// A warm gold, the lightest of the colours: a constant is rare, and a
/// number in the code is worth seeing. Lighter and fuller than the
/// string's green, since to an eye that does not tell red from green the
/// two differ in nothing else.
pub const CONSTANT: Rgb = Rgb(0xe3, 0xc4, 0x7a);
/// Dwarf's cool grey-blue for a type, `#7a8294`, lifted: as it was it
/// ran into a comment beside it.
pub const TYPE: Rgb = Rgb(0x88, 0x91, 0xa3);
/// What the status bar lies on; its text is in the background colour. The
/// text's own colours turned over, so that the bar reads at once as not
/// being text: dwarf's dark bar looked like one more line of it.
pub const STATUS: Rgb = FOREGROUND;

/// What lies under the line the cursor is on, and under a selection.
pub const CURRENT_LINE: Rgb = Rgb(0x1f, 0x25, 0x30);
/// Further from the current line than the line is from the background,
/// and tinted with the keyword's blue: dwarf's grey `#2a2e35` was a step
/// above the current line, and a selection on it was hard to see.
pub const SELECTION: Rgb = Rgb(0x29, 0x37, 0x50);

pub const ADDED: Rgb = Rgb(0xa3, 0xb8, 0x84);
pub const DELETED: Rgb = Rgb(0xc4, 0x7b, 0x6e);
