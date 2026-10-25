use std::fmt;
use std::str::FromStr;

use anyhow::bail;
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Mods: u8 {
        const CTRL = 1;
        const ALT = 2;
        const SHIFT = 4;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    Char(char),
    Enter,
    Tab,
    Backspace,
    Delete,
    Insert,
    Esc,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

/// A keystroke reduced to what a keymap needs: a physical key plus modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key {
    pub code: KeyCode,
    pub mods: Mods,
}

/// `Ctrl+Shift+Z`, the way a menu would print it.
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (flag, name) in [
            (Mods::CTRL, "Ctrl"),
            (Mods::ALT, "Alt"),
            (Mods::SHIFT, "Shift"),
        ] {
            if self.mods.contains(flag) {
                write!(f, "{name}+")?;
            }
        }
        match self.code {
            // The one character that would otherwise print as nothing.
            KeyCode::Char(' ') => write!(f, "Space"),
            KeyCode::Char(c) => write!(f, "{}", c.to_uppercase()),
            KeyCode::F(n) => write!(f, "F{n}"),
            KeyCode::Enter => write!(f, "Enter"),
            KeyCode::Tab => write!(f, "Tab"),
            KeyCode::Backspace => write!(f, "Backspace"),
            KeyCode::Delete => write!(f, "Delete"),
            KeyCode::Insert => write!(f, "Insert"),
            KeyCode::Esc => write!(f, "Esc"),
            KeyCode::Left => write!(f, "Left"),
            KeyCode::Right => write!(f, "Right"),
            KeyCode::Up => write!(f, "Up"),
            KeyCode::Down => write!(f, "Down"),
            KeyCode::Home => write!(f, "Home"),
            KeyCode::End => write!(f, "End"),
            KeyCode::PageUp => write!(f, "PageUp"),
            KeyCode::PageDown => write!(f, "PageDown"),
        }
    }
}

/// The inverse of `Display`: `Ctrl+Shift+L` back into a key. Names are
/// matched without regard to case, so a scenario or a configuration file
/// may write `ctrl+shift+l`. Whatever is left after the modifiers is the
/// key, which is how `Ctrl++` is `+` under Ctrl.
impl FromStr for Key {
    type Err = anyhow::Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        let mut mods = Mods::empty();
        let mut rest = name;
        loop {
            let (prefix, flag) = match rest.split_once('+') {
                Some((prefix, _)) if prefix.eq_ignore_ascii_case("ctrl") => (prefix, Mods::CTRL),
                Some((prefix, _)) if prefix.eq_ignore_ascii_case("alt") => (prefix, Mods::ALT),
                Some((prefix, _)) if prefix.eq_ignore_ascii_case("shift") => (prefix, Mods::SHIFT),
                _ => break,
            };
            mods |= flag;
            rest = &rest[prefix.len() + 1..];
        }
        let code = match rest.to_ascii_lowercase().as_str() {
            "space" => KeyCode::Char(' '),
            "enter" => KeyCode::Enter,
            "tab" => KeyCode::Tab,
            "backspace" => KeyCode::Backspace,
            "delete" => KeyCode::Delete,
            "insert" => KeyCode::Insert,
            "esc" => KeyCode::Esc,
            "left" => KeyCode::Left,
            "right" => KeyCode::Right,
            "up" => KeyCode::Up,
            "down" => KeyCode::Down,
            "home" => KeyCode::Home,
            "end" => KeyCode::End,
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            lower => {
                let mut chars = rest.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => KeyCode::Char(c),
                    _ => match lower.strip_prefix('f').and_then(|n| n.parse().ok()) {
                        Some(n) => KeyCode::F(n),
                        None => bail!("unknown key `{name}`"),
                    },
                }
            }
        };
        Ok(Key::new(code, mods))
    }
}

impl Key {
    /// Normalized for a keymap: a character is lowercased, its case being in
    /// the SHIFT flag, and under CTRL or ALT mapped to its physical position
    /// (ADR 7). Only combinations can be layout-dependent; plain text never is.
    pub fn new(code: KeyCode, mods: Mods) -> Self {
        let code = match code {
            KeyCode::Char(c) if mods.intersects(Mods::CTRL | Mods::ALT) => {
                KeyCode::Char(physical_key(lowercase(c)))
            }
            KeyCode::Char(c) => KeyCode::Char(lowercase(c)),
            other => other,
        };
        Self { code, mods }
    }
}

/// What arrived from the keyboard: the key a keymap matches on, and the
/// character it inserts if it inserts one. The two are kept apart because
/// the key is normalized — lowercased, mapped to the physical position —
/// and the text never is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Keystroke {
    pub key: Key,
    pub text: Option<char>,
}

impl Keystroke {
    /// From the key as the terminal reported it. A combination with CTRL or
    /// ALT inserts nothing, whatever character it came with.
    pub fn new(code: KeyCode, mods: Mods) -> Self {
        let text = match code {
            KeyCode::Char(c) if !mods.intersects(Mods::CTRL | Mods::ALT) => Some(c),
            _ => None,
        };
        Self {
            key: Key::new(code, mods),
            text,
        }
    }
}

const CYRILLIC: &str = "йцукенгшщзхъфывапролджэячсмитьбюё";
const LATIN: &str = "qwertyuiop[]asdfghjkl;'zxcvbnm,.`";

/// Maps a character to the key at the same physical position on a US layout,
/// so that `Ctrl+ф` reaches the command bound to `Ctrl+A`.
pub fn physical_key(c: char) -> char {
    match CYRILLIC.chars().position(|k| k == c) {
        Some(i) => LATIN.chars().nth(i).unwrap_or(c),
        None => c,
    }
}

fn lowercase(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected(code: KeyCode, mods: Mods) -> Key {
        Key { code, mods }
    }

    #[test]
    fn layouts_agree_in_length() {
        assert_eq!(CYRILLIC.chars().count(), LATIN.chars().count());
    }

    #[test]
    fn combination_uses_physical_key() {
        assert_eq!(
            Key::new(KeyCode::Char('ф'), Mods::CTRL),
            expected(KeyCode::Char('a'), Mods::CTRL)
        );
    }

    #[test]
    fn plain_text_keeps_its_character() {
        let stroke = Keystroke::new(KeyCode::Char('ф'), Mods::empty());
        assert_eq!(stroke.key, expected(KeyCode::Char('ф'), Mods::empty()));
        assert_eq!(stroke.text, Some('ф'));
    }

    #[test]
    fn a_key_reads_back_from_its_own_name() {
        let codes = [
            KeyCode::Char('l'),
            KeyCode::Char('+'),
            KeyCode::Char('ф'),
            KeyCode::Char(' '),
            KeyCode::Enter,
            KeyCode::Tab,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Insert,
            KeyCode::Esc,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
            KeyCode::F(12),
        ];
        for code in codes {
            for mods in [
                Mods::empty(),
                Mods::SHIFT,
                Mods::CTRL | Mods::ALT | Mods::SHIFT,
            ] {
                let key = Key::new(code, mods);
                assert_eq!(key.to_string().parse::<Key>().unwrap(), key, "{key}");
            }
        }
    }

    #[test]
    fn space_has_a_name() {
        let key = Key::new(KeyCode::Char(' '), Mods::CTRL);
        assert_eq!(key.to_string(), "Ctrl+Space");
    }

    #[test]
    fn a_name_is_read_whatever_its_case() {
        assert_eq!(
            "ctrl+shift+l".parse::<Key>().unwrap(),
            expected(KeyCode::Char('l'), Mods::CTRL | Mods::SHIFT)
        );
        assert_eq!(
            "PAGEDOWN".parse::<Key>().unwrap(),
            expected(KeyCode::PageDown, Mods::empty())
        );
    }

    #[test]
    fn a_name_that_is_not_a_key_is_refused() {
        assert!("Ctrl+Hyper".parse::<Key>().is_err());
        assert!("Ctrl+".parse::<Key>().is_err());
        assert!("F1x".parse::<Key>().is_err());
    }

    #[test]
    fn the_key_is_normalized_and_the_text_is_not() {
        let stroke = Keystroke::new(KeyCode::Char('K'), Mods::SHIFT);
        assert_eq!(stroke.key.code, KeyCode::Char('k'));
        assert_eq!(stroke.text, Some('K'));
    }

    #[test]
    fn case_lives_in_the_shift_flag() {
        assert_eq!(
            Key::new(KeyCode::Char('K'), Mods::CTRL | Mods::SHIFT),
            expected(KeyCode::Char('k'), Mods::CTRL | Mods::SHIFT)
        );
    }

    #[test]
    fn combinations_insert_nothing() {
        let stroke = Keystroke::new(KeyCode::Char('a'), Mods::CTRL);
        assert_eq!(stroke.text, None);
    }

    #[test]
    fn a_key_prints_the_way_a_menu_would() {
        let shown = |code, mods| Key { code, mods }.to_string();
        assert_eq!(
            shown(KeyCode::Char('z'), Mods::CTRL | Mods::SHIFT),
            "Ctrl+Shift+Z"
        );
        assert_eq!(shown(KeyCode::F(3), Mods::SHIFT), "Shift+F3");
        assert_eq!(shown(KeyCode::Char('/'), Mods::CTRL), "Ctrl+/");
        assert_eq!(shown(KeyCode::Esc, Mods::empty()), "Esc");
    }
}
