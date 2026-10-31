//! The screen as text, for whoever drives the editor from outside: the
//! scenario tests, the replay tool, an AI at the sandbox. Two forms.
//!
//! The whole screen, as the socket answers `screen`, with nothing left out
//! and nothing remembered between calls:
//!
//! ```text
//! screen 40 16
//!  0 text|1 fn main() {}
//!  0 mark|--------------^
//!  0 ink |gg kk ffff
//! 15 text|     [scratch]* · Rust · 1 lines · 1:13
//! 15 ink |bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
//! end
//! ```
//!
//! A row has up to three lines, one per layer: `text` as it reads, `mark`
//! and `ink` with a letter per column. Row numbers are aligned to the
//! right and labels padded, so `|` is one column: what stands before the
//! number is alignment, what follows `|` is cells. Trailing spaces are cut,
//! the width being in the header, and a layer with nothing in it, or a row
//! with nothing at all, is left out: the numbers say what was skipped.
//!
//! Mark letters: `-` current line, `~` selected, `=` a search hit, `^` a
//! caret. Ink letters: `k` keyword, `s` string, `c` comment, `t` type,
//! `f` function, `n` constant, `g` gutter, `r` rule, `b` the status bar.
//!
//! A frame, as a scenario's `---` writes it: the rows that differ, in
//! their text or in one layer, from the last frame taken in that layer,
//! each row with its letters under it, empty rows run together:
//!
//! ```text
//! 0|1 fn main() {}
//!  |--------------^
//! 1-10|
//! 15|     [scratch]* · Rust · 1 lines · 1:13
//! ```
//!
//! The frame is the run's business, not the editor's: the screen comes
//! whole over the socket, and whoever reads the scenario remembers the
//! last one and works out what changed.

use std::fmt::Write as _;

use anyhow::{Context, Result, bail};

use super::grid::{Cell, Grid};
use crate::editor::style::{Ink, Mark};

/// What a snapshot shows under the text: a scenario's `---` shows the
/// marks, its `---ink` the inks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Marks,
    Inks,
}

/// The `match`es are exhaustive on purpose: a mark or ink added later has
/// to choose its letter.
fn mark_letter(mark: Mark) -> char {
    match mark {
        Mark::None => ' ',
        Mark::CurrentLine => '-',
        Mark::Matched => '=',
        Mark::Selected => '~',
        Mark::Cursor => '^',
    }
}

fn ink_letter(ink: Ink) -> char {
    match ink {
        Ink::Text => ' ',
        Ink::Gutter => 'g',
        Ink::Rule => 'r',
        Ink::Status => 'b',
        Ink::Keyword => 'k',
        Ink::String => 's',
        Ink::Comment => 'c',
        Ink::Type => 't',
        Ink::Function => 'f',
        Ink::Constant => 'n',
    }
}

/// One row as letters, one per column, trailing spaces cut; empty when no
/// cell on the row earns one.
fn letters(grid: &Grid, y: usize, letter: impl Fn(&Cell) -> char) -> String {
    let mut letters = String::with_capacity(grid.width());
    for x in 0..grid.width() {
        let cell = grid.cell(x, y).expect("x and y are within the grid");
        // A continuation carries the style of the cluster before it, so the
        // letters stay one per column, as the text does on the screen.
        letters.push(letter(cell));
    }
    trimmed(letters)
}

fn trimmed(mut line: String) -> String {
    line.truncate(line.trim_end_matches(' ').len());
    line
}

/// Width of the row numbers, for `|` to line up.
fn digits(height: usize) -> usize {
    height.saturating_sub(1).to_string().len()
}

/// One row of the screen, each layer with its trailing spaces cut.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Row {
    text: String,
    marks: String,
    inks: String,
}

/// The screen as text, every row and every layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    width: usize,
    height: usize,
    rows: Vec<Row>,
}

impl Screen {
    pub fn of(grid: &Grid) -> Self {
        let rows = (0..grid.height())
            .map(|y| Row {
                text: trimmed(grid.row_text(y)),
                marks: letters(grid, y, |cell| mark_letter(cell.style.mark)),
                inks: letters(grid, y, |cell| ink_letter(cell.style.ink)),
            })
            .collect();
        Self {
            width: grid.width(),
            height: grid.height(),
            rows,
        }
    }

    /// The block described above.
    pub fn write(&self) -> String {
        let digits = digits(self.height);
        let mut out = format!("screen {} {}\n", self.width, self.height);
        for (y, row) in self.rows.iter().enumerate() {
            for (label, line) in [
                ("text", &row.text),
                ("mark", &row.marks),
                ("ink", &row.inks),
            ] {
                if !line.is_empty() {
                    writeln!(out, "{y:>digits$} {label:<4}|{line}").expect("writing to a string");
                }
            }
        }
        out.push_str("end\n");
        out
    }

    /// The block read back, as the replay reads the socket's answer.
    pub fn read(block: &str) -> Result<Self> {
        let mut lines = block.lines();
        let header = lines
            .next()
            .context("nothing where a screen was expected")?;
        let (width, height) = match header.split_whitespace().collect::<Vec<_>>()[..] {
            ["screen", width, height] => (width.parse()?, height.parse()?),
            _ => bail!("not a screen: {header:?}"),
        };
        let mut rows = vec![Row::default(); height];
        for line in lines {
            if line == "end" {
                return Ok(Self {
                    width,
                    height,
                    rows,
                });
            }
            let (head, content) = line
                .split_once('|')
                .with_context(|| format!("not a row of the screen: {line:?}"))?;
            let (y, label) = match head.split_whitespace().collect::<Vec<_>>()[..] {
                [y, label] => (y.parse::<usize>()?, label),
                _ => bail!("not a row of the screen: {line:?}"),
            };
            let row = rows
                .get_mut(y)
                .with_context(|| format!("row {y} on a screen of {height}"))?;
            match label {
                "text" => row.text = content.to_owned(),
                "mark" => row.marks = content.to_owned(),
                "ink" => row.inks = content.to_owned(),
                other => bail!("unknown layer {other:?}"),
            }
        }
        bail!("the screen has no end")
    }

    fn letters(&self, y: usize, layer: Layer) -> &str {
        match layer {
            Layer::Marks => &self.rows[y].marks,
            Layer::Inks => &self.rows[y].inks,
        }
    }
}

/// The frames a run has taken, for the next to differ from: one per layer,
/// since a snapshot differs from the last snapshot of its own kind.
#[derive(Default)]
pub struct Frames {
    marks: Option<Screen>,
    inks: Option<Screen>,
}

impl Frames {
    /// The frame of `screen` in `layer`: the rows that differ from the last
    /// one taken in that layer, or `same` when none does.
    pub fn take(&mut self, screen: Screen, layer: Layer) -> String {
        let previous = match layer {
            Layer::Marks => &mut self.marks,
            Layer::Inks => &mut self.inks,
        };
        let frame = match frame(&screen, previous.as_ref(), layer) {
            text if text.is_empty() => "same\n".to_owned(),
            text => text,
        };
        *previous = Some(screen);
        frame
    }
}

/// Every row, or with `previous` of the same size only the rows whose text
/// or `layer` differs from it; nothing at all when none does.
fn frame(screen: &Screen, previous: Option<&Screen>, layer: Layer) -> String {
    let previous = previous.filter(|p| p.width == screen.width && p.height == screen.height);
    let digits = digits(screen.height);
    let mut out = String::new();
    let mut empty_from: Option<usize> = None;

    let flush_empty = |out: &mut String, from: usize, to: usize| match from == to {
        true => writeln!(out, "{from:>digits$}|").expect("writing to a string"),
        false => writeln!(out, "{from:>digits$}-{to}|").expect("writing to a string"),
    };

    for (y, row) in screen.rows.iter().enumerate() {
        let letters = screen.letters(y, layer);
        let same =
            previous.is_some_and(|p| p.rows[y].text == row.text && p.letters(y, layer) == letters);
        if same {
            if let Some(from) = empty_from.take() {
                flush_empty(&mut out, from, y - 1);
            }
            continue;
        }
        if row.text.is_empty() && letters.is_empty() {
            empty_from.get_or_insert(y);
            continue;
        }
        if let Some(from) = empty_from.take() {
            flush_empty(&mut out, from, y - 1);
        }
        writeln!(out, "{y:>digits$}|{}", row.text).expect("writing to a string");
        if !letters.is_empty() {
            writeln!(out, "{:>digits$}|{letters}", "").expect("writing to a string");
        }
    }
    if let Some(from) = empty_from {
        flush_empty(&mut out, from, screen.height - 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::style::CellStyle;

    fn grid_with_a_current_line() -> Grid {
        let mut grid = Grid::new(10, 3);
        let gutter = CellStyle {
            ink: Ink::Gutter,
            mark: Mark::CurrentLine,
        };
        grid.put_str(0, 0, "1 ", gutter);
        grid.put_str(2, 0, "ab", CellStyle::marked(Mark::CurrentLine));
        grid.put_str(4, 0, "c", CellStyle::marked(Mark::Cursor));
        // The rest of the current line is painted to the edge, as `compose`
        // does, and shows up as letters where the text shows nothing.
        grid.put_str(5, 0, "     ", CellStyle::marked(Mark::CurrentLine));
        grid.put_str(0, 2, "status", CellStyle::of(Ink::Status));
        grid
    }

    #[test]
    fn the_screen_is_whole_with_the_empty_left_out() {
        let mut grid = Grid::new(10, 12);
        let gutter = CellStyle {
            ink: Ink::Gutter,
            mark: Mark::CurrentLine,
        };
        grid.put_str(0, 0, "1 ", gutter);
        grid.put_str(2, 0, "ab", CellStyle::marked(Mark::CurrentLine));
        grid.put_str(4, 0, "c", CellStyle::marked(Mark::Cursor));
        grid.put_str(5, 0, "     ", CellStyle::marked(Mark::CurrentLine));
        grid.put_str(0, 11, "status", CellStyle::of(Ink::Status));

        // Rows 1 to 10 have nothing and are not there; row 11 has no marks
        // and so no mark line. The gutter is `-` in the marks and `g` in
        // the inks.
        let expected = [
            "screen 10 12",
            " 0 text|1 abc",
            " 0 mark|----^-----",
            " 0 ink |gg",
            "11 text|status",
            "11 ink |bbbbbb",
            "end",
            "",
        ];
        let screen = Screen::of(&grid);
        assert_eq!(screen.write(), expected.join("\n"));
        assert_eq!(Screen::read(&screen.write()).unwrap(), screen);
    }

    #[test]
    fn a_block_that_is_not_a_screen_is_refused() {
        assert!(Screen::read("").is_err());
        assert!(Screen::read("ok\n").is_err());
        assert!(Screen::read("screen 4 2\n0 text|ab\n").is_err());
        assert!(Screen::read("screen 4 2\n5 text|ab\nend\n").is_err());
        assert!(Screen::read("screen 4 2\n0 glow|ab\nend\n").is_err());
    }

    #[test]
    fn rows_are_numbered_and_the_layer_goes_under_them() {
        let screen = Screen::of(&grid_with_a_current_line());
        assert_eq!(
            frame(&screen, None, Layer::Marks),
            "0|1 abc\n |----^-----\n1|\n2|status\n"
        );
        assert_eq!(
            frame(&screen, None, Layer::Inks),
            "0|1 abc\n |gg\n1|\n2|status\n |bbbbbb\n"
        );
    }

    #[test]
    fn empty_rows_collapse_into_a_range() {
        let mut grid = Grid::new(4, 12);
        grid.put_str(0, 0, "a", CellStyle::default());
        grid.put_str(0, 11, "s", CellStyle::of(Ink::Status));
        let screen = Screen::of(&grid);
        assert_eq!(frame(&screen, None, Layer::Marks), " 0|a\n 1-10|\n11|s\n");
    }

    #[test]
    fn only_the_rows_that_changed_in_the_layer_are_printed() {
        let mut before = Grid::new(6, 5);
        before.put_str(0, 0, "keep", CellStyle::default());
        before.put_str(0, 1, "gone", CellStyle::default());
        before.put_str(0, 2, "gone", CellStyle::default());
        before.put_str(0, 4, "abc", CellStyle::default());
        let mut after = before.clone();
        after.put_str(0, 1, "    ", CellStyle::default());
        after.put_str(0, 2, "    ", CellStyle::default());
        after.put_str(0, 4, "a", CellStyle::marked(Mark::Cursor));
        let (before, after) = (Screen::of(&before), Screen::of(&after));

        assert_eq!(
            frame(&after, Some(&before), Layer::Marks),
            "1-2|\n4|abc\n |^\n"
        );
        assert_eq!(frame(&after, Some(&after), Layer::Marks), "");
        // A frame of another size has nothing to differ from.
        let other = Screen::of(&Grid::new(6, 4));
        assert_eq!(
            frame(&after, Some(&other), Layer::Marks),
            frame(&after, None, Layer::Marks)
        );

        // A row that only changed colour is the same row to the marks and
        // a changed one to the inks.
        let mut coloured = Grid::new(6, 5);
        coloured.put_str(0, 0, "keep", CellStyle::of(Ink::Keyword));
        coloured.put_str(0, 4, "a", CellStyle::marked(Mark::Cursor));
        coloured.put_str(1, 4, "bc", CellStyle::default());
        let coloured = Screen::of(&coloured);
        assert_eq!(frame(&coloured, Some(&after), Layer::Marks), "");
        assert_eq!(
            frame(&coloured, Some(&after), Layer::Inks),
            "0|keep\n |kkkk\n"
        );
    }

    #[test]
    fn a_wide_cluster_gets_a_letter_per_column_and_a_styled_blank_keeps_its_letter() {
        let mut grid = Grid::new(6, 1);
        grid.put_str(0, 0, "日", CellStyle::marked(Mark::Selected));
        grid.put_str(2, 0, " ", CellStyle::marked(Mark::Cursor));
        let screen = Screen::of(&grid);
        assert_eq!(frame(&screen, None, Layer::Marks), "0|日\n |~~^\n");
    }

    #[test]
    fn a_snapshot_differs_from_the_last_of_its_own_kind() {
        let screen = Screen::of(&grid_with_a_current_line());
        let mut frames = Frames::default();
        assert!(
            frames
                .take(screen.clone(), Layer::Marks)
                .starts_with("0|1 abc\n |----^")
        );
        assert_eq!(frames.take(screen.clone(), Layer::Marks), "same\n");
        // The inks have not been shown yet, so the whole screen is.
        assert!(
            frames
                .take(screen.clone(), Layer::Inks)
                .starts_with("0|1 abc\n |gg\n")
        );
        assert_eq!(frames.take(screen, Layer::Inks), "same\n");
    }
}
