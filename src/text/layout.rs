use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub const DEFAULT_TAB_WIDTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph<'a> {
    /// One grapheme cluster, printed as is.
    Text(&'a str),
    /// Printed as `width` spaces.
    Tab,
    /// Unprintable; the renderer substitutes a placeholder.
    Control,
}

/// One grapheme cluster placed on the display grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell<'a> {
    /// Byte offset within the line.
    pub byte: usize,
    /// Display column where the cluster starts.
    pub column: usize,
    /// Cells occupied.
    pub width: usize,
    pub glyph: Glyph<'a>,
}

impl Cell<'_> {
    pub fn end_column(&self) -> usize {
        self.column + self.width
    }
}

/// Walks a line, assigning every grapheme cluster its display column.
pub fn cells(line: &str, tab_width: usize) -> Cells<'_> {
    Cells {
        graphemes: line.grapheme_indices(true),
        column: 0,
        tab_width,
    }
}

pub struct Cells<'a> {
    graphemes: unicode_segmentation::GraphemeIndices<'a>,
    column: usize,
    tab_width: usize,
}

impl<'a> Iterator for Cells<'a> {
    type Item = Cell<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (byte, grapheme) = self.graphemes.next()?;
        let column = self.column;

        let (glyph, width) = if grapheme == "\t" {
            (Glyph::Tab, self.tab_width - column % self.tab_width)
        } else if is_control(grapheme) {
            (Glyph::Control, 1)
        } else {
            // A cluster of combining marks alone measures zero; give it a cell
            // anyway so that no two clusters share a column.
            (Glyph::Text(grapheme), grapheme.width().max(1))
        };

        self.column += width;
        Some(Cell {
            byte,
            column,
            width,
            glyph,
        })
    }
}

/// The indentation of a line: its leading run of spaces and tabs, and
/// nothing else — a no-break space is text. Everything that reasons about
/// indentation reads it from here, so that they cannot disagree on what it
/// is; its bytes, its width and what it is made of all follow from the slice.
pub fn indentation(line: &str) -> &str {
    let end = line
        .bytes()
        .position(|byte| byte != b' ' && byte != b'\t')
        .unwrap_or(line.len());
    &line[..end]
}

/// Display column at a byte offset, for placing the cursor.
pub fn column_of_byte(line: &str, byte: usize, tab_width: usize) -> usize {
    cells(line, tab_width)
        .find(|cell| cell.byte >= byte)
        .map_or_else(|| line_width(line, tab_width), |cell| cell.column)
}

/// Byte offset at a display column, for translating a mouse click. Columns
/// inside a wide cluster resolve to that cluster's start.
pub fn byte_at_column(line: &str, column: usize, tab_width: usize) -> usize {
    cells(line, tab_width)
        .find(|cell| cell.end_column() > column)
        .map_or(line.len(), |cell| cell.byte)
}

pub fn line_width(line: &str, tab_width: usize) -> usize {
    cells(line, tab_width)
        .last()
        .map_or(0, |cell| cell.end_column())
}

/// The column reached by printing `text` from `column`.
pub fn advance(column: usize, text: &str, tab_width: usize) -> usize {
    let cells = Cells {
        graphemes: text.grapheme_indices(true),
        column,
        tab_width,
    };
    cells.last().map_or(column, |cell| cell.end_column())
}

fn is_control(grapheme: &str) -> bool {
    let mut chars = grapheme.chars();
    matches!((chars.next(), chars.next()), (Some(c), None) if c.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAB: usize = 4;

    fn columns(line: &str) -> Vec<usize> {
        cells(line, TAB).map(|cell| cell.column).collect()
    }

    fn widths(line: &str) -> Vec<usize> {
        cells(line, TAB).map(|cell| cell.width).collect()
    }

    #[test]
    fn ascii_advances_one_column_per_character() {
        assert_eq!(columns("abc"), [0, 1, 2]);
        assert_eq!(line_width("abc", TAB), 3);
    }

    #[test]
    fn tabs_stop_at_multiples_of_the_tab_width() {
        assert_eq!(widths("\t"), [4]);
        assert_eq!(widths("a\t"), [1, 3]);
        assert_eq!(widths("abc\t"), [1, 1, 1, 1]);
        assert_eq!(widths("abcd\t"), [1, 1, 1, 1, 4]);
    }

    #[test]
    fn cyrillic_is_one_column_but_two_bytes() {
        let cells: Vec<_> = cells("да", TAB).collect();
        assert_eq!(cells[0].byte, 0);
        assert_eq!(cells[1].byte, 2);
        assert_eq!(cells[1].column, 1);
        assert_eq!(line_width("да", TAB), 2);
    }

    #[test]
    fn wide_characters_take_two_columns() {
        assert_eq!(widths("日本語"), [2, 2, 2]);
        assert_eq!(columns("日本語"), [0, 2, 4]);
    }

    #[test]
    fn combining_marks_join_their_base() {
        let cells: Vec<_> = cells("e\u{301}x", TAB).collect();
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].width, 1);
        assert_eq!(cells[1].byte, 3);
        assert_eq!(cells[1].column, 1);
    }

    #[test]
    fn control_characters_get_a_placeholder_cell() {
        let cells: Vec<_> = cells("a\u{7}b", TAB).collect();
        assert_eq!(cells[1].glyph, Glyph::Control);
        assert_eq!(cells[1].width, 1);
        assert_eq!(cells[2].column, 2);
    }

    #[test]
    fn columns_and_bytes_convert_both_ways() {
        let line = "a\tдб";
        assert_eq!(column_of_byte(line, 0, TAB), 0);
        assert_eq!(column_of_byte(line, 1, TAB), 1);
        assert_eq!(column_of_byte(line, 2, TAB), 4);
        assert_eq!(column_of_byte(line, 4, TAB), 5);
        assert_eq!(column_of_byte(line, line.len(), TAB), 6);

        assert_eq!(byte_at_column(line, 0, TAB), 0);
        assert_eq!(byte_at_column(line, 1, TAB), 1);
        assert_eq!(byte_at_column(line, 3, TAB), 1);
        assert_eq!(byte_at_column(line, 4, TAB), 2);
        assert_eq!(byte_at_column(line, 99, TAB), line.len());
    }

    #[test]
    fn advancing_from_a_column_stretches_a_tab_from_there() {
        assert_eq!(advance(0, "ab", TAB), 2);
        assert_eq!(advance(3, "", TAB), 3);
        // A tab printed from column 1 stops at 4; from column 4, at 8.
        assert_eq!(advance(1, "\t", TAB), 4);
        assert_eq!(advance(4, "\tb", TAB), 9);
    }

    #[test]
    fn a_click_inside_a_wide_character_lands_on_its_start() {
        assert_eq!(byte_at_column("日x", 1, TAB), 0);
        assert_eq!(byte_at_column("日x", 2, TAB), 3);
    }

    #[test]
    fn indentation_is_the_leading_spaces_and_tabs() {
        assert_eq!(indentation("  \tx"), "  \t");
        assert_eq!(indentation("x  "), "");
        assert_eq!(indentation("   "), "   ");
        assert_eq!(indentation("\u{a0}x"), "");
        assert_eq!(indentation(""), "");
    }

    #[test]
    fn empty_lines_have_no_cells() {
        assert_eq!(cells("", TAB).count(), 0);
        assert_eq!(line_width("", TAB), 0);
        assert_eq!(byte_at_column("", 0, TAB), 0);
        assert_eq!(column_of_byte("", 0, TAB), 0);
    }
}
