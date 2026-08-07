use std::io::Write;

use anyhow::Result;
use crossterm::style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor};
use crossterm::{cursor, queue};

use crate::document::Document;
use crate::layout::{self, Glyph};
use crate::view::View;

/// Draws a whole frame. Every cell is rewritten, so the screen is never
/// cleared first: clearing is what makes a terminal flicker.
pub fn draw(out: &mut impl Write, document: &Document, view: &View, status: &str) -> Result<()> {
    let lines = view.visible_lines(&document.text);
    let mut rows = 0;

    for line in lines {
        let text = document.text.line_str(line);
        queue!(
            out,
            cursor::MoveTo(0, rows as u16),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("{:>1$} ", line + 1, view.gutter - 1)),
            ResetColor,
            Print(view.visible_text(&text)),
        )?;
        rows += 1;
    }

    let blank = " ".repeat(view.gutter + view.width);
    for row in rows..view.height {
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
