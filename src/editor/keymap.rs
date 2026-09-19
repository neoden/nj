//! One table resolves every keystroke (ADR 8). A binding is a key, the
//! context it holds in, and the command it means; the first binding that
//! matches wins, so a key can mean different things in different places by
//! being listed more than once, most specific first.
//!
//! An open panel owns the keyboard (ADR 12) not by intercepting keys but by
//! this table: commands that act on the text are bound in `Context::Text`,
//! and in a panel they simply do not match.

use super::command::{Command, Edit, File, PanelCommand, Select};
use super::key::{Key, KeyCode, Keystroke, Mods};
use super::panel::picker::{self, Pick};
use super::panel::prompt::Purpose;
use super::panel::{After, Panel};
use super::Editor;
use crate::text::movement::{Direction, Motion};

/// Where a binding applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    Any,
    /// No panel is open: the keyboard belongs to the text.
    Text,
    /// The text, with more than one cursor.
    MultipleCursors,
    Panel,
    Prompt,
    Picker,
    Confirm,
}

impl Context {
    fn holds(self, editor: &Editor) -> bool {
        match self {
            Context::Any => true,
            Context::Text => editor.panel().is_none(),
            Context::MultipleCursors => {
                editor.panel().is_none() && editor.view().selections.len() > 1
            }
            Context::Panel => editor.panel().is_some(),
            Context::Prompt => matches!(editor.panel(), Some(Panel::Prompt(_))),
            Context::Picker => matches!(editor.panel(), Some(Panel::Picker(_))),
            Context::Confirm => matches!(editor.panel(), Some(Panel::Confirm(_))),
        }
    }

    /// Whether every state this context holds in is one `other` holds in
    /// too — in which case a binding in `self` listed before one in `other`
    /// for the same key leaves the latter unreachable.
    #[cfg(test)]
    fn covers(self, other: Context) -> bool {
        use Context::*;
        self == other
            || self == Any
            || (self == Text && other == MultipleCursors)
            || (self == Panel && matches!(other, Prompt | Picker | Confirm))
    }
}

#[derive(Debug)]
pub struct Binding {
    pub key: Key,
    pub context: Context,
    pub command: Command,
}

/// The key a command is reached by from the text, for the palette to show
/// beside its name. The first binding is the one listed, so a command bound
/// twice shows its preferred key.
pub fn key_for(command: &Command) -> Option<Key> {
    KEYMAP
        .iter()
        .find(|binding| {
            binding.command == *command && matches!(binding.context, Context::Text | Context::Any)
        })
        .map(|binding| binding.key)
}

/// The command a keystroke means right now. A key nothing is bound to
/// inserts its text where the keyboard is, or answers an open question.
pub fn resolve(stroke: Keystroke, editor: &Editor) -> Option<Command> {
    let bound = KEYMAP
        .iter()
        .find(|binding| binding.key == stroke.key && binding.context.holds(editor))
        .map(|binding| binding.command.clone());
    if bound.is_some() {
        return bound;
    }

    // An answer is read off the key rather than the text, so that a shifted
    // letter still answers and a layout the keymap does not know still works.
    if let (Some(Panel::Confirm(_)), KeyCode::Char(answer)) = (editor.panel(), stroke.key.code) {
        let combination = stroke.key.mods.intersects(Mods::CTRL.union(Mods::ALT));
        return (!combination).then_some(Command::Panel(PanelCommand::Answer(answer)));
    }

    stroke
        .text
        .map(|character| Command::Edit(Edit::Insert(character.to_string())))
}

const NONE: Mods = Mods::empty();
const CTRL: Mods = Mods::CTRL;
const SHIFT: Mods = Mods::SHIFT;
const CTRL_SHIFT: Mods = Mods::CTRL.union(Mods::SHIFT);
const CTRL_ALT: Mods = Mods::CTRL.union(Mods::ALT);

const fn bind(code: KeyCode, mods: Mods, context: Context, command: Command) -> Binding {
    Binding {
        key: Key { code, mods },
        context,
        command,
    }
}

/// A motion, extending the selection when `Shift` is part of the key.
const fn mv(code: KeyCode, mods: Mods, motion: Motion) -> Binding {
    bind(
        code,
        mods,
        Context::Any,
        Command::Move {
            motion,
            extend: mods.contains(Mods::SHIFT),
        },
    )
}

use Context::{Any, MultipleCursors, Panel as InPanel, Picker as InPicker, Prompt as InPrompt, Text};
use Direction::{Backward, Forward};
use KeyCode::{Backspace, Char, Delete, Down, End, Enter, Esc, Home, Left, PageDown, PageUp, Right, Tab, Up, F};
use Motion::{Document, Grapheme, LineEdge, Page, Row, Word};

pub static KEYMAP: &[Binding] = &[
    // Panels. `Esc` always means "never mind".
    bind(Esc, NONE, InPanel, Command::Panel(PanelCommand::Close)),
    bind(Enter, NONE, InPrompt, Command::Panel(PanelCommand::Submit { backwards: false })),
    bind(Enter, SHIFT, InPrompt, Command::Panel(PanelCommand::Submit { backwards: true })),
    bind(Tab, NONE, InPrompt, Command::Panel(PanelCommand::NextField)),
    bind(Enter, NONE, InPicker, Command::Panel(PanelCommand::Pick)),
    // The list grows upwards from the query, so moving up the screen means
    // moving away from the best match.
    bind(Up, NONE, InPicker, Command::Panel(PanelCommand::MoveChoice(1))),
    bind(Down, NONE, InPicker, Command::Panel(PanelCommand::MoveChoice(-1))),
    bind(PageUp, NONE, InPicker, Command::Panel(PanelCommand::MoveChoice(picker::VISIBLE as isize))),
    bind(PageDown, NONE, InPicker, Command::Panel(PanelCommand::MoveChoice(-(picker::VISIBLE as isize)))),
    // A question is answered by a bare letter, which `resolve` handles; no
    // binding is needed, and `Ctrl+Q` in particular must not match (D7).
    // The text.
    bind(Char('q'), CTRL, Text, Command::Quit),
    bind(Char('q'), CTRL_SHIFT, Text, Command::Abort),
    bind(Esc, NONE, MultipleCursors, Command::Select(Select::RemoveSecondary)),
    bind(Esc, NONE, Text, Command::Select(Select::Cancel)),
    bind(Enter, NONE, Text, Command::Edit(Edit::Newline)),
    bind(Tab, SHIFT, Text, Command::Edit(Edit::Outdent)),
    bind(Tab, NONE, Text, Command::Edit(Edit::Indent)),
    bind(Char('z'), CTRL_SHIFT, Text, Command::Edit(Edit::Redo)),
    bind(Char('y'), CTRL, Text, Command::Edit(Edit::Redo)),
    bind(Char('z'), CTRL, Text, Command::Edit(Edit::Undo)),
    bind(Char('s'), CTRL, Text, Command::File(File::Save)),
    bind(Char('s'), CTRL_SHIFT, Text, Command::Panel(PanelCommand::Prompt(Purpose::SaveAs(After::Stay)))),
    bind(Char('o'), CTRL, Text, Command::Panel(PanelCommand::Prompt(Purpose::Open))),
    bind(Char('g'), CTRL, Text, Command::Panel(PanelCommand::Prompt(Purpose::GotoLine))),
    bind(Char('f'), CTRL, Text, Command::Panel(PanelCommand::Prompt(Purpose::Search))),
    bind(Char('h'), CTRL, Text, Command::Panel(PanelCommand::Prompt(Purpose::Replace))),
    bind(Char('p'), CTRL_SHIFT, Text, Command::Panel(PanelCommand::Picker(Pick::Command))),
    bind(Char('c'), CTRL, Text, Command::Edit(Edit::Copy)),
    bind(Char('x'), CTRL, Text, Command::Edit(Edit::Cut)),
    bind(Char('v'), CTRL, Text, Command::Edit(Edit::Paste)),
    bind(Char('d'), CTRL, Text, Command::Select(Select::NextOccurrence)),
    bind(Char('l'), CTRL, Text, Command::Select(Select::Line)),
    bind(Char('/'), CTRL, Text, Command::Edit(Edit::ToggleComment)),
    bind(Up, CTRL_ALT, Text, Command::Select(Select::AddCursor(Backward))),
    bind(Down, CTRL_ALT, Text, Command::Select(Select::AddCursor(Forward))),
    bind(Up, CTRL, Text, Command::Scroll(-1)),
    bind(Down, CTRL, Text, Command::Scroll(1)),
    // Wherever the keyboard is: the field of a panel, or the text.
    bind(F(3), SHIFT, Any, Command::FindNext(Backward)),
    bind(F(3), NONE, Any, Command::FindNext(Forward)),
    bind(Char('l'), CTRL_SHIFT, Any, Command::Select(Select::AllOccurrences)),
    bind(Char('a'), CTRL, Any, Command::Select(Select::All)),
    bind(Backspace, CTRL, Any, Command::Edit(Edit::Delete(Word(Backward)))),
    bind(Delete, CTRL, Any, Command::Edit(Edit::Delete(Word(Forward)))),
    bind(Backspace, NONE, Any, Command::Edit(Edit::Delete(Grapheme(Backward)))),
    bind(Delete, NONE, Any, Command::Edit(Edit::Delete(Grapheme(Forward)))),
    mv(Left, CTRL, Word(Backward)),
    mv(Left, CTRL_SHIFT, Word(Backward)),
    mv(Right, CTRL, Word(Forward)),
    mv(Right, CTRL_SHIFT, Word(Forward)),
    mv(Left, NONE, Grapheme(Backward)),
    mv(Left, SHIFT, Grapheme(Backward)),
    mv(Right, NONE, Grapheme(Forward)),
    mv(Right, SHIFT, Grapheme(Forward)),
    mv(Up, NONE, Row(Backward)),
    mv(Up, SHIFT, Row(Backward)),
    mv(Down, NONE, Row(Forward)),
    mv(Down, SHIFT, Row(Forward)),
    mv(Home, CTRL, Document(Backward)),
    mv(Home, CTRL_SHIFT, Document(Backward)),
    mv(End, CTRL, Document(Forward)),
    mv(End, CTRL_SHIFT, Document(Forward)),
    mv(Home, NONE, LineEdge(Backward)),
    mv(Home, SHIFT, LineEdge(Backward)),
    mv(End, NONE, LineEdge(Forward)),
    mv(End, SHIFT, LineEdge(Forward)),
    mv(PageUp, NONE, Page(Backward)),
    mv(PageUp, SHIFT, Page(Backward)),
    mv(PageDown, NONE, Page(Forward)),
    mv(PageDown, SHIFT, Page(Forward)),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// First match wins, so a binding is dead if an earlier one for the same
    /// key holds wherever it does: the same context twice, or a wider one
    /// listed first. Cheap to check, and exactly the slip a new binding
    /// invites.
    #[test]
    fn no_binding_is_shadowed_by_an_earlier_one() {
        for (later, binding) in KEYMAP.iter().enumerate() {
            let shadow = KEYMAP[..later]
                .iter()
                .find(|earlier| earlier.key == binding.key && earlier.context.covers(binding.context));
            assert!(
                shadow.is_none(),
                "{:?} in {:?} is unreachable behind {:?}",
                binding.key,
                binding.context,
                shadow.unwrap()
            );
        }
    }

    #[test]
    fn contexts_cover_what_they_contain() {
        use Context::*;
        assert!(Any.covers(Text));
        assert!(Text.covers(MultipleCursors));
        assert!(!MultipleCursors.covers(Text));
        assert!(Panel.covers(Confirm));
        assert!(!Prompt.covers(Picker));
        assert!(!Text.covers(Panel));
    }
}
