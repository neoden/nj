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
    Row(Direction),
    LineEdge(Direction),
    Document(Direction),
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
