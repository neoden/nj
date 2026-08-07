use std::ops::Range;

use crate::movement::{self, Direction, Motion};
use crate::selection::SelectionSet;
use crate::text::Text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    GotoLine,
    Search,
    Replace,
}

impl Purpose {
    fn labels(self) -> &'static [&'static str] {
        match self {
            Purpose::GotoLine => &["line:"],
            Purpose::Search => &["find:"],
            Purpose::Replace => &["find:", "replace:"],
        }
    }
}

/// One line of input. No transactions and no undo: a prompt is short-lived,
/// and `Esc` is the undo it needs.
#[derive(Debug, Default)]
pub struct Field {
    pub label: &'static str,
    pub text: Text,
    pub anchor: usize,
    pub cursor: usize,
}

impl Field {
    pub fn contents(&self) -> String {
        self.text.to_string()
    }

    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }

    fn replace_selection(&mut self, inserted: &str) {
        let selection = self.selection();
        self.cursor = selection.start;
        self.text.remove(selection);
        self.text.insert(self.cursor, inserted);
        self.cursor += inserted.len();
        self.anchor = self.cursor;
    }
}

/// The strip at the bottom of the screen while it is asking something.
#[derive(Debug)]
pub struct Prompt {
    pub purpose: Purpose,
    pub fields: Vec<Field>,
    pub active: usize,
    /// Where the cursors were when it opened, so that an incremental search
    /// can start over from there and `Esc` can put them back.
    pub origin: SelectionSet,
}

impl Prompt {
    pub fn new(purpose: Purpose, origin: SelectionSet) -> Self {
        Self {
            purpose,
            fields: purpose
                .labels()
                .iter()
                .map(|&label| Field {
                    label,
                    ..Field::default()
                })
                .collect(),
            active: 0,
            origin,
        }
    }

    /// Rows the strip occupies, which the text area gives up.
    pub fn height(&self) -> usize {
        self.fields.len()
    }

    pub fn contents(&self, index: usize) -> String {
        self.fields[index].contents()
    }

    pub fn focus_next_field(&mut self) {
        self.active = (self.active + 1) % self.fields.len();
    }

    pub fn insert(&mut self, inserted: &str) {
        self.fields[self.active].replace_selection(inserted);
    }

    /// Seeding a field selects what was put there, so that typing replaces it.
    pub fn select_all(&mut self) {
        let field = &mut self.fields[self.active];
        field.anchor = 0;
        field.cursor = field.text.len();
    }

    pub fn delete(&mut self, motion: Motion) {
        let field = &mut self.fields[self.active];
        if !field.selection().is_empty() {
            return field.replace_selection("");
        }

        let target = match motion {
            Motion::Word(direction) => movement::word(&field.text, field.cursor, direction),
            Motion::LineEdge(direction) => movement::line_edge(&field.text, field.cursor, direction),
            _ => movement::grapheme(
                &field.text,
                field.cursor,
                match motion {
                    Motion::Grapheme(direction) => direction,
                    _ => Direction::Backward,
                },
            ),
        };

        let range = field.cursor.min(target)..field.cursor.max(target);
        field.cursor = range.start;
        field.anchor = field.cursor;
        field.text.remove(range);
    }

    pub fn move_cursor(&mut self, motion: Motion, extend: bool) {
        let field = &mut self.fields[self.active];
        field.cursor = match motion {
            Motion::Word(direction) => movement::word(&field.text, field.cursor, direction),
            Motion::LineEdge(direction) | Motion::Document(direction) => {
                movement::line_edge(&field.text, field.cursor, direction)
            }
            Motion::Grapheme(direction) => movement::grapheme(&field.text, field.cursor, direction),
            Motion::Row(_) => field.cursor,
        };
        if !extend {
            field.anchor = field.cursor;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prompt(purpose: Purpose, typed: &str) -> Prompt {
        let mut prompt = Prompt::new(purpose, SelectionSet::single(0));
        prompt.insert(typed);
        prompt
    }

    #[test]
    fn typing_lands_at_the_cursor() {
        let mut prompt = prompt(Purpose::Search, "helo");
        prompt.move_cursor(Motion::Grapheme(Direction::Backward), false);
        prompt.insert("l");

        assert_eq!(prompt.contents(0), "hello");
    }

    #[test]
    fn deleting_a_word_uses_the_editor_rules() {
        let mut prompt = prompt(Purpose::Search, "foo.bar");
        prompt.delete(Motion::Word(Direction::Backward));
        assert_eq!(prompt.contents(0), "foo.");

        prompt.delete(Motion::Word(Direction::Backward));
        assert_eq!(prompt.contents(0), "foo");
    }

    #[test]
    fn multibyte_input_deletes_whole_characters() {
        let mut prompt = prompt(Purpose::Search, "привет");
        prompt.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(prompt.contents(0), "приве");
        assert_eq!(prompt.fields[0].cursor, 10);
    }

    #[test]
    fn typing_replaces_the_selected_query() {
        let mut prompt = prompt(Purpose::Search, "seeded");
        prompt.select_all();
        prompt.insert("n");

        assert_eq!(prompt.contents(0), "n");
    }

    #[test]
    fn backspace_takes_the_selection_first() {
        let mut prompt = prompt(Purpose::Search, "seeded");
        prompt.select_all();
        prompt.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(prompt.contents(0), "");
    }

    #[test]
    fn shift_extends_the_selection_in_the_field() {
        let mut prompt = prompt(Purpose::Search, "word");
        prompt.move_cursor(Motion::Grapheme(Direction::Backward), true);
        prompt.move_cursor(Motion::Grapheme(Direction::Backward), true);
        assert_eq!(prompt.fields[0].selection(), 2..4);

        prompt.insert("k");
        assert_eq!(prompt.contents(0), "wok");
    }

    #[test]
    fn replace_has_two_fields_that_take_turns() {
        let mut prompt = prompt(Purpose::Replace, "old");
        assert_eq!(prompt.height(), 2);

        prompt.focus_next_field();
        prompt.insert("new");
        assert_eq!(prompt.contents(0), "old");
        assert_eq!(prompt.contents(1), "new");

        prompt.focus_next_field();
        assert_eq!(prompt.active, 0);
    }
}
