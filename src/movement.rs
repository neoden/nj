use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

use crate::text::Text;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Backward,
    Forward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Grapheme(Direction),
    Word(Direction),
    Row(Direction),
    LineEdge(Direction),
    Document(Direction),
}

/// Word movement stops wherever the class changes, so `foo.bar` is three
/// stops rather than one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Whitespace,
    Word,
    Punctuation,
}

impl Class {
    pub fn of(character: char) -> Self {
        if character.is_whitespace() {
            Class::Whitespace
        } else if character.is_alphanumeric() || character == '_' {
            Class::Word
        } else {
            Class::Punctuation
        }
    }

    fn at(text: &Text, offset: usize) -> Option<Self> {
        text.char_at(offset).map(Class::of)
    }
}

pub fn grapheme(text: &Text, offset: usize, direction: Direction) -> usize {
    let line = text.line_of(offset);
    let start = text.line_start(line);
    let content = text.line_str(line);
    let local = offset - start;

    match direction {
        Direction::Forward => match next_boundary(&content, local) {
            Some(boundary) => start + boundary,
            None if line + 1 < text.line_count() => text.line_start(line + 1),
            None => offset,
        },
        Direction::Backward => match previous_boundary(&content, local) {
            Some(boundary) => start + boundary,
            None if line > 0 => line_end(text, line - 1),
            None => offset,
        },
    }
}

/// Forward lands after the end of a word, backward on its start — the
/// asymmetry every editor with this binding has.
pub fn word(text: &Text, offset: usize, direction: Direction) -> usize {
    let mut offset = offset;

    match direction {
        Direction::Forward => {
            while Class::at(text, offset) == Some(Class::Whitespace) {
                match step(text, offset, direction) {
                    Some(next) => offset = next,
                    None => return offset,
                }
            }
            let Some(class) = Class::at(text, offset) else {
                return offset;
            };
            while Class::at(text, offset) == Some(class) {
                match step(text, offset, direction) {
                    Some(next) => offset = next,
                    None => return offset,
                }
            }
        }
        Direction::Backward => {
            while let Some(previous) = step(text, offset, direction) {
                if Class::at(text, previous) != Some(Class::Whitespace) {
                    break;
                }
                offset = previous;
            }
            let Some(class) = step(text, offset, direction).and_then(|p| Class::at(text, p)) else {
                return offset;
            };
            while let Some(previous) = step(text, offset, direction) {
                if Class::at(text, previous) != Some(class) {
                    break;
                }
                offset = previous;
            }
        }
    }

    offset
}

/// The word an offset sits in or touches. A cursor resting on whitespace
/// takes the word behind it, which is where it usually stands after typing.
pub fn word_around(text: &Text, offset: usize) -> Range<usize> {
    let solid = |class: Option<Class>| class.filter(|&class| class != Class::Whitespace);
    let class = solid(Class::at(text, offset)).or_else(|| {
        solid(step(text, offset, Direction::Backward).and_then(|previous| Class::at(text, previous)))
    });
    let Some(class) = class else {
        return offset..offset;
    };

    let mut start = offset;
    while let Some(previous) = step(text, start, Direction::Backward) {
        if Class::at(text, previous) != Some(class) {
            break;
        }
        start = previous;
    }

    let mut end = offset;
    while Class::at(text, end) == Some(class) {
        match step(text, end, Direction::Forward) {
            Some(next) => end = next,
            None => break,
        }
    }

    start..end
}

fn step(text: &Text, offset: usize, direction: Direction) -> Option<usize> {
    let next = grapheme(text, offset, direction);
    (next != offset).then_some(next)
}

pub fn line_edge(text: &Text, offset: usize, direction: Direction) -> usize {
    let line = text.line_of(offset);
    match direction {
        Direction::Backward => text.line_start(line),
        Direction::Forward => line_end(text, line),
    }
}

pub fn document_edge(text: &Text, direction: Direction) -> usize {
    match direction {
        Direction::Backward => 0,
        Direction::Forward => text.len(),
    }
}

fn line_end(text: &Text, line: usize) -> usize {
    text.line_start(line) + text.line_str(line).len()
}

fn next_boundary(text: &str, offset: usize) -> Option<usize> {
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .find(|&index| index > offset)
        .or_else(|| (offset < text.len()).then_some(text.len()))
}

fn previous_boundary(text: &str, offset: usize) -> Option<usize> {
    text.grapheme_indices(true)
        .map(|(index, _)| index)
        .take_while(|&index| index < offset)
        .last()
}

#[cfg(test)]
mod tests {
    use super::Direction::*;
    use super::*;

    #[test]
    fn horizontal_movement_steps_over_whole_characters() {
        let text = Text::from_str("да");
        assert_eq!(grapheme(&text, 0, Forward), 2);
        assert_eq!(grapheme(&text, 2, Forward), 4);
        assert_eq!(grapheme(&text, 4, Backward), 2);
    }

    #[test]
    fn a_combining_mark_travels_with_its_base() {
        let text = Text::from_str("e\u{301}x");
        assert_eq!(grapheme(&text, 0, Forward), 3);
        assert_eq!(grapheme(&text, 3, Backward), 0);
    }

    #[test]
    fn movement_crosses_line_boundaries() {
        let text = Text::from_str("ab\ncd");
        assert_eq!(grapheme(&text, 2, Forward), 3);
        assert_eq!(grapheme(&text, 3, Backward), 2);
    }

    #[test]
    fn movement_stops_at_the_document_edges() {
        let text = Text::from_str("ab");
        assert_eq!(grapheme(&text, 0, Backward), 0);
        assert_eq!(grapheme(&text, 2, Forward), 2);
    }

    #[test]
    fn words_end_forwards_and_start_backwards() {
        let text = Text::from_str("hello world");
        assert_eq!(word(&text, 0, Forward), 5);
        assert_eq!(word(&text, 5, Forward), 11);
        assert_eq!(word(&text, 11, Backward), 6);
        assert_eq!(word(&text, 6, Backward), 0);
    }

    #[test]
    fn punctuation_is_a_word_of_its_own() {
        let text = Text::from_str("foo.bar");
        assert_eq!(word(&text, 0, Forward), 3);
        assert_eq!(word(&text, 3, Forward), 4);
        assert_eq!(word(&text, 4, Forward), 7);
        assert_eq!(word(&text, 7, Backward), 4);
        assert_eq!(word(&text, 4, Backward), 3);
    }

    #[test]
    fn identifiers_stay_whole() {
        let text = Text::from_str("foo_bar42 x");
        assert_eq!(word(&text, 0, Forward), 9);
    }

    #[test]
    fn words_are_not_only_latin() {
        let text = Text::from_str("привет мир");
        assert_eq!(word(&text, 0, Forward), 12);
        assert_eq!(word(&text, 12, Forward), 19);
        assert_eq!(word(&text, 19, Backward), 13);
    }

    #[test]
    fn line_breaks_are_whitespace() {
        let text = Text::from_str("one\n\ntwo");
        assert_eq!(word(&text, 3, Forward), 8);
        assert_eq!(word(&text, 8, Backward), 5);
    }

    #[test]
    fn words_stop_at_the_document_edges() {
        let text = Text::from_str("  x  ");
        assert_eq!(word(&text, 5, Forward), 5);
        assert_eq!(word(&text, 0, Backward), 0);
        assert_eq!(word(&text, 5, Backward), 2);
    }

    #[test]
    fn line_edges() {
        let text = Text::from_str("ab\ncdef\n");
        assert_eq!(line_edge(&text, 5, Backward), 3);
        assert_eq!(line_edge(&text, 5, Forward), 7);
    }

    #[test]
    fn document_edges() {
        let text = Text::from_str("ab\ncdef\n");
        assert_eq!(document_edge(&text, Backward), 0);
        assert_eq!(document_edge(&text, Forward), 8);
    }
}
