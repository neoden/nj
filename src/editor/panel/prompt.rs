use std::ops::Range;

use crate::text::movement::{self, Motion};
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
            Purpose::GotoLine => &["Line:"],
            Purpose::Search => &["Find:"],
            Purpose::Replace => &["Find:", "Replace:"],
        }
    }

    /// Whether the first field is a query that drives a search as it is typed.
    pub fn searches(self) -> bool {
        matches!(self, Purpose::Search | Purpose::Replace)
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

    pub fn insert(&mut self, inserted: &str) {
        let selection = self.selection();
        self.cursor = selection.start;
        self.text.remove(selection);
        self.text.insert(self.cursor, inserted);
        self.cursor += inserted.len();
        self.anchor = self.cursor;
    }

    /// Seeding a field selects what was put there, so that typing replaces it.
    pub fn select_all(&mut self) {
        self.anchor = 0;
        self.cursor = self.text.len();
    }

    pub fn delete(&mut self, motion: Motion) {
        if !self.selection().is_empty() {
            return self.insert("");
        }

        let target = self.target(motion);
        let range = self.cursor.min(target)..self.cursor.max(target);
        self.cursor = range.start;
        self.anchor = self.cursor;
        self.text.remove(range);
    }

    pub fn move_cursor(&mut self, motion: Motion, extend: bool) {
        self.cursor = self.target(motion);
        if !extend {
            self.anchor = self.cursor;
        }
    }

    fn target(&self, motion: Motion) -> usize {
        match motion {
            Motion::Word(direction) => movement::word(&self.text, self.cursor, direction),
            Motion::LineEdge(direction) | Motion::Document(direction) => {
                movement::line_edge(&self.text, self.cursor, direction)
            }
            Motion::Grapheme(direction) => movement::grapheme(&self.text, self.cursor, direction),
            Motion::Row(_) | Motion::Page(_) => self.cursor,
        }
    }
}

/// The strip at the bottom of the screen while it is asking something.
#[derive(Debug)]
pub struct Prompt {
    pub purpose: Purpose,
    pub fields: Vec<Field>,
    pub active: usize,
}

impl Prompt {
    pub fn new(purpose: Purpose) -> Self {
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

    /// The field with the keyboard.
    pub fn field_mut(&mut self) -> &mut Field {
        &mut self.fields[self.active]
    }

    pub fn insert(&mut self, inserted: &str) {
        self.field_mut().insert(inserted);
    }

    pub fn select_all(&mut self) {
        self.field_mut().select_all();
    }

    pub fn delete(&mut self, motion: Motion) {
        self.field_mut().delete(motion);
    }

    pub fn move_cursor(&mut self, motion: Motion, extend: bool) {
        self.field_mut().move_cursor(motion, extend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::movement::Direction;

    fn prompt(purpose: Purpose, typed: &str) -> Prompt {
        let mut prompt = Prompt::new(purpose);
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
