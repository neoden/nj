use std::borrow::Cow;
use std::fmt;
use std::ops::Range;

use ropey::{LineType, Rope, RopeBuilder, RopeSlice};
use unicode_segmentation::{GraphemeCursor, GraphemeIncomplete};

/// Buffers hold no CRLF: line endings are normalized on load and restored on
/// save, so LF is the only break we ever have to recognize. A lone CR is left
/// where it is, being data rather than a line ending.
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
    #[cfg(test)]
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
        }
    }

    pub fn len(&self) -> usize {
        self.rope.len()
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
            .char_indices_at(self.rope.ceil_char_boundary(from))
            .find(|&(index, character)| {
                let end = index + needle.len();
                // The needle's length may land inside a character of the
                // document, which is not a match and not a valid slice.
                character == first
                    && end <= self.len()
                    && self.rope.is_char_boundary(end)
                    && self.rope.slice(index..end) == *needle
            })
            .map(|(index, _)| index)
    }

    /// The grapheme boundary after `byte`, or the end of the text. Walked over
    /// the rope's own chunks, so a step costs the cluster rather than the line.
    pub fn next_grapheme(&self, byte: usize) -> usize {
        let mut cursor = GraphemeCursor::new(byte, self.len(), true);
        let (mut chunk, mut chunk_start) = self.rope.chunk(byte);

        loop {
            match cursor.next_boundary(chunk, chunk_start) {
                Ok(Some(boundary)) => return boundary,
                Ok(None) => return self.len(),
                Err(GraphemeIncomplete::NextChunk) => {
                    let next = chunk_start + chunk.len();
                    if next >= self.len() {
                        return self.len();
                    }
                    (chunk, chunk_start) = self.rope.chunk(next);
                }
                Err(GraphemeIncomplete::PreContext(until)) => {
                    let (context, start) = self.rope.chunk(until.saturating_sub(1));
                    cursor.provide_context(context, start);
                }
                Err(_) => return byte,
            }
        }
    }

    /// The grapheme boundary before `byte`, or the start of the text.
    pub fn previous_grapheme(&self, byte: usize) -> usize {
        let mut cursor = GraphemeCursor::new(byte, self.len(), true);
        let (mut chunk, mut chunk_start) = self.rope.chunk(byte);

        loop {
            match cursor.prev_boundary(chunk, chunk_start) {
                Ok(Some(boundary)) => return boundary,
                Ok(None) => return 0,
                Err(GraphemeIncomplete::PrevChunk) => {
                    if chunk_start == 0 {
                        return 0;
                    }
                    (chunk, chunk_start) = self.rope.chunk(chunk_start - 1);
                }
                Err(GraphemeIncomplete::PreContext(until)) => {
                    let (context, start) = self.rope.chunk(until.saturating_sub(1));
                    cursor.provide_context(context, start);
                }
                Err(_) => return byte,
            }
        }
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

/// Grows a `Text` a chunk at a time, so a file need not exist as a string
/// before it becomes a rope.
#[derive(Default)]
pub struct TextBuilder {
    builder: RopeBuilder,
}

impl TextBuilder {
    pub fn append(&mut self, chunk: &str) {
        self.builder.append(chunk);
    }

    pub fn finish(self) -> Text {
        Text {
            rope: self.builder.finish(),
        }
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
    fn finding_reports_byte_offsets() {
        let text = Text::from_str("one two one");
        assert_eq!(text.find("one", 0), Some(0));
        assert_eq!(text.find("one", 1), Some(8));
        assert_eq!(text.find("three", 0), None);
    }

    #[test]
    fn a_needle_never_cuts_a_character_in_half() {
        // The needle is two bytes long and `ф` starts one byte in.
        let text = Text::from_str("aфb");
        assert_eq!(text.find("ab", 0), None);

        let text = Text::from_str("= '·'");
        assert_eq!(text.find("''", 0), None);
        assert_eq!(text.find("'·'", 0), Some(2));
    }

    #[test]
    fn finding_works_around_multibyte_text() {
        let text = Text::from_str("привет мир привет");
        assert_eq!(text.find("мир", 0), Some(13));
        assert_eq!(text.find("привет", 1), Some(20));
    }

    #[test]
    fn graphemes_step_over_whole_clusters() {
        let text = Text::from_str("да");
        assert_eq!(text.next_grapheme(0), 2);
        assert_eq!(text.previous_grapheme(2), 0);

        let text = Text::from_str("e\u{301}x");
        assert_eq!(text.next_grapheme(0), 3);
        assert_eq!(text.previous_grapheme(3), 0);
    }

    #[test]
    fn graphemes_stop_at_the_edges() {
        let text = Text::from_str("ab");
        assert_eq!(text.previous_grapheme(0), 0);
        assert_eq!(text.next_grapheme(2), 2);
        assert_eq!(Text::default().next_grapheme(0), 0);
    }

    #[test]
    fn a_line_longer_than_a_chunk_walks_one_cluster_at_a_time() {
        // Long enough to span many of the rope's chunks, with a cluster that
        // straddles their boundaries — which is what a cursor seeing only one
        // chunk at a time would get wrong.
        let text = Text::from_str(&"e\u{301}".repeat(20_000));
        assert_eq!(text.len(), 60_000);

        assert_eq!(text.next_grapheme(0), 3);
        assert_eq!(text.previous_grapheme(text.len()), text.len() - 3);

        let mut at = 0;
        let mut forwards = 0;
        while at < text.len() {
            at = text.next_grapheme(at);
            forwards += 1;
        }
        assert_eq!(forwards, 20_000);

        let mut backwards = 0;
        while at > 0 {
            at = text.previous_grapheme(at);
            backwards += 1;
        }
        assert_eq!(backwards, 20_000);
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
