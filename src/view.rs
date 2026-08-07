use std::ops::Range;

use crate::layout::{self, Glyph, DEFAULT_TAB_WIDTH};
use crate::text::Text;

pub const CONTROL_PLACEHOLDER: char = '·';

/// A window onto a document: which part of it is on screen, and how wide the
/// screen is. Cursors will live here too.
#[derive(Debug)]
pub struct View {
    pub top_line: usize,
    pub left_column: usize,
    /// Width of the text area, excluding the gutter.
    pub width: usize,
    /// Rows of text, excluding the status line.
    pub height: usize,
    pub gutter: usize,
    pub tab_width: usize,
}

impl Default for View {
    fn default() -> Self {
        Self {
            top_line: 0,
            left_column: 0,
            width: 0,
            height: 0,
            gutter: 0,
            tab_width: DEFAULT_TAB_WIDTH,
        }
    }
}

impl View {
    /// Divides the terminal into a gutter, a text area and a status line.
    pub fn fit(&mut self, term_width: usize, term_height: usize, line_count: usize) {
        self.gutter = digits(line_count) + 1;
        self.width = term_width.saturating_sub(self.gutter);
        self.height = term_height.saturating_sub(1);
    }

    pub fn visible_lines(&self, text: &Text) -> Range<usize> {
        let end = self.top_line.saturating_add(self.height).min(text.line_count());
        self.top_line..end
    }

    /// Scrolling stops with the last line at the bottom of the screen; there
    /// is no empty space past the end of the document.
    pub fn max_top_line(&self, text: &Text) -> usize {
        text.line_count().saturating_sub(self.height)
    }

    pub fn scroll_lines(&mut self, delta: isize, text: &Text) {
        self.top_line = self
            .top_line
            .saturating_add_signed(delta)
            .min(self.max_top_line(text));
    }

    /// Bounded by the widest line currently on screen, so scrolling right
    /// cannot wander into empty space.
    pub fn scroll_columns(&mut self, delta: isize, text: &Text) {
        let widest = self
            .visible_lines(text)
            .map(|line| layout::line_width(&text.line_str(line), self.tab_width))
            .max()
            .unwrap_or(0);
        let limit = widest.saturating_sub(self.width);
        self.left_column = self.left_column.saturating_add_signed(delta).min(limit);
    }

    pub fn scroll_to_top(&mut self) {
        self.top_line = 0;
        self.left_column = 0;
    }

    pub fn scroll_to_bottom(&mut self, text: &Text) {
        self.top_line = self.max_top_line(text);
        self.left_column = 0;
    }

    /// The slice of a line that is on screen, expanded and padded to exactly
    /// `width` columns. A cluster cut by either edge becomes blanks, because
    /// half a character cannot be drawn.
    pub fn visible_text(&self, line: &str) -> String {
        let left = self.left_column;
        let right = left + self.width;
        let mut out = String::with_capacity(self.width);
        let mut column = left;

        for cell in layout::cells(line, self.tab_width) {
            if cell.end_column() <= left {
                continue;
            }
            if cell.column >= right {
                break;
            }

            let visible_start = cell.column.max(left);
            let visible_end = cell.end_column().min(right);
            if cell.column < left || cell.end_column() > right {
                out.extend(std::iter::repeat_n(' ', visible_end - visible_start));
            } else {
                match cell.glyph {
                    Glyph::Text(text) => out.push_str(text),
                    Glyph::Tab => out.extend(std::iter::repeat_n(' ', cell.width)),
                    Glyph::Control => out.push(CONTROL_PLACEHOLDER),
                }
            }
            column = visible_end;
        }

        out.extend(std::iter::repeat_n(' ', right - column));
        out
    }
}

fn digits(n: usize) -> usize {
    n.max(1).ilog10() as usize + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(width: usize, height: usize) -> View {
        let mut view = View::default();
        view.fit(width, height, 1);
        view
    }

    #[test]
    fn gutter_grows_with_the_line_count() {
        let mut view = View::default();
        view.fit(80, 24, 9);
        assert_eq!(view.gutter, 2);
        assert_eq!(view.width, 78);
        assert_eq!(view.height, 23);

        view.fit(80, 24, 1000);
        assert_eq!(view.gutter, 5);
        assert_eq!(view.width, 75);
    }

    #[test]
    fn visible_lines_stop_at_the_end_of_the_document() {
        let text = Text::from_str("1\n2\n3\n");
        let mut view = view(80, 25);
        assert_eq!(view.visible_lines(&text), 0..4);

        view.height = 2;
        assert_eq!(view.visible_lines(&text), 0..2);

        view.top_line = 3;
        assert_eq!(view.visible_lines(&text), 3..4);
    }

    #[test]
    fn scrolling_stops_with_the_last_line_at_the_bottom() {
        let text = Text::from_str("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
        let mut view = view(80, 5);
        assert_eq!(view.height, 4);

        view.scroll_lines(-1, &text);
        assert_eq!(view.top_line, 0);

        view.scroll_lines(99, &text);
        assert_eq!(view.top_line, 6);
        assert_eq!(view.visible_lines(&text), 6..10);

        view.scroll_to_bottom(&text);
        assert_eq!(view.top_line, 6);
    }

    #[test]
    fn a_document_shorter_than_the_screen_does_not_scroll() {
        let text = Text::from_str("1\n2\n3");
        let mut view = view(80, 25);

        view.scroll_lines(99, &text);
        assert_eq!(view.top_line, 0);
    }

    #[test]
    fn horizontal_scrolling_stops_at_the_widest_visible_line() {
        let text = Text::from_str("short\nmuch longer line");
        let mut view = view(10, 25);

        view.scroll_columns(99, &text);
        assert_eq!(view.left_column, 16 - view.width);

        view.scroll_columns(-99, &text);
        assert_eq!(view.left_column, 0);
    }

    #[test]
    fn text_is_padded_to_the_full_width() {
        let view = view(10, 25);
        assert_eq!(view.visible_text("ab"), "ab".to_owned() + &" ".repeat(view.width - 2));
        assert_eq!(view.visible_text("").len(), view.width);
    }

    #[test]
    fn tabs_are_expanded_in_place() {
        let mut view = view(20, 25);
        view.width = 8;
        assert_eq!(view.visible_text("a\tb"), "a   b   ");
    }

    #[test]
    fn horizontal_scrolling_clips_from_the_left() {
        let mut view = view(20, 25);
        view.width = 4;
        view.left_column = 2;
        assert_eq!(view.visible_text("abcdefgh"), "cdef");
    }

    #[test]
    fn a_character_cut_by_an_edge_becomes_blanks() {
        let mut view = view(20, 25);
        view.width = 4;

        // The wide character straddles the right edge.
        assert_eq!(view.visible_text("abc日"), "abc ");

        // And now the left edge.
        view.left_column = 1;
        assert_eq!(view.visible_text("日abc"), " abc");
    }

    #[test]
    fn control_characters_are_replaced() {
        let mut view = view(20, 25);
        view.width = 3;
        assert_eq!(view.visible_text("a\u{7}b"), "a·b");
    }
}
