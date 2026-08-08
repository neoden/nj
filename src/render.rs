use std::io::Write;

use anyhow::Result;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::{cursor, queue};

use crate::editor::Editor;
use crate::layout::{self, Glyph, DEFAULT_TAB_WIDTH};
use crate::panel::Panel;
use crate::picker::Picker;
use crate::prompt::{Field, Prompt};
use crate::view::CellStyle;

/// Draws a whole frame. Every cell is rewritten, so the screen is never
/// cleared first: clearing is what makes a terminal flicker.
pub fn draw(out: &mut impl Write, editor: &Editor, left: &str, right: &str) -> Result<()> {
    let document = &editor.document;
    let view = &editor.view;
    let mut drawn = 0;

    for row in view.rows(&document.text) {
        let text = document.text.line_str(row.line);
        let highlight = view.highlight_line(&document.text, row.line, &editor.matches);
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
                // Underlined rather than coloured, so that a hit reads the
                // same whatever the terminal's palette looks like.
                CellStyle::Matched => queue!(
                    out,
                    SetAttribute(Attribute::Underlined),
                    Print(text),
                    SetAttribute(Attribute::NoUnderline)
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
    if let Some(panel) = &editor.panel {
        rule(out, view.height, width)?;
        match panel {
            Panel::Prompt(prompt) => draw_prompt(out, prompt, view.height + 1, width)?,
            Panel::Picker(picker) => draw_picker(out, picker, view.height + 1, width)?,
        }
        rule(out, view.height + panel.height() - 1, width)?;
    }

    // The status bar is nailed to the last row so that it never moves when a
    // panel opens or changes height.
    queue!(
        out,
        cursor::MoveTo(0, (view.height + editor.panel_height()) as u16),
        SetAttribute(Attribute::Reverse),
        Print(status_line(left, right, width)),
        SetAttribute(Attribute::Reset),
    )?;

    out.flush()?;
    Ok(())
}

/// The query sits at the bottom, right above the status bar, and the list
/// grows upwards from it. That way neither the field nor the best candidate
/// moves as the list changes size.
fn draw_picker(out: &mut impl Write, picker: &Picker, top: usize, width: usize) -> Result<()> {
    let rows = picker.visible().count();

    for (row, (index, candidate)) in picker.visible().enumerate() {
        let selected = index == picker.selected;
        // Indented to line up with the text of the query below.
        let line = pad(&format!("   {candidate}"), width);

        queue!(out, cursor::MoveTo(0, (top + rows - 1 - row) as u16))?;
        match selected {
            true => queue!(
                out,
                SetBackgroundColor(Color::DarkGrey),
                Print(line),
                ResetColor
            )?,
            false => queue!(out, Print(line))?,
        }
    }

    draw_field(out, &picker.query, true, top + rows, width)
}

fn rule(out: &mut impl Write, row: usize, width: usize) -> Result<()> {
    queue!(
        out,
        cursor::MoveTo(0, row as u16),
        SetForegroundColor(Color::DarkGrey),
        Print("─".repeat(width)),
        ResetColor,
    )?;
    Ok(())
}

/// The left half belongs to the document, the right half to whatever panel is
/// open. The left is what gives way when there is not enough room, and the
/// right keeps a space so it never touches the edge of the screen.
fn status_line(left: &str, right: &str, width: usize) -> String {
    let right = match right.is_empty() {
        true => String::new(),
        false => format!("{right} "),
    };
    let right = pad(&right, layout::line_width(&right, 1).min(width));
    let left = pad(left, width.saturating_sub(right.chars().count()));
    format!("{left}{right}")
}

/// Each field is a labelled line with the caret drawn on it, since the real
/// terminal cursor stays hidden and we paint our own everywhere else.
fn draw_prompt(out: &mut impl Write, prompt: &Prompt, top: usize, width: usize) -> Result<()> {
    for (index, field) in prompt.fields.iter().enumerate() {
        draw_field(out, field, index == prompt.active, top + index, width)?;
    }
    Ok(())
}

/// The caret and the selection are painted by us, since the terminal's own
/// cursor stays hidden everywhere in this editor.
fn draw_field(
    out: &mut impl Write,
    field: &Field,
    active: bool,
    row: usize,
    width: usize,
) -> Result<()> {
    let contents = field.contents();
    let label = format!(" {} ", field.label);
    let offset = label.chars().count();
    let column_of = |byte| offset + layout::column_of_byte(&contents, byte, DEFAULT_TAB_WIDTH);

    let caret = active.then(|| column_of(field.cursor));
    let selection = field.selection();
    let selected = column_of(selection.start)..column_of(selection.end);

    queue!(out, cursor::MoveTo(0, row as u16))?;
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
