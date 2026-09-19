//! The shadow grid of ADR 5: a frame is composed into cells first, and only
//! what differs from the frame before is sent to the terminal. The grid is
//! also what a test looks at, since a stream of escape sequences is not
//! something one can compare with an expectation.

use std::io::Write;

use anyhow::Result;
use crossterm::style::{Attribute, Color, Print, SetAttribute, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, queue};

use crate::editor::style::CellStyle;
use crate::text::layout::{self, Glyph};

/// Drawn in place of a control character, which has no glyph of its own.
pub const CONTROL_PLACEHOLDER: &str = "·";

/// What a cell shows. A cluster wider than one column takes the cells after
/// it as continuations, which print nothing of their own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbol {
    Char(char),
    Cluster(Box<str>),
    Continuation,
}

impl Symbol {
    fn of(cluster: &str) -> Self {
        let mut chars = cluster.chars();
        match (chars.next(), chars.next()) {
            (Some(character), None) => Symbol::Char(character),
            _ => Symbol::Cluster(cluster.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub symbol: Symbol,
    pub style: CellStyle,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: Symbol::Char(' '),
            style: CellStyle::Normal,
        }
    }
}

/// Tabs in what is put on the grid are worth one column: the text area has
/// expanded its own already, and a panel's strings are one-line prose.
const TAB_WIDTH: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); width * height],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Every cell blank, ready for the next frame.
    pub fn clear(&mut self) {
        self.cells.fill(Cell::default());
    }

    pub fn cell(&self, x: usize, y: usize) -> Option<&Cell> {
        (x < self.width && y < self.height).then(|| &self.cells[y * self.width + x])
    }

    /// Writes one cluster of `columns` width at `(x, y)`; whatever does not
    /// fit on the row is dropped, so callers need not clip.
    pub fn put(&mut self, x: usize, y: usize, cluster: &str, columns: usize, style: CellStyle) {
        if y >= self.height || x >= self.width {
            return;
        }
        let row = y * self.width;
        self.cells[row + x] = Cell {
            symbol: Symbol::of(cluster),
            style,
        };
        for continuation in x + 1..(x + columns).min(self.width) {
            self.cells[row + continuation] = Cell {
                symbol: Symbol::Continuation,
                style,
            };
        }
    }

    /// Writes a string from `(x, y)` on, cluster by cluster, and returns how
    /// many columns it took.
    pub fn put_str(&mut self, x: usize, y: usize, text: &str, style: CellStyle) -> usize {
        let mut columns = 0;
        for cell in layout::cells(text, TAB_WIDTH) {
            match cell.glyph {
                Glyph::Text(cluster) => self.put(x + cell.column, y, cluster, cell.width, style),
                Glyph::Tab => self.put(x + cell.column, y, " ", 1, style),
                Glyph::Control => self.put(x + cell.column, y, CONTROL_PLACEHOLDER, 1, style),
            }
            columns = cell.end_column();
        }
        columns
    }

    /// The text of a row, as it reads on the screen.
    pub fn row_text(&self, y: usize) -> String {
        let mut text = String::with_capacity(self.width);
        for x in 0..self.width {
            match &self.cells[y * self.width + x].symbol {
                Symbol::Char(character) => text.push(*character),
                Symbol::Cluster(cluster) => text.push_str(cluster),
                Symbol::Continuation => {}
            }
        }
        text
    }

    /// Sends what differs from `previous` to the terminal — everything, when
    /// there is no previous frame to differ from. Each changed run of a row
    /// costs one cursor move; a style is set only when it changes.
    pub fn flush(&self, out: &mut impl Write, previous: Option<&Grid>) -> Result<()> {
        let previous = previous.filter(|p| p.width == self.width && p.height == self.height);
        let mut style = None;

        for y in 0..self.height {
            let mut x = 0;
            while x < self.width {
                let cell = &self.cells[y * self.width + x];
                let same = previous.is_some_and(|p| p.cells[y * self.width + x] == *cell);
                if same || cell.symbol == Symbol::Continuation {
                    x += 1;
                    continue;
                }

                queue!(out, cursor::MoveTo(x as u16, y as u16))?;
                // The run goes on while cells keep differing; a continuation
                // is carried by the cell before it.
                while x < self.width {
                    let cell = &self.cells[y * self.width + x];
                    let same = previous.is_some_and(|p| p.cells[y * self.width + x] == *cell);
                    if same {
                        break;
                    }
                    if style != Some(cell.style) {
                        set_style(out, cell.style)?;
                        style = Some(cell.style);
                    }
                    match &cell.symbol {
                        Symbol::Char(character) => queue!(out, Print(character))?,
                        Symbol::Cluster(cluster) => queue!(out, Print(cluster))?,
                        Symbol::Continuation => {}
                    }
                    x += 1;
                }
            }
        }

        queue!(out, SetAttribute(Attribute::Reset))?;
        out.flush()?;
        Ok(())
    }
}

/// Assumes a dark terminal, which is what a theme (F12) will fix.
const CURRENT_LINE: Color = Color::AnsiValue(236);

fn set_style(out: &mut impl Write, style: CellStyle) -> Result<()> {
    queue!(out, SetAttribute(Attribute::Reset))?;
    match style {
        CellStyle::Normal => {}
        CellStyle::CurrentLine => queue!(out, SetBackgroundColor(CURRENT_LINE))?,
        CellStyle::Selected => queue!(out, SetBackgroundColor(Color::DarkGrey))?,
        // Underlined rather than coloured, so that a hit reads the same
        // whatever the terminal's palette looks like.
        CellStyle::Matched => queue!(out, SetAttribute(Attribute::Underlined))?,
        CellStyle::Cursor | CellStyle::Status => queue!(out, SetAttribute(Attribute::Reverse))?,
        CellStyle::Gutter | CellStyle::Rule => queue!(out, SetForegroundColor(Color::DarkGrey))?,
        CellStyle::GutterCurrent => queue!(
            out,
            SetForegroundColor(Color::Grey),
            SetBackgroundColor(CURRENT_LINE)
        )?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_cluster_takes_two_cells_and_reads_back_once() {
        let mut grid = Grid::new(6, 1);
        assert_eq!(grid.put_str(0, 0, "a日b", CellStyle::Normal), 4);
        assert_eq!(grid.row_text(0), "a日b  ");
        assert_eq!(grid.cell(2, 0).unwrap().symbol, Symbol::Continuation);
        assert_eq!(grid.cell(3, 0).unwrap().symbol, Symbol::Char('b'));
    }

    #[test]
    fn what_does_not_fit_is_dropped() {
        let mut grid = Grid::new(3, 2);
        grid.put_str(1, 0, "abcdef", CellStyle::Normal);
        grid.put_str(0, 5, "off the grid", CellStyle::Normal);
        assert_eq!(grid.row_text(0), " ab");
        assert_eq!(grid.row_text(1), "   ");
    }

    #[test]
    fn combining_marks_stay_in_one_cell() {
        let mut grid = Grid::new(3, 1);
        grid.put_str(0, 0, "e\u{301}x", CellStyle::Normal);
        assert_eq!(grid.row_text(0), "e\u{301}x ");
        assert!(matches!(grid.cell(0, 0).unwrap().symbol, Symbol::Cluster(_)));
    }

    fn flushed(grid: &Grid, previous: Option<&Grid>) -> String {
        let mut out = Vec::new();
        grid.flush(&mut out, previous).unwrap();
        String::from_utf8(out).unwrap()
    }

    /// What was sent, with the escape sequences taken out.
    fn visible(sent: &str) -> String {
        let mut text = String::new();
        let mut chars = sent.chars();
        while let Some(character) = chars.next() {
            if character != '\x1b' {
                text.push(character);
                continue;
            }
            // A CSI sequence runs to its final byte, a letter.
            for byte in chars.by_ref() {
                if byte.is_ascii_alphabetic() {
                    break;
                }
            }
        }
        text
    }

    #[test]
    fn only_what_changed_is_sent() {
        let mut before = Grid::new(10, 2);
        before.put_str(0, 0, "hello", CellStyle::Normal);
        let mut after = before.clone();
        after.put_str(0, 0, "hallo", CellStyle::Normal);
        after.put_str(0, 1, "x", CellStyle::Cursor);

        let sent = flushed(&after, Some(&before));
        // Two moves, one per changed run: to the `a`, and to the `x`.
        assert!(sent.contains("\x1b[1;2H"));
        assert!(sent.contains("\x1b[2;1H"));
        assert_eq!(sent.matches('H').count(), 2);
        assert_eq!(visible(&sent), "ax");
    }

    #[test]
    fn a_first_frame_and_a_resize_send_everything() {
        let mut grid = Grid::new(4, 1);
        grid.put_str(0, 0, "ab", CellStyle::Normal);
        assert!(flushed(&grid, None).contains("ab  "));

        let smaller = Grid::new(3, 1);
        assert!(flushed(&grid, Some(&smaller)).contains("ab  "));
    }

    #[test]
    fn a_style_is_set_once_per_run_of_it() {
        let mut grid = Grid::new(6, 1);
        grid.put_str(0, 0, "abc", CellStyle::Selected);
        grid.put_str(3, 0, "def", CellStyle::Normal);
        let sent = flushed(&grid, None);
        // Reset, set once for the selection, reset once for the rest, and
        // the trailing reset.
        assert_eq!(sent.matches("\x1b[0m").count(), 3);
    }
}
