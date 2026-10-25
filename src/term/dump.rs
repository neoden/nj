//! A frame as text, for a scenario to read and a test to compare (ADR 25).
//! One line per screen row, prefixed with the row's number; under a row, when
//! any cell on it carries a style worth seeing, a second line with a letter
//! per cell. Empty rows that follow one another collapse into a range. Given
//! the frame before, only the rows that differ from it are printed, so a
//! scenario reads as what each step changed.

use crate::editor::style::CellStyle;

use super::grid::Grid;

/// The letter a style shows in the line under a row. A space is a style
/// that the row's text already makes visible — a number in the gutter, the
/// status bar — so it does not earn the row a second line on its own. The
/// `match` is exhaustive on purpose: a style added later has to choose.
fn letter(style: CellStyle) -> char {
    match style {
        CellStyle::Normal | CellStyle::Gutter | CellStyle::Rule | CellStyle::Status => ' ',
        CellStyle::CurrentLine => '-',
        CellStyle::GutterCurrent => ':',
        CellStyle::Matched => '=',
        CellStyle::Selected => '~',
        CellStyle::Cursor => '^',
    }
}

/// The styles of one row as letters, trailing spaces cut. Empty when nothing
/// on the row is worth a letter.
fn styles(grid: &Grid, y: usize) -> String {
    let mut letters = String::with_capacity(grid.width());
    for x in 0..grid.width() {
        let cell = grid.cell(x, y).expect("x and y are within the grid");
        // A continuation carries the style of the cluster before it, so the
        // letters stay one per column, as the text does on the screen.
        letters.push(letter(cell.style));
    }
    letters.truncate(letters.trim_end().len());
    letters
}

fn text(grid: &Grid, y: usize) -> String {
    let mut text = grid.row_text(y);
    text.truncate(text.trim_end_matches(' ').len());
    text
}

/// Every row, or with `previous` of the same size only the rows that differ
/// from it; nothing at all when none does.
pub fn dump(grid: &Grid, previous: Option<&Grid>) -> String {
    let previous = previous.filter(|p| p.width() == grid.width() && p.height() == grid.height());
    let digits = grid.height().saturating_sub(1).to_string().len();
    let mut out = String::new();
    let mut empty_from: Option<usize> = None;

    let flush_empty = |out: &mut String, from: usize, to: usize| match from == to {
        true => out.push_str(&format!("{from:>digits$}|\n")),
        false => out.push_str(&format!("{from:>digits$}-{to}|\n")),
    };

    for y in 0..grid.height() {
        let same =
            previous.is_some_and(|p| (0..grid.width()).all(|x| p.cell(x, y) == grid.cell(x, y)));
        if same {
            if let Some(from) = empty_from.take() {
                flush_empty(&mut out, from, y - 1);
            }
            continue;
        }
        let (text, styles) = (text(grid, y), styles(grid, y));
        if text.is_empty() && styles.is_empty() {
            empty_from.get_or_insert(y);
            continue;
        }
        if let Some(from) = empty_from.take() {
            flush_empty(&mut out, from, y - 1);
        }
        out.push_str(&format!("{y:>digits$}|{text}\n"));
        if !styles.is_empty() {
            out.push_str(&format!("{:>digits$}|{styles}\n", ""));
        }
    }
    if let Some(from) = empty_from {
        flush_empty(&mut out, from, grid.height() - 1);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_numbered_and_styles_go_under_them() {
        let mut grid = Grid::new(10, 3);
        grid.put_str(0, 0, "1 ", CellStyle::GutterCurrent);
        grid.put_str(2, 0, "ab", CellStyle::CurrentLine);
        grid.put_str(4, 0, "c", CellStyle::Cursor);
        // The rest of the current line is painted to the edge, as `compose`
        // does, and shows up as letters where the text shows nothing.
        grid.put_str(5, 0, "     ", CellStyle::CurrentLine);
        grid.put_str(0, 2, "status", CellStyle::Status);
        assert_eq!(dump(&grid, None), "0|1 abc\n |::--^-----\n1|\n2|status\n");
    }

    #[test]
    fn empty_rows_collapse_into_a_range() {
        let mut grid = Grid::new(4, 12);
        grid.put_str(0, 0, "a", CellStyle::Normal);
        grid.put_str(0, 11, "s", CellStyle::Status);
        assert_eq!(dump(&grid, None), " 0|a\n 1-10|\n11|s\n");
    }

    #[test]
    fn only_the_rows_that_changed_are_printed() {
        let mut before = Grid::new(6, 5);
        before.put_str(0, 0, "keep", CellStyle::Normal);
        before.put_str(0, 1, "gone", CellStyle::Normal);
        before.put_str(0, 2, "gone", CellStyle::Normal);
        before.put_str(0, 4, "abc", CellStyle::Normal);
        let mut after = before.clone();
        after.put_str(0, 1, "    ", CellStyle::Normal);
        after.put_str(0, 2, "    ", CellStyle::Normal);
        after.put_str(0, 4, "a", CellStyle::Cursor);

        assert_eq!(dump(&after, Some(&before)), "1-2|\n4|abc\n |^\n");
        assert_eq!(dump(&after, Some(&after)), "");
        // A frame of another size has nothing to differ from.
        assert_eq!(dump(&after, Some(&Grid::new(6, 4))), dump(&after, None));
    }

    #[test]
    fn a_wide_cluster_gets_a_letter_per_column_and_a_styled_blank_keeps_its_letter() {
        let mut grid = Grid::new(6, 1);
        grid.put_str(0, 0, "日", CellStyle::Selected);
        grid.put_str(2, 0, " ", CellStyle::Cursor);
        assert_eq!(dump(&grid, None), "0|日\n |~~^\n");
    }
}
