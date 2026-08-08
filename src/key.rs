use bitflags::bitflags;
use crossterm::event::{KeyCode as TermKeyCode, KeyEvent, KeyEventKind, KeyModifiers};

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

impl Key {
    pub fn from_event(event: &KeyEvent) -> Option<Self> {
        if event.kind == KeyEventKind::Release {
            return None;
        }

        let mut mods = Mods::empty();
        mods.set(Mods::CTRL, event.modifiers.contains(KeyModifiers::CONTROL));
        mods.set(Mods::ALT, event.modifiers.contains(KeyModifiers::ALT));
        mods.set(Mods::SHIFT, event.modifiers.contains(KeyModifiers::SHIFT));

        let code = match event.code {
            TermKeyCode::Char(c) => {
                let c = lowercase(c);
                // Only combinations can be layout-dependent; plain text never is.
                let c = if mods.intersects(Mods::CTRL | Mods::ALT) {
                    physical_key(c)
                } else {
                    c
                };
                KeyCode::Char(c)
            }
            TermKeyCode::BackTab => {
                mods |= Mods::SHIFT;
                KeyCode::Tab
            }
            TermKeyCode::Enter => KeyCode::Enter,
            TermKeyCode::Tab => KeyCode::Tab,
            TermKeyCode::Backspace => KeyCode::Backspace,
            TermKeyCode::Delete => KeyCode::Delete,
            TermKeyCode::Insert => KeyCode::Insert,
            TermKeyCode::Esc => KeyCode::Esc,
            TermKeyCode::Left => KeyCode::Left,
            TermKeyCode::Right => KeyCode::Right,
            TermKeyCode::Up => KeyCode::Up,
            TermKeyCode::Down => KeyCode::Down,
            TermKeyCode::Home => KeyCode::Home,
            TermKeyCode::End => KeyCode::End,
            TermKeyCode::PageUp => KeyCode::PageUp,
            TermKeyCode::PageDown => KeyCode::PageDown,
            TermKeyCode::F(n) => KeyCode::F(n),
            _ => return None,
        };

        Some(Self { code, mods })
    }
}

/// The character a keystroke should insert, if any.
pub fn text_char(event: &KeyEvent) -> Option<char> {
    if event.kind == KeyEventKind::Release
        || event
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    match event.code {
        TermKeyCode::Char(c) => Some(c),
        _ => None,
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

    fn key(code: TermKeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    fn expected(code: KeyCode, mods: Mods) -> Option<Key> {
        Some(Key { code, mods })
    }

    #[test]
    fn layouts_agree_in_length() {
        assert_eq!(CYRILLIC.chars().count(), LATIN.chars().count());
    }

    #[test]
    fn combination_uses_physical_key() {
        let event = key(TermKeyCode::Char('ф'), KeyModifiers::CONTROL);
        assert_eq!(
            Key::from_event(&event),
            expected(KeyCode::Char('a'), Mods::CTRL)
        );
    }

    #[test]
    fn plain_text_keeps_its_character() {
        let event = key(TermKeyCode::Char('ф'), KeyModifiers::NONE);
        assert_eq!(
            Key::from_event(&event),
            expected(KeyCode::Char('ф'), Mods::empty())
        );
        assert_eq!(text_char(&event), Some('ф'));
    }

    #[test]
    fn case_lives_in_the_shift_flag() {
        let event = key(
            TermKeyCode::Char('K'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        );
        assert_eq!(
            Key::from_event(&event),
            expected(KeyCode::Char('k'), Mods::CTRL | Mods::SHIFT)
        );
    }

    #[test]
    fn back_tab_is_shift_tab() {
        let event = key(TermKeyCode::BackTab, KeyModifiers::SHIFT);
        assert_eq!(
            Key::from_event(&event),
            expected(KeyCode::Tab, Mods::SHIFT)
        );
    }

    #[test]
    fn combinations_insert_nothing() {
        assert_eq!(
            text_char(&key(TermKeyCode::Char('a'), KeyModifiers::CONTROL)),
            None
        );
    }
}
