use std::io::Write;

use anyhow::Result;
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, queue};

use crate::editor::Editor;
use crate::layout::{self, Glyph};
use crate::panel::Panel;
use crate::picker::Picker;
use crate::prompt::{Field, Prompt};
use crate::view::CellStyle;

/// Panels and the status bar hold one-line strings rather than code, so a tab
/// in one of them is worth a single column.
const PANEL_TAB_WIDTH: usize = 1;

/// Assumes a dark terminal, which is what a theme (F12) will fix.
const CURRENT_LINE: Color = Color::AnsiValue(236);

/// Starts a row at the left edge of the editor's band. What follows it should
/// end with [`end_row`], which blanks the rest of the terminal.
fn start_row(out: &mut impl Write, margin: &str, row: usize) -> Result<()> {
    queue!(out, cursor::MoveTo(0, row as u16), Print(margin))?;
    Ok(())
}

fn end_row(out: &mut impl Write) -> Result<()> {
    queue!(out, SetAttribute(Attribute::Reset), Clear(ClearType::UntilNewLine))?;
    Ok(())
}

/// Draws a whole frame. Every cell is rewritten, so the screen is never
/// cleared first: clearing is what makes a terminal flicker.
pub fn draw(out: &mut impl Write, editor: &Editor, left: &str, right: &str) -> Result<()> {
    let buffer = &editor.buffer;
    let view = &editor.view;
    let margin = &" ".repeat(view.margin);
    let mut drawn = 0;

    for row in view.rows(&buffer.text) {
        let text = buffer.text.line_str(row.line);
        let highlight = view.highlight_line(&buffer.text, row.line, &editor.matches);
        // Wrapped continuations leave the gutter empty: the number belongs to
        // the logical line, not to every row it spans.
        let number = match row.index {
            0 => format!("{:>1$} ", row.line + 1, view.gutter - 1),
            _ => " ".repeat(view.gutter),
        };

        let (ink, paper) = match highlight.is_current() {
            true => (Color::Grey, CURRENT_LINE),
            false => (Color::DarkGrey, Color::Reset),
        };

        start_row(out, margin, drawn)?;
        queue!(
            out,
            SetBackgroundColor(paper),
            SetForegroundColor(ink),
            Print(number),
            ResetColor,
        )?;
        for (text, style) in view.visible_runs(&text, row.start_column, &highlight) {
            match style {
                CellStyle::Normal => queue!(out, Print(text))?,
                CellStyle::CurrentLine => queue!(
                    out,
                    SetBackgroundColor(CURRENT_LINE),
                    Print(text),
                    ResetColor
                )?,
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
        end_row(out)?;
        drawn += 1;
    }

    for row in drawn..view.height {
        start_row(out, margin, row)?;
        end_row(out)?;
    }

    let width = view.gutter + view.width;
    // What the panel asked for is not what it got: only the view knows how much
    // of the screen was left over, and it may have granted nothing at all.
    let body = view.panel_height.saturating_sub(2);
    if let Some(panel) = editor.panel.as_ref().filter(|_| body > 0) {
        let top = view.height + 1;
        rule(out, margin, view.height, width)?;
        match panel {
            Panel::Prompt(prompt) => draw_prompt(out, margin, prompt, top, width, body)?,
            Panel::Picker(picker) => draw_picker(out, margin, picker, top, width, body)?,
            Panel::Confirm(confirm) => {
                for (row, line) in confirm.rows().iter().take(body).enumerate() {
                    start_row(out, margin, top + row)?;
                    queue!(out, Print(pad(line, width)))?;
                    end_row(out)?;
                }
            }
        }
        rule(out, margin, view.height + view.panel_height - 1, width)?;
    }

    // The status bar is nailed to the last row so that it never moves when a
    // panel opens or changes height.
    start_row(out, margin, view.height + view.panel_height)?;
    queue!(
        out,
        SetAttribute(Attribute::Reverse),
        Print(status_line(left, right, width)),
        SetAttribute(Attribute::Reset),
    )?;
    end_row(out)?;

    out.flush()?;
    Ok(())
}

/// The query sits at the bottom, right above the status bar, and the list
/// grows upwards from it. That way neither the field nor the best candidate
/// moves as the list changes size.
fn draw_picker(
    out: &mut impl Write,
    margin: &str,
    picker: &Picker,
    top: usize,
    width: usize,
    height: usize,
) -> Result<()> {
    // The query keeps the bottom row; candidates take what is left of a strip
    // that may be shorter than the list, best matches nearest the query.
    let rows = picker.visible().count().min(height - 1);

    for (row, (index, candidate)) in picker.visible().take(rows).enumerate() {
        let selected = index == picker.selected;
        // Indented to line up with the text of the query below.
        let line = pad(&format!("   {candidate}"), width);

        start_row(out, margin, top + rows - 1 - row)?;
        match selected {
            true => queue!(
                out,
                SetBackgroundColor(Color::DarkGrey),
                Print(line),
                ResetColor
            )?,
            false => queue!(out, Print(line))?,
        }
        end_row(out)?;
    }

    draw_field(out, margin, &picker.query, true, top + rows, width)
}

fn rule(out: &mut impl Write, margin: &str, row: usize, width: usize) -> Result<()> {
    start_row(out, margin, row)?;
    queue!(
        out,
        SetForegroundColor(Color::DarkGrey),
        Print("─".repeat(width)),
        ResetColor,
    )?;
    end_row(out)
}

/// The left half belongs to the document, the right half to whatever panel is
/// open. The left is what gives way when there is not enough room, and the
/// right keeps a space so it never touches the edge of the screen.
fn status_line(left: &str, right: &str, width: usize) -> String {
    let right = match right.is_empty() {
        true => String::new(),
        false => format!("{right} "),
    };
    let columns = layout::line_width(&right, PANEL_TAB_WIDTH).min(width);
    let right = pad(&right, columns);
    let left = pad(left, width - columns);
    format!("{left}{right}")
}

/// Each field is a labelled line with the caret drawn on it, since the real
/// terminal cursor stays hidden and we paint our own everywhere else.
fn draw_prompt(
    out: &mut impl Write,
    margin: &str,
    prompt: &Prompt,
    top: usize,
    width: usize,
    height: usize,
) -> Result<()> {
    for (index, field) in prompt.fields.iter().take(height).enumerate() {
        draw_field(out, margin, field, index == prompt.active, top + index, width)?;
    }
    Ok(())
}

/// The caret and the selection are painted by us, since the terminal's own
/// cursor stays hidden everywhere in this editor.
fn draw_field(
    out: &mut impl Write,
    margin: &str,
    field: &Field,
    active: bool,
    row: usize,
    width: usize,
) -> Result<()> {
    let contents = field.contents();
    let label = format!(" {} ", field.label);
    let offset = layout::line_width(&label, PANEL_TAB_WIDTH);
    let column_of = |byte| offset + layout::column_of_byte(&contents, byte, PANEL_TAB_WIDTH);

    let caret = active.then(|| column_of(field.cursor));
    let selection = field.selection();
    let selected = column_of(selection.start)..column_of(selection.end);

    // Walked as cells rather than characters: the caret is placed in display
    // columns, so counting anything else drifts away from it on wide glyphs.
    let line = pad(&format!("{label}{contents}"), width);
    start_row(out, margin, row)?;
    for cell in layout::cells(&line, PANEL_TAB_WIDTH) {
        let glyph = match cell.glyph {
            Glyph::Text(text) => text,
            _ => " ",
        };
        if caret == Some(cell.column) {
            queue!(
                out,
                SetAttribute(Attribute::Reverse),
                Print(glyph),
                SetAttribute(Attribute::NoReverse)
            )?;
        } else if active && selected.contains(&cell.column) {
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
    end_row(out)
}

/// Truncates or pads to exactly `width` columns, counting display width rather
/// than characters so that a path with wide glyphs cannot overflow the line.
fn pad(text: &str, width: usize) -> String {
    let mut out = String::with_capacity(width);
    let mut column = 0;

    for cell in layout::cells(text, PANEL_TAB_WIDTH) {
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
    use crate::buffer::Buffer;
    use crate::text::Text;

    fn drawn(term_width: usize) -> (String, usize) {
        let mut editor = Editor::new(Buffer::from_text(Text::from_str("hello\n")));
        editor.view.fit(term_width, 4, 2, 0);

        let mut out = Vec::new();
        draw(&mut out, &editor, "", "").unwrap();
        (String::from_utf8(out).unwrap(), editor.view.margin)
    }

    #[test]
    fn every_row_starts_at_the_edge_of_the_band() {
        let (wide, margin) = drawn(200);
        assert!(margin > 0);
        for row in 1..=4 {
            let start = format!("\x1b[{row};1H{}\x1b", " ".repeat(margin));
            assert!(wide.contains(&start), "row {row} is not indented");
        }

        // A terminal narrower than the cap keeps the editor at the left edge.
        let (narrow, margin) = drawn(80);
        assert_eq!(margin, 0);
        assert!(narrow.contains("\x1b[1;1H\x1b"));
    }

    #[test]
    fn only_the_current_line_paints_its_gutter() {
        let (frame, margin) = drawn(200);
        let painted = |row: usize| {
            let marker = format!("\x1b[{row};1H");
            let at = frame.find(&marker).unwrap() + marker.len() + margin;
            frame[at..].starts_with("\x1b[48;")
        };

        assert!(painted(1), "the cursor's row should carry a background");
        assert!(!painted(2));
    }

    #[test]
    fn padding_counts_display_columns() {
        assert_eq!(pad("ab", 4), "ab  ");
        assert_eq!(pad("abcdef", 4), "abcd");
        assert_eq!(pad("日本", 3), "日 ");
        assert_eq!(pad("", 2), "  ");
    }

    fn columns(line: &str) -> usize {
        layout::line_width(line, PANEL_TAB_WIDTH)
    }

    #[test]
    fn the_status_bar_is_exactly_as_wide_as_the_screen() {
        assert_eq!(columns(&status_line("left", "right", 20)), 20);
        // Wide glyphs in a path are worth two columns each, which counting
        // characters would have missed.
        assert_eq!(columns(&status_line("left", "日本.rs", 20)), 20);
        // Both halves want more room than there is.
        assert_eq!(columns(&status_line("a long message", "日本語.rs", 12)), 12);
        assert_eq!(columns(&status_line("", "", 8)), 8);
    }
}
