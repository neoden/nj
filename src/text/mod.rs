//! A buffer's contents and positions in it: bytes for storage, graphemes
//! for movement, columns for display, versions for time — and nothing about
//! files, screens or commands.

pub mod anchor;
pub mod fuzzy;
pub mod layout;
pub mod movement;
pub mod search;
pub mod selection;
pub mod transaction;

use std::borrow::Cow;
use std::fmt;
use std::ops::Range;

use ropey::iter::Chunks;
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
    fn line(&self, line: usize) -> RopeSlice<'_> {
        let slice = self.rope.line(line, LINES);
        match slice.trailing_line_break_idx(LINES) {
            Some(idx) => slice.slice(..idx),
            None => slice,
        }
    }

    /// Bytes in a line, not counting its break.
    pub fn line_len(&self, line: usize) -> usize {
        self.line(line).len()
    }

    /// Borrows when the line lives in a single chunk, which is the common case.
    pub fn line_str(&self, line: usize) -> Cow<'_, str> {
        let slice = self.line(line);
        match slice.as_str() {
            Some(text) => Cow::Borrowed(text),
            None => Cow::Owned(slice.to_string()),
        }
    }

    /// A copy of the bytes in `range`, which must fall on character
    /// boundaries.
    pub fn substring(&self, range: Range<usize>) -> String {
        self.rope.slice(range).to_string()
    }

    pub fn char_at(&self, byte: usize) -> Option<char> {
        (byte < self.len()).then(|| self.rope.char(byte))
    }

    /// The first hit starting at or after `from`.
    pub fn find(&self, needle: &str, from: usize) -> Option<usize> {
        Scanner::new(self, needle, from)?.next()
    }

    /// The last hit starting before `before`, walked chunk by chunk from
    /// there towards the start. The mirror image of `find`.
    pub fn rfind(&self, needle: &str, before: usize) -> Option<usize> {
        if needle.is_empty() || before == 0 {
            return None;
        }

        // A hit starting before `before` may run past it, so the window
        // reaches that far — and no further, so that every hit found in it
        // starts before `before` without a check.
        let end = self
            .rope
            .floor_char_boundary((before + needle.len() - 1).min(self.len()));
        let (mut chunks, chunk_start) = self.rope.chunks_at(end);
        let mut window = match chunks.next() {
            Some(chunk) => {
                // Stepped over, so that `prev()` below yields the chunk before it.
                chunks.prev();
                chunk[..end - chunk_start].to_owned()
            }
            None => String::new(),
        };
        let mut window_start = end - window.len();

        loop {
            if let Some(at) = window.rfind(needle) {
                return Some(window_start + at);
            }
            let chunk = chunks.prev()?;

            let mut keep = (needle.len() - 1).min(window.len());
            while !window.is_char_boundary(keep) {
                keep += 1;
            }
            window.truncate(keep);
            window.insert_str(0, chunk);
            window_start -= chunk.len();
        }
    }

    /// Whether `needle` sits at `at`. The boundary check is what keeps the
    /// slice from splitting a character in half.
    pub fn matches_at(&self, at: usize, needle: &str) -> bool {
        let end = at + needle.len();
        end <= self.len() && self.rope.is_char_boundary(end) && self.rope.slice(at..end) == *needle
    }

    /// Every hit, left to right.
    pub fn matches<'a>(&'a self, needle: &'a str) -> Matches<'a> {
        Matches {
            scanner: Scanner::new(self, needle, 0),
            len: needle.len(),
        }
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

    pub fn insert(&mut self, at: usize, text: &str) {
        self.rope.insert(at, text);
    }

    pub fn remove(&mut self, range: Range<usize>) {
        self.rope.remove(range);
    }
}

/// Walks the text forward chunk by chunk, keeping a window with enough of
/// the previous chunk to catch a hit lying across the seam. UTF-8 is
/// self-synchronizing, so a hit found this way can never start or end inside
/// a character. Hits do not overlap: the next search starts where the last
/// hit ended, which is why a pattern able to match nothing is not allowed.
struct Scanner<'a> {
    chunks: Chunks<'a>,
    needle: &'a str,
    window: String,
    /// Offset of the window's first byte in the text.
    window_start: usize,
    /// Within the window: the end of the last hit, before which nothing is
    /// looked at again.
    searched: usize,
}

impl<'a> Scanner<'a> {
    /// `None` for an empty needle, or a start past the end of the text.
    fn new(text: &'a Text, needle: &'a str, from: usize) -> Option<Self> {
        if needle.is_empty() {
            return None;
        }
        let from = text.rope.ceil_char_boundary(from);
        let (mut chunks, chunk_start) = text.rope.chunks_at(from);
        let window = chunks.next()?[from - chunk_start..].to_owned();
        Some(Self {
            chunks,
            needle,
            window,
            window_start: from,
            searched: 0,
        })
    }

    fn next(&mut self) -> Option<usize> {
        loop {
            if let Some(at) = self.window[self.searched..].find(self.needle) {
                let found = self.searched + at;
                self.searched = found + self.needle.len();
                return Some(self.window_start + found);
            }
            let chunk = self.chunks.next()?;

            // Keep the tail a hit could still begin in; whatever is searched
            // past it stays searched.
            let mut cut = self.window.len().saturating_sub(self.needle.len() - 1);
            while !self.window.is_char_boundary(cut) {
                cut -= 1;
            }
            self.window.drain(..cut);
            self.window_start += cut;
            self.searched = self.searched.saturating_sub(cut);
            self.window.push_str(chunk);
        }
    }
}

/// Every hit of a literal, left to right, as a range.
pub struct Matches<'a> {
    scanner: Option<Scanner<'a>>,
    len: usize,
}

impl Iterator for Matches<'_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        let found = self.scanner.as_mut()?.next()?;
        Some(found..found + self.len)
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
    fn substrings_and_line_lengths_come_out_in_bytes() {
        let text = Text::from_str("да\nnet");
        assert_eq!(text.line_len(0), 4);
        assert_eq!(text.line_len(1), 3);
        assert_eq!(text.substring(2..6), "а\nn");

        // Long enough to lie across several chunks.
        let text = Text::from_str(&format!("{}\n{}", "д".repeat(3_000), "x"));
        assert_eq!(text.line_len(0), 6_000);
        assert_eq!(text.line_len(1), 1);
        assert_eq!(text.substring(5_998..6_002), "д\nx");
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
    fn a_match_lying_across_a_seam_is_still_found() {
        // Chunks run to about a kilobyte, so sliding the needle through this
        // range puts it on every seam within it.
        for offset in 900..1_200 {
            let mut content = "a".repeat(4_000);
            content.replace_range(offset..offset + 5, "xyzzy");
            let text = Text::from_str(&content);

            assert_eq!(text.find("xyzzy", 0), Some(offset), "at {offset}");
            assert_eq!(text.find("xyzzy", offset), Some(offset), "at {offset}");
            assert_eq!(text.find("xyzzy", offset + 1), None, "at {offset}");
        }
    }

    #[test]
    fn finding_backwards_reports_the_last_hit_before_the_limit() {
        let text = Text::from_str("one two one");
        assert_eq!(text.rfind("one", text.len()), Some(8));
        // Starting before the limit is enough; running past it is fine.
        assert_eq!(text.rfind("one", 9), Some(8));
        assert_eq!(text.rfind("one", 8), Some(0));
        assert_eq!(text.rfind("one", 0), None);
        assert_eq!(text.rfind("three", text.len()), None);
        assert_eq!(text.rfind("", 5), None);
    }

    #[test]
    fn a_hit_across_a_seam_is_found_backwards_too() {
        for offset in 900..1_200 {
            let mut content = "a".repeat(4_000);
            content.replace_range(offset..offset + 5, "xyzzy");
            let text = Text::from_str(&content);

            assert_eq!(text.rfind("xyzzy", text.len()), Some(offset), "at {offset}");
            assert_eq!(text.rfind("xyzzy", offset + 1), Some(offset), "at {offset}");
            assert_eq!(text.rfind("xyzzy", offset), None, "at {offset}");
        }
    }

    #[test]
    fn multibyte_hits_survive_the_seams_backwards() {
        for offset in (900..1_200).step_by(2) {
            let mut content = "д".repeat(2_000);
            content.replace_range(offset..offset + 6, "мир");
            let text = Text::from_str(&content);

            assert_eq!(text.rfind("мир", text.len()), Some(offset), "at {offset}");
            // A limit inside a character is still a limit.
            assert_eq!(text.rfind("мир", offset + 1), Some(offset), "at {offset}");
        }
    }

    #[test]
    fn multibyte_matches_survive_the_seams() {
        for offset in (900..1_200).step_by(2) {
            let mut content = "д".repeat(2_000);
            content.replace_range(offset..offset + 6, "мир");
            let text = Text::from_str(&content);

            assert_eq!(text.find("мир", 0), Some(offset), "at {offset}");
        }
    }

    #[test]
    fn walking_the_hits_survives_the_seams() {
        for offset in 900..1_200 {
            // Hits butting against each other on both sides of the seam, and
            // one lying across it.
            let mut content = "a".repeat(4_000);
            content.replace_range(offset - 10..offset + 5, "xyzxyzxyzxyzxyz");
            let text = Text::from_str(&content);

            let hits: Vec<_> = text.matches("xyz").collect();
            let expected: Vec<_> = (0..5).map(|i| offset - 10 + i * 3).map(|s| s..s + 3).collect();
            assert_eq!(hits, expected, "at {offset}");
            assert_eq!(text.matches("a").count(), 4_000 - 15, "at {offset}");
        }
    }

    #[test]
    fn every_hit_is_walked_once() {
        let text = Text::from_str("x . x . x");
        assert_eq!(text.matches("x").collect::<Vec<_>>(), [0..1, 4..5, 8..9]);

        // Overlapping candidates are not counted twice.
        assert_eq!(Text::from_str("aaaa").matches("aa").count(), 2);
        assert_eq!(Text::from_str("abc").matches("").count(), 0);
    }

    #[test]
    fn matching_at_a_position_never_splits_a_character() {
        let text = Text::from_str("aфb");
        assert!(text.matches_at(0, "a"));
        assert!(text.matches_at(1, "ф"));
        // Two bytes in from the start lands inside `ф`.
        assert!(!text.matches_at(0, "ab"));
        assert!(!text.matches_at(0, "aфbc"));
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
