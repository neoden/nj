use std::ops::Range;

use crate::layout::{self, Glyph, DEFAULT_TAB_WIDTH};
use crate::movement::{self, Direction, Motion};
use crate::selection::{Selection, SelectionSet};
use crate::text::Text;

pub const CONTROL_PLACEHOLDER: char = '·';

/// Rows kept between the cursor and the edge of the screen while scrolling.
const SCROLL_MARGIN: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellStyle {
    Normal,
    Selected,
    Cursor,
}

/// Where selections and carets fall on one line, in display columns.
#[derive(Debug, Default)]
pub struct LineHighlight {
    selected: Vec<Range<usize>>,
    carets: Vec<usize>,
}

impl LineHighlight {
    fn style_at(&self, column: usize) -> CellStyle {
        if self.carets.contains(&column) {
            CellStyle::Cursor
        } else if self.selected.iter().any(|range| range.contains(&column)) {
            CellStyle::Selected
        } else {
            CellStyle::Normal
        }
    }
}

/// One row of the screen: the part of a logical line drawn on it.
///
/// Without soft wrap every line is exactly one row. With it, a long line
/// yields several rows, and only `rows_in_line` and `row_start_column` need to
/// learn about that — everything downstream already speaks rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Row {
    pub line: usize,
    /// Which row of that line this is; zero for the first.
    pub index: usize,
    /// Display column of the line where this row begins.
    pub start_column: usize,
}

/// A window onto a document: which part of it is on screen, and how wide the
/// screen is. Cursors will live here too.
#[derive(Debug)]
pub struct View {
    pub selections: SelectionSet,
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
            selections: SelectionSet::single(0),
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

    /// Rows filling the screen, top to bottom.
    pub fn rows<'a>(&'a self, text: &'a Text) -> impl Iterator<Item = Row> + 'a {
        let first = (self.top_line < text.line_count()).then(|| self.row(text, self.top_line, 0));
        std::iter::successors(first, move |row| self.next_row(text, *row)).take(self.height)
    }

    pub fn next_row(&self, text: &Text, row: Row) -> Option<Row> {
        if row.index + 1 < self.rows_in_line(text, row.line) {
            return Some(self.row(text, row.line, row.index + 1));
        }
        let line = row.line + 1;
        (line < text.line_count()).then(|| self.row(text, line, 0))
    }

    pub fn previous_row(&self, text: &Text, row: Row) -> Option<Row> {
        if row.index > 0 {
            return Some(self.row(text, row.line, row.index - 1));
        }
        let line = row.line.checked_sub(1)?;
        Some(self.row(text, line, self.rows_in_line(text, line) - 1))
    }

    /// The row an offset falls on.
    pub fn row_of(&self, text: &Text, offset: usize) -> Row {
        let line = text.line_of(offset);
        let content = text.line_str(line);
        let column =
            layout::column_of_byte(&content, offset - text.line_start(line), self.tab_width);
        let index = (0..self.rows_in_line(text, line))
            .rev()
            .find(|&index| self.row_start_column(text, line, index) <= column)
            .unwrap_or(0);
        self.row(text, line, index)
    }

    fn row(&self, text: &Text, line: usize, index: usize) -> Row {
        Row {
            line,
            index,
            start_column: self.row_start_column(text, line, index),
        }
    }

    /// Soft wrap will make this depend on the line and the width. Until then,
    /// one line is one row.
    fn rows_in_line(&self, _text: &Text, _line: usize) -> usize {
        1
    }

    /// Zero without soft wrap. Horizontal scrolling is deliberately kept out
    /// of this: it belongs to the window, not to the layout of the line.
    fn row_start_column(&self, _text: &Text, _line: usize, _index: usize) -> usize {
        0
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

    pub fn move_selections(&mut self, text: &Text, motion: Motion, extend: bool) {
        let moved = self
            .selections
            .mapped(|selection| self.move_selection(text, selection, motion, extend));
        self.selections = moved;
        self.scroll_to_cursor(text);
    }

    fn move_selection(
        &self,
        text: &Text,
        selection: Selection,
        motion: Motion,
        extend: bool,
    ) -> Selection {
        let (head, goal_column) = match motion {
            Motion::Row(direction) => self.move_by_row(text, &selection, direction),
            Motion::Grapheme(direction) => (movement::grapheme(text, selection.head, direction), None),
            Motion::LineEdge(direction) => (movement::line_edge(text, selection.head, direction), None),
            Motion::Document(direction) => (movement::document_edge(text, direction), None),
        };

        Selection {
            anchor: if extend { selection.anchor } else { head },
            head,
            goal_column,
        }
    }

    fn move_by_row(
        &self,
        text: &Text,
        selection: &Selection,
        direction: Direction,
    ) -> (usize, Option<usize>) {
        let row = self.row_of(text, selection.head);
        let content = text.line_str(row.line);
        let column = layout::column_of_byte(
            &content,
            selection.head - text.line_start(row.line),
            self.tab_width,
        );
        let goal = selection
            .goal_column
            .unwrap_or(column - row.start_column);

        let target = match direction {
            Direction::Backward => self.previous_row(text, row),
            Direction::Forward => self.next_row(text, row),
        };

        let head = match target {
            Some(target) => {
                let content = text.line_str(target.line);
                let column = target.start_column + goal;
                text.line_start(target.line)
                    + layout::byte_at_column(&content, column, self.tab_width)
            }
            None => movement::document_edge(text, direction),
        };
        (head, Some(goal))
    }

    /// Brings the primary cursor back on screen, keeping a margin above and
    /// below it where the document allows.
    pub fn scroll_to_cursor(&mut self, text: &Text) {
        let head = self.selections.primary().head;
        let line = text.line_of(head);
        let margin = SCROLL_MARGIN.min(self.height.saturating_sub(1) / 2);

        if line < self.top_line + margin {
            self.top_line = line.saturating_sub(margin);
        }
        if line + margin >= self.top_line + self.height {
            self.top_line = (line + margin + 1).saturating_sub(self.height);
        }
        self.top_line = self.top_line.min(self.max_top_line(text));

        let content = text.line_str(line);
        let column = layout::column_of_byte(&content, head - text.line_start(line), self.tab_width);
        if column < self.left_column {
            self.left_column = column;
        }
        if column >= self.left_column + self.width {
            self.left_column = column + 1 - self.width;
        }
    }

    /// Selections and carets crossing `line`, in display columns. A selection
    /// running past the end of a line highlights one extra column, to show
    /// that the line break itself is selected.
    pub fn highlight_line(&self, text: &Text, line: usize) -> LineHighlight {
        let start = text.line_start(line);
        let content = text.line_str(line);
        let end = start + content.len();

        let column_of =
            |offset: usize| layout::column_of_byte(&content, offset - start, self.tab_width);

        let mut highlight = LineHighlight::default();
        for selection in self.selections.iter() {
            let range = selection.range();
            if !selection.is_empty() && range.start <= end && range.end >= start {
                let from = column_of(range.start.max(start));
                let to = match range.end > end {
                    true => layout::line_width(&content, self.tab_width) + 1,
                    false => column_of(range.end),
                };
                highlight.selected.push(from..to);
            }
            if (start..=end).contains(&selection.head) {
                highlight.carets.push(column_of(selection.head));
            }
        }
        highlight
    }

    /// Translates a click into a byte offset. Rows past the last one land on
    /// the last row, as they do in any editor.
    pub fn offset_at_screen(&self, text: &Text, row: usize, column: usize) -> usize {
        let rows: Vec<Row> = self.rows(text).collect();
        let Some(row) = rows.get(row).or(rows.last()) else {
            return 0;
        };
        let content = text.line_str(row.line);
        let column = row.start_column + self.left_column + column;
        text.line_start(row.line) + layout::byte_at_column(&content, column, self.tab_width)
    }

    /// The part of a line drawn on one row, expanded and padded to exactly
    /// `width` columns. A cluster cut by either edge becomes blanks, because
    /// half a character cannot be drawn.
    pub fn visible_runs(
        &self,
        line: &str,
        start_column: usize,
        highlight: &LineHighlight,
    ) -> Vec<(String, CellStyle)> {
        let left = start_column + self.left_column;
        let right = left + self.width;
        let mut runs = Runs::default();
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
            // A cluster spanning several columns may be cut by an edge, or be
            // half selected, so it is emitted one column at a time.
            if cell.column < left || cell.end_column() > right {
                runs.blanks(visible_start..visible_end, highlight);
            } else {
                match cell.glyph {
                    Glyph::Text(text) => runs.push(text, highlight.style_at(cell.column)),
                    Glyph::Tab => runs.blanks(visible_start..visible_end, highlight),
                    Glyph::Control => runs.push(
                        &CONTROL_PLACEHOLDER.to_string(),
                        highlight.style_at(cell.column),
                    ),
                }
            }
            column = visible_end;
        }

        runs.blanks(column..right, highlight);
        runs.finish()
    }

    pub fn visible_text(&self, line: &str, start_column: usize) -> String {
        self.visible_runs(line, start_column, &LineHighlight::default())
            .into_iter()
            .map(|(text, _)| text)
            .collect()
    }
}

#[derive(Default)]
struct Runs {
    runs: Vec<(String, CellStyle)>,
}

impl Runs {
    fn push(&mut self, text: &str, style: CellStyle) {
        match self.runs.last_mut() {
            Some((buffer, last)) if *last == style => buffer.push_str(text),
            _ => self.runs.push((text.to_owned(), style)),
        }
    }

    fn blanks(&mut self, columns: Range<usize>, highlight: &LineHighlight) {
        for column in columns {
            self.push(" ", highlight.style_at(column));
        }
    }

    fn finish(self) -> Vec<(String, CellStyle)> {
        self.runs
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
        assert_eq!(
            view.visible_text("ab", 0),
            "ab".to_owned() + &" ".repeat(view.width - 2)
        );
        assert_eq!(view.visible_text("", 0).len(), view.width);
    }

    #[test]
    fn tabs_are_expanded_in_place() {
        let mut view = view(20, 25);
        view.width = 8;
        assert_eq!(view.visible_text("a\tb", 0), "a   b   ");
    }

    #[test]
    fn horizontal_scrolling_clips_from_the_left() {
        let mut view = view(20, 25);
        view.width = 4;
        view.left_column = 2;
        assert_eq!(view.visible_text("abcdefgh", 0), "cdef");
    }

    #[test]
    fn a_row_starting_mid_line_clips_the_same_way() {
        let mut view = view(20, 25);
        view.width = 4;
        assert_eq!(view.visible_text("abcdefgh", 4), "efgh");
    }

    #[test]
    fn a_character_cut_by_an_edge_becomes_blanks() {
        let mut view = view(20, 25);
        view.width = 4;

        // The wide character straddles the right edge.
        assert_eq!(view.visible_text("abc日", 0), "abc ");

        // And now the left edge.
        view.left_column = 1;
        assert_eq!(view.visible_text("日abc", 0), " abc");
    }

    #[test]
    fn control_characters_are_replaced() {
        let mut view = view(20, 25);
        view.width = 3;
        assert_eq!(view.visible_text("a\u{7}b", 0), "a·b");
    }

    #[test]
    fn every_line_is_one_row_without_wrapping() {
        let text = Text::from_str("a\nb\nc");
        let view = view(80, 25);
        let rows: Vec<_> = view.rows(&text).collect();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].line, 1);
        assert!(rows.iter().all(|row| row.index == 0 && row.start_column == 0));
    }

    #[test]
    fn rows_fill_the_screen_and_stop_at_the_document() {
        let text = Text::from_str("a\nb\nc\nd\ne\nf\ng\nh");
        let mut view = view(80, 4);
        assert_eq!(view.rows(&text).count(), 3);

        view.top_line = 6;
        assert_eq!(view.rows(&text).count(), 2);
    }

    #[test]
    fn rows_walk_forwards_and_backwards() {
        let text = Text::from_str("a\nb\nc");
        let view = view(80, 25);

        let first = view.row_of(&text, 0);
        assert_eq!(first.line, 0);
        assert_eq!(view.previous_row(&text, first), None);

        let second = view.next_row(&text, first).unwrap();
        assert_eq!(second.line, 1);
        assert_eq!(view.previous_row(&text, second), Some(first));

        let last = view.row_of(&text, 4);
        assert_eq!(last.line, 2);
        assert_eq!(view.next_row(&text, last), None);
    }

    fn view_of(text: &Text, offset: usize) -> View {
        let mut view = view(80, 25);
        view.fit(80, 25, text.line_count());
        view.selections = SelectionSet::single(offset);
        view
    }

    #[test]
    fn vertical_movement_keeps_the_column() {
        let text = Text::from_str("hello\nworld");
        let mut view = view_of(&text, 3);

        view.move_selections(&text, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 9);

        view.move_selections(&text, Motion::Row(Direction::Backward), false);
        assert_eq!(view.selections.primary().head, 3);
    }

    #[test]
    fn a_short_line_does_not_lose_the_goal_column() {
        let text = Text::from_str("longest line\nx\nanother long line");
        let mut view = view_of(&text, 8);

        view.move_selections(&text, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 14);
        assert_eq!(view.selections.primary().goal_column, Some(8));

        view.move_selections(&text, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 15 + 8);
    }

    #[test]
    fn the_goal_column_is_measured_in_display_columns() {
        let text = Text::from_str("\tx\nyyyyy");
        let mut view = view_of(&text, 1);

        // The tab occupies columns 0..4, so `x` sits at column 4.
        view.move_selections(&text, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 3 + 4);
    }

    #[test]
    fn horizontal_movement_forgets_the_goal_column() {
        let text = Text::from_str("hello\nworld");
        let mut view = view_of(&text, 3);

        view.move_selections(&text, Motion::Row(Direction::Forward), false);
        assert!(view.selections.primary().goal_column.is_some());

        view.move_selections(&text, Motion::Grapheme(Direction::Forward), false);
        assert_eq!(view.selections.primary().goal_column, None);
    }

    #[test]
    fn vertical_movement_falls_off_into_the_document_edges() {
        let text = Text::from_str("aaaa\nbbbb");
        let mut view = view_of(&text, 7);

        view.move_selections(&text, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, text.len());

        view.move_selections(&text, Motion::Row(Direction::Backward), false);
        assert_eq!(view.selections.primary().head, 2);
    }

    #[test]
    fn extending_leaves_the_anchor_behind() {
        let text = Text::from_str("hello");
        let mut view = view_of(&text, 1);

        view.move_selections(&text, Motion::Grapheme(Direction::Forward), true);
        assert_eq!(view.selections.primary().anchor, 1);
        assert_eq!(view.selections.primary().head, 2);

        view.move_selections(&text, Motion::Grapheme(Direction::Forward), false);
        assert!(view.selections.primary().is_empty());
    }

    #[test]
    fn the_screen_follows_the_cursor() {
        let text = Text::from_str(&"line\n".repeat(100));
        let mut view = view_of(&text, 0);
        view.fit(80, 11, text.line_count());

        for _ in 0..20 {
            view.move_selections(&text, Motion::Row(Direction::Forward), false);
        }
        assert_eq!(text.line_of(view.selections.primary().head), 20);
        assert_eq!(view.top_line, 20 + SCROLL_MARGIN + 1 - view.height);

        view.move_selections(&text, Motion::Document(Direction::Backward), false);
        assert_eq!(view.top_line, 0);
    }

    #[test]
    fn a_caret_and_a_selection_style_their_columns() {
        let text = Text::from_str("hello");
        let mut view = view_of(&text, 0);
        view.width = 5;
        view.selections = SelectionSet::single(1);
        view.selections = view.selections.mapped(|selection| Selection {
            head: 3,
            ..selection
        });

        let highlight = view.highlight_line(&text, 0);
        let runs = view.visible_runs("hello", 0, &highlight);
        assert_eq!(
            runs,
            vec![
                ("h".to_owned(), CellStyle::Normal),
                ("el".to_owned(), CellStyle::Selected),
                ("l".to_owned(), CellStyle::Cursor),
                ("o".to_owned(), CellStyle::Normal),
            ]
        );
    }

    #[test]
    fn clicks_land_on_the_byte_under_them() {
        let text = Text::from_str("hello\n\tмир");
        let view = view(80, 25);

        assert_eq!(view.offset_at_screen(&text, 0, 3), 3);
        // The tab covers columns 0..4, so column 5 is the second letter.
        assert_eq!(view.offset_at_screen(&text, 1, 5), 6 + 1 + 2);
        // Clicking past the last row lands on the last row.
        assert_eq!(view.offset_at_screen(&text, 99, 0), 6);
    }
}
