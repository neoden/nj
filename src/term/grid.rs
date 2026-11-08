//! The shadow grid: a frame is composed into cells first, and only
//! what differs from the frame before is sent to the terminal. The grid is
//! also what a test looks at, since a stream of escape sequences is not
//! something one can compare with an expectation.

use std::io::Write;

use anyhow::Result;
use crossterm::style::{Attribute, Print, SetAttribute, SetBackgroundColor, SetForegroundColor};
use crossterm::{cursor, queue};

use super::theme;
use crate::editor::style::{CellStyle, Ink, Mark};
use crate::text::layout::{self, Glyph};

/// Drawn in place of a control character, which has no glyph of its own:
/// the replacement character, one cell in every font, and not a character
/// a text would hold on its own, so it cannot be mistaken for one.
pub const CONTROL_PLACEHOLDER: &str = "\u{fffd}";

/// The base a bare cluster is drawn on: the dotted circle Unicode charts
/// show an isolated combining mark on.
pub const BARE_BASE: &str = "\u{25cc}";

/// A bare cluster with its base, as one printable cluster.
pub fn based(cluster: &str) -> String {
    format!("{BARE_BASE}{cluster}")
}

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
            style: CellStyle::default(),
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
                Glyph::Bare(cluster) => self.put(x + cell.column, y, &based(cluster), 1, style),
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

fn set_style(out: &mut impl Write, style: CellStyle) -> Result<()> {
    queue!(out, SetAttribute(Attribute::Reset))?;
    let ink = match style.ink {
        // A module is plain: any colour of its own was the keyword's blue
        // or close to it, and `use` stands right before it.
        Ink::Text | Ink::Module => None,
        // Where a function is defined, a call being plain.
        Ink::Function => Some(theme::FUNCTION),
        Ink::Type => Some(theme::TYPE),
        // The number of the current line is lit, the others stay dim.
        Ink::Gutter if style.mark == Mark::CurrentLine => Some(theme::FOREGROUND),
        Ink::Gutter | Ink::Rule => Some(theme::DIM),
        Ink::Status => {
            queue!(
                out,
                SetForegroundColor(theme::BACKGROUND.into()),
                SetBackgroundColor(theme::STATUS.into())
            )?;
            None
        }
        Ink::Comment => Some(theme::COMMENT),
        // A builtin is told from a keyword in the frames, not yet on the
        // screen.
        Ink::Keyword | Ink::Builtin => Some(theme::KEYWORD),
        Ink::String => Some(theme::STRING),
        Ink::Constant => Some(theme::CONSTANT),
        // Next to `break` and `continue`, so of the keyword's family, and
        // told from it.
        Ink::Label => Some(theme::LABEL),
        // As `git diff` colours them.
        Ink::Added => Some(theme::ADDED),
        Ink::Deleted => Some(theme::DELETED),
    };
    if let Some(ink) = ink {
        queue!(out, SetForegroundColor(ink.into()))?;
    }
    match style.mark {
        Mark::None => {}
        Mark::CurrentLine => queue!(out, SetBackgroundColor(theme::CURRENT_LINE.into()))?,
        Mark::Selected => queue!(out, SetBackgroundColor(theme::SELECTION.into()))?,
        // Underlined rather than coloured, so that a hit reads the same
        // whatever the terminal's palette looks like.
        Mark::Matched => queue!(out, SetAttribute(Attribute::Underlined))?,
        // The same block on whatever it stands, a keyword or a string.
        Mark::Cursor => queue!(
            out,
            SetForegroundColor(theme::BACKGROUND.into()),
            SetBackgroundColor(theme::FOREGROUND.into())
        )?,
    }
    Ok(())
}

/// What was sent to the terminal, with the escape sequences taken out, for
/// a test to read what a frame drew.
#[cfg(test)]
pub(crate) fn visible(sent: &[u8]) -> String {
    let mut text = String::new();
    let sent = String::from_utf8_lossy(sent);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_cluster_takes_two_cells_and_reads_back_once() {
        let mut grid = Grid::new(6, 1);
        assert_eq!(grid.put_str(0, 0, "a日b", CellStyle::default()), 4);
        assert_eq!(grid.row_text(0), "a日b  ");
        assert_eq!(grid.cell(2, 0).unwrap().symbol, Symbol::Continuation);
        assert_eq!(grid.cell(3, 0).unwrap().symbol, Symbol::Char('b'));
    }

    #[test]
    fn what_does_not_fit_is_dropped() {
        let mut grid = Grid::new(3, 2);
        grid.put_str(1, 0, "abcdef", CellStyle::default());
        grid.put_str(0, 5, "off the grid", CellStyle::default());
        assert_eq!(grid.row_text(0), " ab");
        assert_eq!(grid.row_text(1), "   ");
    }

    #[test]
    fn combining_marks_stay_in_one_cell() {
        let mut grid = Grid::new(3, 1);
        grid.put_str(0, 0, "e\u{301}x", CellStyle::default());
        assert_eq!(grid.row_text(0), "e\u{301}x ");
        assert!(matches!(
            grid.cell(0, 0).unwrap().symbol,
            Symbol::Cluster(_)
        ));
    }

    fn flushed(grid: &Grid, previous: Option<&Grid>) -> String {
        let mut out = Vec::new();
        grid.flush(&mut out, previous).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn only_what_changed_is_sent() {
        let mut before = Grid::new(10, 2);
        before.put_str(0, 0, "hello", CellStyle::default());
        let mut after = before.clone();
        after.put_str(0, 0, "hallo", CellStyle::default());
        after.put_str(0, 1, "x", CellStyle::marked(Mark::Cursor));

        let sent = flushed(&after, Some(&before));
        // Two moves, one per changed run: to the `a`, and to the `x`.
        assert!(sent.contains("\x1b[1;2H"));
        assert!(sent.contains("\x1b[2;1H"));
        assert_eq!(sent.matches('H').count(), 2);
        assert_eq!(visible(sent.as_bytes()), "ax");
    }

    #[test]
    fn a_first_frame_and_a_resize_send_everything() {
        let mut grid = Grid::new(4, 1);
        grid.put_str(0, 0, "ab", CellStyle::default());
        assert!(flushed(&grid, None).contains("ab  "));

        let smaller = Grid::new(3, 1);
        assert!(flushed(&grid, Some(&smaller)).contains("ab  "));
    }

    #[test]
    fn a_style_is_set_once_per_run_of_it() {
        let mut grid = Grid::new(6, 1);
        grid.put_str(0, 0, "abc", CellStyle::marked(Mark::Selected));
        grid.put_str(3, 0, "def", CellStyle::default());
        let sent = flushed(&grid, None);
        // Reset, set once for the selection, reset once for the rest, and
        // the trailing reset.
        assert_eq!(sent.matches("\x1b[0m").count(), 3);
    }
}
