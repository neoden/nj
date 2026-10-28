//! Where crossterm's events become the editor's input, and the only place
//! that knows both. Whatever the editor has no use for — releases, modifier
//! state, buttons it does not bind — is dropped here.

use crossterm::event::{
    Event, KeyCode as TermKeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};

use crate::editor::input::{Input, Mouse, MouseKind};
use crate::editor::key::{Key, KeyCode, Keystroke, Mods};

pub fn translate(event: Event) -> Option<Input> {
    Some(match event {
        Event::Key(key) => Input::Key(keystroke(&key)?),
        Event::Mouse(mouse) => Input::Mouse(self::mouse(mouse)?),
        Event::Paste(text) => Input::Paste(text),
        Event::Resize(..) => Input::Resized,
        _ => return None,
    })
}

/// `None` for a release, which no binding is interested in.
fn keystroke(event: &KeyEvent) -> Option<Keystroke> {
    if event.kind == KeyEventKind::Release {
        return None;
    }
    let mut mods = mods(event.modifiers);
    let code = match event.code {
        TermKeyCode::Char(c) => KeyCode::Char(c),
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
    Some(Keystroke::new(code, mods))
}

fn mouse(event: MouseEvent) -> Option<Mouse> {
    let kind = match event.kind {
        MouseEventKind::ScrollUp => MouseKind::ScrollUp,
        MouseEventKind::ScrollDown => MouseKind::ScrollDown,
        MouseEventKind::Down(MouseButton::Left) => MouseKind::Press,
        MouseEventKind::Drag(MouseButton::Left) => MouseKind::Drag,
        _ => return None,
    };
    Some(Mouse {
        kind,
        mods: mods(event.modifiers),
        row: event.row as usize,
        column: event.column as usize,
    })
}

/// The event a terminal would send for `key`: the other direction, for a
/// scenario that is written in key names and has to enter where real input
/// enters. A shifted letter arrives from the terminal as its
/// uppercase form with SHIFT set, and so it does here.
pub fn event(key: Key) -> Event {
    let code = match key.code {
        KeyCode::Char(c) if key.mods == Mods::SHIFT => {
            TermKeyCode::Char(c.to_uppercase().next().unwrap_or(c))
        }
        KeyCode::Char(c) => TermKeyCode::Char(c),
        KeyCode::Enter => TermKeyCode::Enter,
        KeyCode::Tab => TermKeyCode::Tab,
        KeyCode::Backspace => TermKeyCode::Backspace,
        KeyCode::Delete => TermKeyCode::Delete,
        KeyCode::Insert => TermKeyCode::Insert,
        KeyCode::Esc => TermKeyCode::Esc,
        KeyCode::Left => TermKeyCode::Left,
        KeyCode::Right => TermKeyCode::Right,
        KeyCode::Up => TermKeyCode::Up,
        KeyCode::Down => TermKeyCode::Down,
        KeyCode::Home => TermKeyCode::Home,
        KeyCode::End => TermKeyCode::End,
        KeyCode::PageUp => TermKeyCode::PageUp,
        KeyCode::PageDown => TermKeyCode::PageDown,
        KeyCode::F(n) => TermKeyCode::F(n),
    };
    let mut modifiers = KeyModifiers::NONE;
    modifiers.set(KeyModifiers::CONTROL, key.mods.contains(Mods::CTRL));
    modifiers.set(KeyModifiers::ALT, key.mods.contains(Mods::ALT));
    modifiers.set(KeyModifiers::SHIFT, key.mods.contains(Mods::SHIFT));
    Event::Key(KeyEvent::new(code, modifiers))
}

fn mods(modifiers: KeyModifiers) -> Mods {
    let mut mods = Mods::empty();
    mods.set(Mods::CTRL, modifiers.contains(KeyModifiers::CONTROL));
    mods.set(Mods::ALT, modifiers.contains(KeyModifiers::ALT));
    mods.set(Mods::SHIFT, modifiers.contains(KeyModifiers::SHIFT));
    mods
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::key::Key;

    fn key(code: TermKeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    fn pressed(event: Event) -> Option<Key> {
        match translate(event)? {
            Input::Key(stroke) => Some(stroke.key),
            _ => None,
        }
    }

    #[test]
    fn a_key_arrives_with_its_modifiers() {
        assert_eq!(
            pressed(key(
                TermKeyCode::Char('K'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )),
            Some(Key::new(KeyCode::Char('k'), Mods::CTRL | Mods::SHIFT))
        );
        assert_eq!(
            pressed(key(TermKeyCode::F(3), KeyModifiers::NONE)),
            Some(Key::new(KeyCode::F(3), Mods::empty()))
        );
    }

    #[test]
    fn a_key_sent_as_an_event_arrives_as_itself() {
        for key in [
            Key::new(KeyCode::Char('l'), Mods::CTRL | Mods::SHIFT),
            Key::new(KeyCode::Char('ф'), Mods::SHIFT),
            Key::new(KeyCode::Tab, Mods::SHIFT),
            Key::new(KeyCode::F(3), Mods::empty()),
        ] {
            assert_eq!(pressed(event(key)), Some(key), "{key}");
        }
        // A shifted letter inserts its uppercase form, as it does from a
        // terminal.
        match translate(event(Key::new(KeyCode::Char('a'), Mods::SHIFT))) {
            Some(Input::Key(stroke)) => assert_eq!(stroke.text, Some('A')),
            other => panic!("a keystroke, not {other:?}"),
        }
    }

    #[test]
    fn back_tab_is_shift_tab() {
        assert_eq!(
            pressed(key(TermKeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(Key::new(KeyCode::Tab, Mods::SHIFT))
        );
    }

    #[test]
    fn a_resize_reaches_the_loop() {
        assert_eq!(translate(Event::Resize(80, 24)), Some(Input::Resized));
    }

    #[test]
    fn a_release_is_nothing() {
        let mut event = KeyEvent::new(TermKeyCode::Char('a'), KeyModifiers::NONE);
        event.kind = KeyEventKind::Release;
        assert_eq!(translate(Event::Key(event)), None);
    }

    #[test]
    fn the_left_button_and_the_wheel_are_the_mouse_the_editor_knows() {
        let event = |kind| {
            Event::Mouse(MouseEvent {
                kind,
                column: 7,
                row: 2,
                modifiers: KeyModifiers::ALT,
            })
        };
        assert_eq!(
            translate(event(MouseEventKind::Down(MouseButton::Left))),
            Some(Input::Mouse(Mouse {
                kind: MouseKind::Press,
                mods: Mods::ALT,
                row: 2,
                column: 7,
            }))
        );
        assert!(matches!(
            translate(event(MouseEventKind::ScrollUp)),
            Some(Input::Mouse(Mouse {
                kind: MouseKind::ScrollUp,
                ..
            }))
        ));
        assert_eq!(
            translate(event(MouseEventKind::Down(MouseButton::Right))),
            None
        );
        assert_eq!(translate(event(MouseEventKind::Moved)), None);
    }
}
