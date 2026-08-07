use std::io::Write;

use anyhow::Result;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::{cursor, queue};

use crate::document::Document;
use crate::layout::{self, Glyph};
use crate::view::{CellStyle, View};

/// Draws a whole frame. Every cell is rewritten, so the screen is never
/// cleared first: clearing is what makes a terminal flicker.
pub fn draw(out: &mut impl Write, document: &Document, view: &View, status: &str) -> Result<()> {
    let mut drawn = 0;

    for row in view.rows(&document.text) {
        let text = document.text.line_str(row.line);
        let highlight = view.highlight_line(&document.text, row.line);
        // Wrapped continuations leave the gutter empty: the number belongs to
        // the logical line, not to every row it spans.
        let number = match row.index {
            0 => format!("{:>1$} ", row.line + 1, view.gutter - 1),
            _ => " ".repeat(view.gutter),
        };

        queue!(
            out,
            cursor::MoveTo(0, drawn as u16),
            SetForegroundColor(Color::DarkGrey),
            Print(number),
            ResetColor,
        )?;
        for (text, style) in view.visible_runs(&text, row.start_column, &highlight) {
            match style {
                CellStyle::Normal => queue!(out, Print(text))?,
                CellStyle::Selected => queue!(
                    out,
                    SetBackgroundColor(Color::DarkGrey),
                    Print(text),
                    ResetColor
                )?,
                CellStyle::Cursor => queue!(
                    out,
                    SetAttribute(Attribute::Reverse),
                    Print(text),
                    SetAttribute(Attribute::NoReverse)
                )?,
            }
        }
        drawn += 1;
    }

    let blank = " ".repeat(view.gutter + view.width);
    for row in drawn..view.height {
        queue!(out, cursor::MoveTo(0, row as u16), Print(&blank))?;
    }

    queue!(
        out,
        cursor::MoveTo(0, view.height as u16),
        SetAttribute(Attribute::Reverse),
        Print(pad(status, view.gutter + view.width)),
        SetAttribute(Attribute::Reset),
    )?;

    out.flush()?;
    Ok(())
}

/// Truncates or pads to exactly `width` columns, counting display width rather
/// than characters so that a path with wide glyphs cannot overflow the line.
fn pad(text: &str, width: usize) -> String {
    let mut out = String::with_capacity(width);
    let mut column = 0;

    for cell in layout::cells(text, 1) {
        if cell.end_column() > width {
            break;
        }
        match cell.glyph {
            Glyph::Text(text) => out.push_str(text),
            _ => out.push(' '),
        }
        column = cell.end_column();
    }

    out.extend(std::iter::repeat_n(' ', width - column));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_counts_display_columns() {
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("abcdef", 4), "abcd");
        assert_eq!(pad("日本", 3), "日 ");
        assert_eq!(pad("", 2), "  ");
    }
}
