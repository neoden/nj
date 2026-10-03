use std::ops::Range;

use crate::text::Text;
use crate::text::movement::{self, Motion};

/// One line of input, the part the forms are built of. No transactions and
/// no undo: a form is short-lived, and `Esc` is the undo it needs.
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

    /// One line is all there is: a break in pasted text becomes a space.
    pub fn insert(&mut self, inserted: &str) {
        let inserted = inserted.replace('\n', " ");
        let selection = self.selection();
        self.cursor = selection.start;
        self.text.remove(selection);
        self.text.insert(self.cursor, &inserted);
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
            Motion::LineEdge(direction) => movement::line_edge(&self.text, self.cursor, direction),
            Motion::Document(direction) => movement::document_edge(&self.text, direction),
            Motion::Grapheme(direction) => movement::grapheme(&self.text, self.cursor, direction),
            Motion::Row(_) | Motion::Page(_) => self.cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::movement::Direction;

    fn field(typed: &str) -> Field {
        let mut field = Field::default();
        field.insert(typed);
        field
    }

    #[test]
    fn typing_lands_at_the_cursor() {
        let mut field = field("helo");
        field.move_cursor(Motion::Grapheme(Direction::Backward), false);
        field.insert("l");

        assert_eq!(field.contents(), "hello");
    }

    #[test]
    fn a_field_holds_one_line() {
        let mut field = field("two\nlines");
        assert_eq!(field.contents(), "two lines");

        // Home stops at the text; Ctrl+Home goes to the very start.
        field.move_cursor(Motion::Document(Direction::Backward), false);
        field.insert("  ");
        field.move_cursor(Motion::LineEdge(Direction::Forward), false);
        field.move_cursor(Motion::LineEdge(Direction::Backward), false);
        assert_eq!(field.cursor, 2);
        field.move_cursor(Motion::Document(Direction::Backward), false);
        assert_eq!(field.cursor, 0);
    }

    #[test]
    fn deleting_a_word_uses_the_editor_rules() {
        let mut field = field("foo.bar");
        field.delete(Motion::Word(Direction::Backward));
        assert_eq!(field.contents(), "foo.");

        field.delete(Motion::Word(Direction::Backward));
        assert_eq!(field.contents(), "foo");
    }

    #[test]
    fn multibyte_input_deletes_whole_characters() {
        let mut field = field("привет");
        field.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(field.contents(), "приве");
        assert_eq!(field.cursor, 10);
    }

    #[test]
    fn typing_replaces_the_selected_query() {
        let mut field = field("seeded");
        field.select_all();
        field.insert("n");

        assert_eq!(field.contents(), "n");
    }

    #[test]
    fn backspace_takes_the_selection_first() {
        let mut field = field("seeded");
        field.select_all();
        field.delete(Motion::Grapheme(Direction::Backward));

        assert_eq!(field.contents(), "");
    }

    #[test]
    fn shift_extends_the_selection_in_the_field() {
        let mut field = field("word");
        field.move_cursor(Motion::Grapheme(Direction::Backward), true);
        field.move_cursor(Motion::Grapheme(Direction::Backward), true);
        assert_eq!(field.selection(), 2..4);

        field.insert("k");
        assert_eq!(field.contents(), "wok");
    }
}
