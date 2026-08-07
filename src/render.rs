use std::io::Write;

use anyhow::Result;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::{cursor, queue};

use crate::editor::Editor;
use crate::layout::{self, Glyph, DEFAULT_TAB_WIDTH};
use crate::prompt::Prompt;
use crate::view::CellStyle;

/// Draws a whole frame. Every cell is rewritten, so the screen is never
/// cleared first: clearing is what makes a terminal flicker.
pub fn draw(out: &mut impl Write, editor: &Editor, status: &str) -> Result<()> {
    let document = &editor.document;
    let view = &editor.view;
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

    let width = view.gutter + view.width;
    match &editor.prompt {
        Some(prompt) => draw_prompt(out, prompt, view.height, width)?,
        None => queue!(
            out,
            cursor::MoveTo(0, view.height as u16),
            SetAttribute(Attribute::Reverse),
            Print(pad(status, width)),
            SetAttribute(Attribute::Reset),
        )?,
    }

    out.flush()?;
    Ok(())
}

/// Each field is a labelled line with the caret drawn on it, since the real
/// terminal cursor stays hidden and we paint our own everywhere else.
fn draw_prompt(
    out: &mut impl Write,
    prompt: &Prompt,
    top: usize,
    width: usize,
) -> Result<()> {
    for (index, field) in prompt.fields.iter().enumerate() {
        let contents = field.contents();
        let label = format!(" {} ", field.label);
        let offset = label.chars().count();
        let column_of =
            |byte| offset + layout::column_of_byte(&contents, byte, DEFAULT_TAB_WIDTH);

        let active = index == prompt.active;
        let caret = active.then(|| column_of(field.cursor));
        let selection = field.selection();
        let selected = column_of(selection.start)..column_of(selection.end);

        queue!(out, cursor::MoveTo(0, (top + index) as u16))?;
        for (column, glyph) in pad(&format!("{label}{contents}"), width).chars().enumerate() {
            if caret == Some(column) {
                queue!(
                    out,
                    SetAttribute(Attribute::Reverse),
                    Print(glyph),
                    SetAttribute(Attribute::NoReverse)
                )?;
            } else if active && selected.contains(&column) {
                queue!(
                    out,
                    SetBackgroundColor(Color::DarkGrey),
                    Print(glyph),
                    ResetColor
                )?;
            } else {
                queue!(out, Print(glyph))?;
            }
        }
    }
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
