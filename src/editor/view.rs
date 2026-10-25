use std::ops::Range;

use super::screen::Area;
use super::style::CellStyle;
use super::workspace::BufferId;
use crate::buffer::Buffer;
use crate::text::anchor::{Anchor, Bias, Version};
use crate::text::layout;
use crate::text::movement::{self, Direction, Motion};
use crate::text::selection::{Selection, SelectionSet};

/// Rows kept between the cursor and the edge of the screen while scrolling.
const SCROLL_MARGIN: usize = 2;

/// Where selections, carets and search hits fall on one line, in display
/// columns. A cell under several of them takes the topmost.
#[derive(Debug, Default)]
pub struct LineHighlight {
    matched: Vec<Range<usize>>,
    selected: Vec<Range<usize>>,
    carets: Vec<usize>,
    /// The line a lone bare cursor sits on.
    current: bool,
}

impl LineHighlight {
    /// For a panel's field: one caret and what it has selected.
    pub fn field(caret: usize, selected: Range<usize>) -> Self {
        Self {
            carets: vec![caret],
            selected: (!selected.is_empty())
                .then_some(selected)
                .into_iter()
                .collect(),
            ..Self::default()
        }
    }

    pub fn is_current(&self) -> bool {
        self.current
    }

    pub fn style_at(&self, column: usize) -> CellStyle {
        if self.carets.contains(&column) {
            CellStyle::Cursor
        } else if self.selected.iter().any(|range| range.contains(&column)) {
            CellStyle::Selected
        } else if self.matched.iter().any(|range| range.contains(&column)) {
            CellStyle::Matched
        } else if self.current {
            CellStyle::CurrentLine
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

/// A window onto a buffer: the cursors in it, which part of it is on screen,
/// and the area of the screen it was given. Whatever is derived from the
/// buffer — the tab width, say — is read from the buffer, not kept here.
#[derive(Debug)]
pub struct View {
    pub buffer: BufferId,
    pub selections: SelectionSet,
    /// The buffer version the selections are positions in. The view in focus
    /// is kept current by the editor; any other is brought up to date when it
    /// is next looked at (ADR 18).
    seen: Version,
    pub top_line: usize,
    pub left_column: usize,
    /// Assigned from outside; the gutter is carved out of it.
    pub area: Area,
    pub gutter: usize,
}

/// The gutter holds the line number and a space after it.
pub fn gutter_width(line_count: usize) -> usize {
    digits(line_count) + 1
}

impl View {
    /// Onto `buffer`, at its current version. The gutter is sized from the
    /// start, so a frame composed before the first `resize` still has one.
    pub fn new(id: BufferId, buffer: &Buffer) -> Self {
        Self {
            buffer: id,
            selections: SelectionSet::single(0),
            seen: buffer.version(),
            top_line: 0,
            left_column: 0,
            area: Area::default(),
            gutter: gutter_width(buffer.text().line_count()),
        }
    }

    /// Carries the selections forward through whatever was done to the
    /// buffer since they were last placed. A view left behind for longer than
    /// the journal reaches loses its place and starts over at the top.
    pub fn catch_up(&mut self, buffer: &Buffer) {
        if self.seen == buffer.version() {
            return;
        }
        let resolve = |offset: usize| {
            let anchor = Anchor {
                version: self.seen,
                offset,
                bias: Bias::After,
            };
            buffer.resolve(anchor).unwrap_or(0)
        };
        self.selections = self.selections.mapped(|selection| Selection {
            anchor: resolve(selection.anchor),
            head: resolve(selection.head),
            goal_column: None,
        });
        self.seen = buffer.version();
    }

    /// The selections were just placed in the buffer's current text, by
    /// whoever changed it; there is nothing to catch up on.
    pub fn mark_current(&mut self, buffer: &Buffer) {
        self.seen = buffer.version();
    }

    /// A new area means a new window onto the text, so what is scrolled
    /// into it is worked out again here rather than by whoever resized: a
    /// smaller screen must still show the cursor, a larger one must not
    /// keep a top line that leaves the bottom empty. Called every frame, so
    /// an unchanged area changes nothing, or a wheel scroll would be undone.
    pub fn resize(&mut self, area: Area, buffer: &Buffer) {
        let gutter = gutter_width(buffer.text().line_count());
        if area == self.area && gutter == self.gutter {
            return;
        }
        self.area = area;
        self.gutter = gutter;
        self.scroll_to_cursor(buffer);
    }

    /// Columns of text, excluding the gutter.
    pub fn width(&self) -> usize {
        self.area.width.saturating_sub(self.gutter)
    }

    /// Rows of text.
    pub fn height(&self) -> usize {
        self.area.height
    }

    /// Rows filling the screen, top to bottom.
    pub fn rows<'a>(&'a self, buffer: &'a Buffer) -> impl Iterator<Item = Row> + 'a {
        let text = buffer.text();
        let first = (self.top_line < text.line_count()).then(|| self.row(buffer, self.top_line, 0));
        std::iter::successors(first, move |row| self.next_row(buffer, *row)).take(self.height())
    }

    pub fn next_row(&self, buffer: &Buffer, row: Row) -> Option<Row> {
        if row.index + 1 < self.rows_in_line(buffer, row.line) {
            return Some(self.row(buffer, row.line, row.index + 1));
        }
        let line = row.line + 1;
        (line < buffer.text().line_count()).then(|| self.row(buffer, line, 0))
    }

    pub fn previous_row(&self, buffer: &Buffer, row: Row) -> Option<Row> {
        if row.index > 0 {
            return Some(self.row(buffer, row.line, row.index - 1));
        }
        let line = row.line.checked_sub(1)?;
        Some(self.row(buffer, line, self.rows_in_line(buffer, line) - 1))
    }

    /// The row an offset falls on.
    pub fn row_of(&self, buffer: &Buffer, offset: usize) -> Row {
        let text = buffer.text();
        let line = text.line_of(offset);
        let content = text.line_str(line);
        let column =
            layout::column_of_byte(&content, offset - text.line_start(line), buffer.tab_width());
        let index = (0..self.rows_in_line(buffer, line))
            .rev()
            .find(|&index| self.row_start_column(buffer, line, index) <= column)
            .unwrap_or(0);
        self.row(buffer, line, index)
    }

    fn row(&self, buffer: &Buffer, line: usize, index: usize) -> Row {
        Row {
            line,
            index,
            start_column: self.row_start_column(buffer, line, index),
        }
    }

    /// Soft wrap will make this depend on the line and the width. Until then,
    /// one line is one row.
    fn rows_in_line(&self, _buffer: &Buffer, _line: usize) -> usize {
        1
    }

    /// Zero without soft wrap. Horizontal scrolling is deliberately kept out
    /// of this: it belongs to the window, not to the layout of the line.
    fn row_start_column(&self, _buffer: &Buffer, _line: usize, _index: usize) -> usize {
        0
    }

    /// Scrolling stops with the last line at the bottom of the screen; there
    /// is no empty space past the end of the document.
    pub fn max_top_line(&self, buffer: &Buffer) -> usize {
        buffer.text().line_count().saturating_sub(self.height())
    }

    pub fn scroll_lines(&mut self, delta: isize, buffer: &Buffer) {
        self.top_line = self
            .top_line
            .saturating_add_signed(delta)
            .min(self.max_top_line(buffer));
    }

    pub fn move_selections(&mut self, buffer: &Buffer, motion: Motion, extend: bool) {
        let moved = self
            .selections
            .mapped(|selection| self.move_selection(buffer, selection, motion, extend));
        self.selections = moved;
        self.scroll_to_cursor(buffer);
    }

    fn move_selection(
        &self,
        buffer: &Buffer,
        selection: Selection,
        motion: Motion,
        extend: bool,
    ) -> Selection {
        let text = buffer.text();
        let (head, goal_column) = match motion {
            Motion::Row(direction) => self.move_by_rows(buffer, &selection, direction, 1),
            Motion::Page(direction) => {
                let rows = self.height().saturating_sub(1).max(1);
                self.move_by_rows(buffer, &selection, direction, rows)
            }
            Motion::Grapheme(direction) => {
                (movement::grapheme(text, selection.head, direction), None)
            }
            Motion::Word(direction) => (movement::word(text, selection.head, direction), None),
            Motion::LineEdge(direction) => {
                (movement::line_edge(text, selection.head, direction), None)
            }
            Motion::Document(direction) => (movement::document_edge(text, direction), None),
        };

        Selection {
            anchor: if extend { selection.anchor } else { head },
            head,
            goal_column,
        }
    }

    fn move_by_rows(
        &self,
        buffer: &Buffer,
        selection: &Selection,
        direction: Direction,
        rows: usize,
    ) -> (usize, Option<usize>) {
        let text = buffer.text();
        let row = self.row_of(buffer, selection.head);
        let content = text.line_str(row.line);
        let column = layout::column_of_byte(
            &content,
            selection.head - text.line_start(row.line),
            buffer.tab_width(),
        );
        let goal = selection.goal_column.unwrap_or(column - row.start_column);

        // Running out of rows on the way stops at the last one reached, in
        // the goal column: a page from near the end lands on the last line.
        // Starting with no row to go to at all — Down on the last line —
        // goes to the edge of the text instead, as the key has to do
        // something.
        let mut target = row;
        for _ in 0..rows {
            let next = match direction {
                Direction::Backward => self.previous_row(buffer, target),
                Direction::Forward => self.next_row(buffer, target),
            };
            match next {
                Some(next) => target = next,
                None if target == row => {
                    return (movement::document_edge(text, direction), Some(goal));
                }
                None => break,
            }
        }

        let content = text.line_str(target.line);
        let head = text.line_start(target.line)
            + layout::byte_at_column(&content, target.start_column + goal, buffer.tab_width());
        (head, Some(goal))
    }

    /// Grows the column of cursors from whichever end is being extended.
    pub fn add_cursor(&mut self, buffer: &Buffer, direction: Direction) {
        let text = buffer.text();
        let edge = match direction {
            Direction::Backward => self.selections.iter().next(),
            Direction::Forward => self.selections.iter().last(),
        };
        let Some(&edge) = edge else {
            return;
        };

        let row = self.row_of(buffer, edge.head);
        let target = match direction {
            Direction::Backward => self.previous_row(buffer, row),
            Direction::Forward => self.next_row(buffer, row),
        };
        let Some(target) = target else {
            return;
        };

        let content = text.line_str(row.line);
        let column = layout::column_of_byte(
            &content,
            edge.head - text.line_start(row.line),
            buffer.tab_width(),
        );
        let goal = edge.goal_column.unwrap_or(column - row.start_column);

        let content = text.line_str(target.line);
        let head = text.line_start(target.line)
            + layout::byte_at_column(&content, target.start_column + goal, buffer.tab_width());

        self.selections.push(Selection {
            anchor: head,
            head,
            goal_column: Some(goal),
        });
        self.scroll_to_cursor(buffer);
    }

    /// Brings the primary cursor back on screen, keeping a margin above and
    /// below it where the document allows.
    pub fn scroll_to_cursor(&mut self, buffer: &Buffer) {
        let text = buffer.text();
        let head = self.selections.primary().head;
        let line = text.line_of(head);
        let margin = SCROLL_MARGIN.min(self.height().saturating_sub(1) / 2);

        if line < self.top_line + margin {
            self.top_line = line.saturating_sub(margin);
        }
        if line + margin >= self.top_line + self.height() {
            self.top_line = (line + margin + 1).saturating_sub(self.height());
        }
        self.top_line = self.top_line.min(self.max_top_line(buffer));

        let content = text.line_str(line);
        let column =
            layout::column_of_byte(&content, head - text.line_start(line), buffer.tab_width());
        if column < self.left_column {
            self.left_column = column;
        }
        if column >= self.left_column + self.width() {
            self.left_column = column + 1 - self.width();
        }
    }

    /// Selections and carets crossing `line`, in display columns. A selection
    /// running past the end of a line highlights one extra column, to show
    /// that the line break itself is selected.
    pub fn highlight_line(
        &self,
        buffer: &Buffer,
        line: usize,
        matches: &[Range<usize>],
    ) -> LineHighlight {
        let text = buffer.text();
        let tab_width = buffer.tab_width();
        let start = text.line_start(line);
        let content = text.line_str(line);
        let end = start + content.len();

        let column_of = |offset: usize| layout::column_of_byte(&content, offset - start, tab_width);

        let cursor = self.selections.primary();
        let mut highlight = LineHighlight {
            current: self.selections.len() == 1
                && cursor.is_empty()
                && text.line_of(cursor.head) == line,
            ..LineHighlight::default()
        };
        // Sorted and disjoint, so the hits on this line are a run.
        let first = matches.partition_point(|found| found.end < start);
        for found in matches[first..]
            .iter()
            .take_while(|found| found.start <= end)
        {
            let from = column_of(found.start.max(start));
            let to = column_of(found.end.min(end));
            highlight.matched.push(from..to);
        }

        for selection in self.selections.touching(start..end) {
            let range = selection.range();
            if !selection.is_empty() {
                let from = column_of(range.start.max(start));
                let to = match range.end > end {
                    true => layout::line_width(&content, tab_width) + 1,
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
    pub fn offset_at_screen(&self, buffer: &Buffer, row: usize, column: usize) -> usize {
        let text = buffer.text();
        let rows: Vec<Row> = self.rows(buffer).collect();
        let Some(row) = rows.get(row).or(rows.last()) else {
            return 0;
        };
        let content = text.line_str(row.line);
        let column = row.start_column + self.left_column + column;
        text.line_start(row.line) + layout::byte_at_column(&content, column, buffer.tab_width())
    }
}

fn digits(n: usize) -> usize {
    n.max(1).ilog10() as usize + 1
}

#[cfg(test)]
mod tests {
    use super::super::screen::Screen;
    use super::*;
    use crate::text::Text;

    fn buf(content: &str) -> Buffer {
        Buffer::from_text(Text::from_str(content))
    }

    /// The view a terminal of this size gives a one-line buffer, with no
    /// panel open: the status bar takes a row and the gutter two columns.
    fn view(width: usize, height: usize) -> View {
        let buffer = Buffer::default();
        let mut view = View::new(BufferId::FIRST, &buffer);
        fit(&mut view, width, height, &buffer);
        view
    }

    fn fit(view: &mut View, width: usize, height: usize, buffer: &Buffer) {
        let line_count = buffer.text().line_count();
        let screen = Screen::divide(width, height, gutter_width(line_count), 0);
        view.resize(screen.text, buffer);
    }

    /// Columns of text, whatever the gutter takes.
    fn set_width(view: &mut View, columns: usize) {
        view.area.width = view.gutter + columns;
    }

    #[test]
    fn gutter_grows_with_the_line_count() {
        let mut view = view(80, 24);
        fit(&mut view, 80, 24, &buf(&"x\n".repeat(8)));
        assert_eq!(view.gutter, 2);
        assert_eq!(view.width(), 78);
        assert_eq!(view.height(), 23);

        fit(&mut view, 80, 24, &buf(&"x\n".repeat(999)));
        assert_eq!(view.gutter, 5);
        assert_eq!(view.width(), 75);
    }

    /// The window changed, so what it shows is worked out again: a screen
    /// that shrank still shows the cursor, one that grew has no blank rows
    /// below the last line — and one that stayed the same leaves a wheel
    /// scroll alone.
    #[test]
    fn resizing_keeps_the_cursor_on_screen_and_the_document_on_it() {
        let buffer = buf(&"line\n".repeat(50));
        let mut view = view_of(&buffer, 0);
        view.selections = SelectionSet::single(40 * 5);
        assert_eq!(view.top_line, 0);

        fit(&mut view, 80, 10, &buffer);
        assert!(view.top_line + view.height() > 40);
        assert!(view.top_line <= 40);

        view.scroll_lines(-10, &buffer);
        let scrolled = view.top_line;
        fit(&mut view, 80, 10, &buffer);
        assert_eq!(view.top_line, scrolled);

        view.scroll_lines(99, &buffer);
        fit(&mut view, 80, 200, &buffer);
        assert_eq!(view.top_line, 0);
    }

    fn visible_lines(view: &View, buffer: &Buffer) -> Range<usize> {
        let rows: Vec<_> = view.rows(buffer).collect();
        match (rows.first(), rows.last()) {
            (Some(first), Some(last)) => first.line..last.line + 1,
            _ => 0..0,
        }
    }

    #[test]
    fn visible_lines_stop_at_the_end_of_the_document() {
        let buffer = buf("1\n2\n3\n");
        let mut view = view(80, 25);
        assert_eq!(visible_lines(&view, &buffer), 0..4);

        view.area.height = 2;
        assert_eq!(visible_lines(&view, &buffer), 0..2);

        view.top_line = 3;
        assert_eq!(visible_lines(&view, &buffer), 3..4);
    }

    #[test]
    fn scrolling_stops_with_the_last_line_at_the_bottom() {
        let buffer = buf("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
        let mut view = view(80, 5);
        assert_eq!(view.height(), 4);

        view.scroll_lines(-1, &buffer);
        assert_eq!(view.top_line, 0);

        view.scroll_lines(99, &buffer);
        assert_eq!(view.top_line, 6);
        assert_eq!(visible_lines(&view, &buffer), 6..10);
    }

    #[test]
    fn a_document_shorter_than_the_screen_does_not_scroll() {
        let buffer = buf("1\n2\n3");
        let mut view = view(80, 25);

        view.scroll_lines(99, &buffer);
        assert_eq!(view.top_line, 0);
    }

    #[test]
    fn the_screen_scrolls_sideways_for_the_cursor() {
        let buffer = buf("a very long line indeed");
        let mut view = view_of(&buffer, 0);
        set_width(&mut view, 10);

        view.move_selections(&buffer, Motion::LineEdge(Direction::Forward), false);
        assert_eq!(view.left_column, buffer.text().len() + 1 - view.width());

        view.move_selections(&buffer, Motion::LineEdge(Direction::Backward), false);
        assert_eq!(view.left_column, 0);
    }

    #[test]
    fn every_line_is_one_row_without_wrapping() {
        let buffer = buf("a\nb\nc");
        let view = view(80, 25);
        let rows: Vec<_> = view.rows(&buffer).collect();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1].line, 1);
        assert!(
            rows.iter()
                .all(|row| row.index == 0 && row.start_column == 0)
        );
    }

    #[test]
    fn rows_fill_the_screen_and_stop_at_the_document() {
        let buffer = buf("a\nb\nc\nd\ne\nf\ng\nh");
        let mut view = view(80, 4);
        assert_eq!(view.rows(&buffer).count(), 3);

        view.top_line = 6;
        assert_eq!(view.rows(&buffer).count(), 2);
    }

    #[test]
    fn rows_walk_forwards_and_backwards() {
        let buffer = buf("a\nb\nc");
        let view = view(80, 25);

        let first = view.row_of(&buffer, 0);
        assert_eq!(first.line, 0);
        assert_eq!(view.previous_row(&buffer, first), None);

        let second = view.next_row(&buffer, first).unwrap();
        assert_eq!(second.line, 1);
        assert_eq!(view.previous_row(&buffer, second), Some(first));

        let last = view.row_of(&buffer, 4);
        assert_eq!(last.line, 2);
        assert_eq!(view.next_row(&buffer, last), None);
    }

    fn view_of(buffer: &Buffer, offset: usize) -> View {
        let mut view = view(80, 25);
        fit(&mut view, 80, 25, buffer);
        view.selections = SelectionSet::single(offset);
        view
    }

    #[test]
    fn vertical_movement_keeps_the_column() {
        let buffer = buf("hello\nworld");
        let mut view = view_of(&buffer, 3);

        view.move_selections(&buffer, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 9);

        view.move_selections(&buffer, Motion::Row(Direction::Backward), false);
        assert_eq!(view.selections.primary().head, 3);
    }

    #[test]
    fn a_short_line_does_not_lose_the_goal_column() {
        let buffer = buf("longest line\nx\nanother long line");
        let mut view = view_of(&buffer, 8);

        view.move_selections(&buffer, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 14);
        assert_eq!(view.selections.primary().goal_column, Some(8));

        view.move_selections(&buffer, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 15 + 8);
    }

    #[test]
    fn the_goal_column_is_measured_in_display_columns() {
        let buffer = buf("\tx\nyyyyy");
        let mut view = view_of(&buffer, 1);

        // The tab occupies columns 0..4, so `x` sits at column 4.
        view.move_selections(&buffer, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 3 + 4);
    }

    #[test]
    fn horizontal_movement_forgets_the_goal_column() {
        let buffer = buf("hello\nworld");
        let mut view = view_of(&buffer, 3);

        view.move_selections(&buffer, Motion::Row(Direction::Forward), false);
        assert!(view.selections.primary().goal_column.is_some());

        view.move_selections(&buffer, Motion::Grapheme(Direction::Forward), false);
        assert_eq!(view.selections.primary().goal_column, None);
    }

    #[test]
    fn vertical_movement_falls_off_into_the_document_edges() {
        let buffer = buf("aaaa\nbbbb");
        let mut view = view_of(&buffer, 7);

        view.move_selections(&buffer, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, buffer.text().len());

        view.move_selections(&buffer, Motion::Row(Direction::Backward), false);
        assert_eq!(view.selections.primary().head, 2);
    }

    #[test]
    fn a_page_moves_a_screenful_of_rows_at_once() {
        let buffer = buf(&"line\n".repeat(100));
        let mut view = view_of(&buffer, 0);
        fit(&mut view, 80, 11, &buffer);

        view.move_selections(&buffer, Motion::Page(Direction::Forward), false);
        assert_eq!(
            buffer.text().line_of(view.selections.primary().head),
            view.height() - 1
        );

        view.move_selections(&buffer, Motion::Page(Direction::Backward), false);
        assert_eq!(view.selections.primary().head, 0);
    }

    /// Near the end a page lands on the last line in its column; a single
    /// row from the last line goes to the end of the text.
    #[test]
    fn a_page_short_of_rows_stops_on_the_last_line_in_its_column() {
        let buffer = buf(&("ab\n".repeat(11) + "ab"));
        // Twelve lines; line 8, column 1, in a view of five rows per page.
        let mut view = view_of(&buffer, 8 * 3 + 1);
        fit(&mut view, 80, 6, &buffer);

        view.move_selections(&buffer, Motion::Page(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, 11 * 3 + 1);

        view.move_selections(&buffer, Motion::Row(Direction::Forward), false);
        assert_eq!(view.selections.primary().head, buffer.text().len());
    }

    #[test]
    fn extending_leaves_the_anchor_behind() {
        let buffer = buf("hello");
        let mut view = view_of(&buffer, 1);

        view.move_selections(&buffer, Motion::Grapheme(Direction::Forward), true);
        assert_eq!(view.selections.primary().anchor, 1);
        assert_eq!(view.selections.primary().head, 2);

        view.move_selections(&buffer, Motion::Grapheme(Direction::Forward), false);
        assert!(view.selections.primary().is_empty());
    }

    #[test]
    fn the_screen_follows_the_cursor() {
        let buffer = buf(&"line\n".repeat(100));
        let mut view = view_of(&buffer, 0);
        fit(&mut view, 80, 11, &buffer);

        for _ in 0..20 {
            view.move_selections(&buffer, Motion::Row(Direction::Forward), false);
        }
        assert_eq!(buffer.text().line_of(view.selections.primary().head), 20);
        assert_eq!(view.top_line, 20 + SCROLL_MARGIN + 1 - view.height());

        view.move_selections(&buffer, Motion::Document(Direction::Backward), false);
        assert_eq!(view.top_line, 0);
    }

    #[test]
    fn a_caret_and_a_selection_style_their_columns() {
        let buffer = buf("hello");
        let mut view = view_of(&buffer, 0);
        view.selections = Selection {
            anchor: 1,
            head: 3,
            goal_column: None,
        }
        .into();

        let highlight = view.highlight_line(&buffer, 0, &[]);
        let styles: Vec<_> = (0..5).map(|column| highlight.style_at(column)).collect();
        assert_eq!(
            styles,
            [
                CellStyle::Normal,
                CellStyle::Selected,
                CellStyle::Selected,
                CellStyle::Cursor,
                CellStyle::Normal,
            ]
        );
    }

    #[test]
    fn the_current_line_needs_a_lone_bare_cursor() {
        let buffer = buf("one\ntwo\n");
        let mut view = view_of(&buffer, 0);

        assert!(view.highlight_line(&buffer, 0, &[]).current);
        assert!(!view.highlight_line(&buffer, 1, &[]).current);

        view.selections = Selection {
            anchor: 0,
            head: 2,
            goal_column: None,
        }
        .into();
        assert!(!view.highlight_line(&buffer, 0, &[]).current);

        view.selections = SelectionSet::single(0);
        view.selections.push(Selection::cursor(4));
        assert!(!view.highlight_line(&buffer, 0, &[]).current);
    }

    #[test]
    fn the_current_line_runs_to_the_edge_of_the_screen() {
        let buffer = buf("one\ntwo\n");
        let mut view = view_of(&buffer, 1);
        set_width(&mut view, 6);

        let highlight = view.highlight_line(&buffer, 0, &[]);
        assert!((0..view.width()).all(|column| highlight.style_at(column) != CellStyle::Normal));
    }

    #[test]
    fn clicks_land_on_the_byte_under_them() {
        let buffer = buf("hello\n\tмир");
        let view = view(80, 25);

        assert_eq!(view.offset_at_screen(&buffer, 0, 3), 3);
        // The tab covers columns 0..4, so column 5 is the second letter.
        assert_eq!(view.offset_at_screen(&buffer, 1, 5), 6 + 1 + 2);
        // Clicking past the last row lands on the last row.
        assert_eq!(view.offset_at_screen(&buffer, 99, 0), 6);
    }
}
