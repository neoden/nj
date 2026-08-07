use std::borrow::Cow;
use std::fmt;
use std::ops::Range;

use ropey::{LineType, Rope, RopeSlice};

/// Buffers never contain CR: line endings are normalized on load and restored
/// on save, so LF is the only break we ever have to recognize.
const LINES: LineType = LineType::LF;

/// The text of one buffer, addressed in byte offsets throughout.
///
/// Cloning is O(1) and shares storage, which is how background work gets a
/// stable snapshot while editing continues.
#[derive(Debug, Clone, Default)]
pub struct Text {
    rope: Rope,
}

impl Text {
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }

    pub fn len(&self) -> usize {
        self.rope.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rope.len() == 0
    }

    /// A trailing break opens a new empty last line, the way editors count.
    pub fn line_count(&self) -> usize {
        self.rope.len_lines(LINES)
    }

    /// Line contents without the trailing break.
    pub fn line(&self, line: usize) -> RopeSlice<'_> {
        let slice = self.rope.line(line, LINES);
        match slice.trailing_line_break_idx(LINES) {
            Some(idx) => slice.slice(..idx),
            None => slice,
        }
    }

    /// Borrows when the line lives in a single chunk, which is the common case.
    pub fn line_str(&self, line: usize) -> Cow<'_, str> {
        let slice = self.line(line);
        match slice.as_str() {
            Some(text) => Cow::Borrowed(text),
            None => Cow::Owned(slice.to_string()),
        }
    }

    pub fn char_at(&self, byte: usize) -> Option<char> {
        (byte < self.len()).then(|| self.rope.char(byte))
    }

    pub fn find(&self, needle: &str, from: usize) -> Option<usize> {
        let first = needle.chars().next()?;
        self.rope
            .char_indices_at(from)
            .find(|&(index, character)| {
                character == first
                    && index + needle.len() <= self.len()
                    && self.rope.slice(index..index + needle.len()) == *needle
            })
            .map(|(index, _)| index)
    }

    pub fn line_start(&self, line: usize) -> usize {
        self.rope.line_to_byte_idx(line, LINES)
    }

    pub fn line_of(&self, byte: usize) -> usize {
        self.rope.byte_to_line_idx(byte, LINES)
    }

    pub fn slice(&self, range: Range<usize>) -> RopeSlice<'_> {
        self.rope.slice(range)
    }

    pub fn insert(&mut self, at: usize, text: &str) {
        self.rope.insert(at, text);
    }

    pub fn remove(&mut self, range: Range<usize>) {
        self.rope.remove(range);
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.rope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_still_has_one_line() {
        let text = Text::default();
        assert_eq!(text.line_count(), 1);
        assert_eq!(text.line(0), "");
    }

    #[test]
    fn trailing_break_opens_a_last_empty_line() {
        assert_eq!(Text::from_str("a").line_count(), 1);
        assert_eq!(Text::from_str("a\n").line_count(), 2);
        assert_eq!(Text::from_str("a\nb").line_count(), 2);
    }

    #[test]
    fn line_excludes_the_break() {
        let text = Text::from_str("first\nsecond\n");
        assert_eq!(text.line(0), "first");
        assert_eq!(text.line(1), "second");
        assert_eq!(text.line(2), "");
    }

    #[test]
    fn offsets_map_to_lines() {
        let text = Text::from_str("aa\nbbb\n");
        assert_eq!(text.line_start(1), 3);
        assert_eq!(text.line_of(0), 0);
        assert_eq!(text.line_of(2), 0);
        assert_eq!(text.line_of(3), 1);
    }

    #[test]
    fn multibyte_offsets_are_byte_offsets() {
        let text = Text::from_str("привет\nмир");
        assert_eq!(text.line(0), "привет");
        assert_eq!(text.line_start(1), 13);
        assert_eq!(text.line_of(13), 1);
    }

    #[test]
    fn edits_apply_at_byte_offsets() {
        let mut text = Text::from_str("привет");
        text.insert(12, ", мир");
        assert_eq!(text.to_string(), "привет, мир");
        text.remove(0..12);
        assert_eq!(text.to_string(), ", мир");
    }

    #[test]
    fn snapshots_are_independent() {
        let mut text = Text::from_str("original");
        let snapshot = text.clone();
        text.insert(0, "edited ");
        assert_eq!(snapshot.to_string(), "original");
        assert_eq!(text.to_string(), "edited original");
    }
}
