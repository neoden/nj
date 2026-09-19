//! Composes a frame into the grid (ADR 5). Nothing here writes to the
//! terminal: the grid is filled from the editor's state, and the renderer
//! flushes what changed since the frame before.

use std::io::Write;
use std::ops::Range;

use anyhow::Result;

use super::grid::{Grid, CONTROL_PLACEHOLDER};
use crate::editor::panel::picker::Picker;
use crate::editor::panel::prompt::{Field, Prompt};
use crate::editor::panel::Panel;
use crate::editor::style::CellStyle;
use crate::editor::view::LineHighlight;
use crate::editor::Editor;
use crate::text::layout::{self, Glyph};

/// Panels and the status bar hold one-line strings rather than code, so a tab
/// in one of them is worth a single column.
const PANEL_TAB_WIDTH: usize = 1;

/// Holds the frame last sent, so that the next one costs only its difference.
/// Two grids trade places every frame; nothing is allocated between resizes.
#[derive(Debug)]
pub struct Renderer {
    previous: Option<Grid>,
    scratch: Grid,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            previous: None,
            scratch: Grid::new(0, 0),
        }
    }

    /// Draws a frame of `width` by `height` cells. A change of size throws
    /// the previous frame away, since nothing on it is where it was.
    pub fn draw(
        &mut self,
        out: &mut impl Write,
        editor: &Editor,
        (width, height): (usize, usize),
    ) -> Result<()> {
        if self.scratch.width() != width || self.scratch.height() != height {
            self.scratch = Grid::new(width, height);
            self.previous = None;
        }
        self.scratch.clear();
        compose(&mut self.scratch, editor);
        self.scratch.flush(out, self.previous.as_ref())?;

        let spare = self.previous.take().unwrap_or_else(|| Grid::new(width, height));
        self.previous = Some(std::mem::replace(&mut self.scratch, spare));
        Ok(())
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Lays the whole screen out on the grid: the text with its gutter, a panel
/// if one is open, and the status bar on the last row.
pub fn compose(grid: &mut Grid, editor: &Editor) {
    let (view, buffer) = (editor.view(), editor.buffer());
    let screen = editor.screen();
    let x = view.area.x;
    let width = view.area.width;

    for (row, y) in view.rows(buffer).zip(view.area.y..) {
        let text = buffer.text().line_str(row.line);
        let highlight = view.highlight_line(buffer, row.line, editor.matches());
        // Wrapped continuations leave the gutter empty: the number belongs to
        // the logical line, not to every row it spans.
        let number = match row.index {
            0 => format!("{:>1$} ", row.line + 1, view.gutter - 1),
            _ => " ".repeat(view.gutter),
        };
        let gutter = match highlight.is_current() {
            true => CellStyle::GutterCurrent,
            false => CellStyle::Gutter,
        };
        grid.put_str(x, y, &number, gutter);
        let left = row.start_column + view.left_column;
        let columns = left..left + view.width();
        draw_row(grid, x + view.gutter, y, columns, &text, buffer.tab_width(), &highlight);
    }

    // What the panel asked for is not what it got: only the screen knows how
    // much was left over, and it may have granted nothing at all.
    let body = screen.panel_height.saturating_sub(2);
    if let Some(panel) = editor.panel().filter(|_| body > 0) {
        let top = screen.panel_top() + 1;
        rule(grid, x, screen.panel_top(), width);
        match panel {
            Panel::Prompt(prompt) => draw_prompt(grid, x, prompt, top, width, body),
            Panel::Picker(picker) => draw_picker(grid, x, picker, top, width, body),
            Panel::Confirm(confirm) => {
                for (row, line) in confirm.rows().iter().take(body).enumerate() {
                    grid.put_str(x, top + row, &pad(line, width), CellStyle::Normal);
                }
            }
        }
        rule(grid, x, screen.status_row() - 1, width);
    }

    let (left, right) = status(editor);
    grid.put_str(x, screen.status_row(), &status_line(&left, &right, width), CellStyle::Status);
}

/// The right side is the document, always in the same place. The left side is
/// whatever is going on right now, so the eye only goes there when something
/// is happening.
fn status(editor: &Editor) -> (String, String) {
    let (view, buffer) = (editor.view(), editor.buffer());
    let line = buffer.text().line_of(view.selections.primary().head);
    let column = editor.cursor_column();

    let right = format!(
        "{}{} · {} · {} lines · {}:{}",
        buffer.name(),
        if buffer.is_modified() { "*" } else { "" },
        buffer.language.name,
        buffer.text().line_count(),
        line + 1,
        column + 1,
    );

    let left = match editor.message() {
        Some(message) => message.to_owned(),
        None => {
            let cursors = view.selections.len();
            [
                editor.panel_status(),
                match cursors {
                    1 => String::new(),
                    count => format!("{count} cursors"),
                },
            ]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" · ")
        }
    };

    (format!(" {left}"), right)
}

/// The display `columns` of a line, drawn on one row of the grid starting at
/// `x`, blank where the line has nothing. A cluster cut by either edge
/// becomes blanks too, because half a character cannot be drawn.
fn draw_row(
    grid: &mut Grid,
    x: usize,
    y: usize,
    columns: Range<usize>,
    line: &str,
    tab_width: usize,
    highlight: &LineHighlight,
) {
    let Range { start: left, end: right } = columns;
    let mut painter = Painter { grid, x, y, left, highlight };
    let mut column = left;

    for cell in layout::cells(line, tab_width) {
        if cell.end_column() <= left {
            continue;
        }
        if cell.column >= right {
            break;
        }

        let visible = cell.column.max(left)..cell.end_column().min(right);
        // A cluster spanning several columns may be cut by an edge, or be
        // half selected, so it goes down one column at a time.
        if cell.column < left || cell.end_column() > right {
            painter.blanks(visible.clone());
        } else {
            match cell.glyph {
                Glyph::Text(text) => painter.put(cell.column, text, cell.width),
                Glyph::Tab => painter.blanks(visible.clone()),
                Glyph::Control => painter.put(cell.column, CONTROL_PLACEHOLDER, 1),
            }
        }
        column = visible.end;
    }

    painter.blanks(column..right);
}

/// Puts cells of one row on the grid, addressed by display column of the
/// line, with the style the highlight gives that column.
struct Painter<'a> {
    grid: &'a mut Grid,
    x: usize,
    y: usize,
    /// The line column at the left edge of the row.
    left: usize,
    highlight: &'a LineHighlight,
}

impl Painter<'_> {
    fn put(&mut self, column: usize, cluster: &str, width: usize) {
        let style = self.highlight.style_at(column);
        self.grid.put(self.x + column - self.left, self.y, cluster, width, style);
    }

    fn blanks(&mut self, columns: Range<usize>) {
        for column in columns {
            self.put(column, " ", 1);
        }
    }
}

/// The query sits at the bottom, right above the status bar, and the list
/// grows upwards from it. That way neither the field nor the best candidate
/// moves as the list changes size.
fn draw_picker(grid: &mut Grid, x: usize, picker: &Picker, top: usize, width: usize, height: usize) {
    // The query keeps the bottom row; candidates take what is left of a strip
    // that may be shorter than the list, best matches nearest the query.
    let rows = picker.visible().count().min(height - 1);

    for (row, (index, candidate)) in picker.visible().take(rows).enumerate() {
        let style = match index == picker.selected {
            true => CellStyle::Selected,
            false => CellStyle::Normal,
        };
        // Indented to line up with the text of the query below; the detail,
        // if any, sits at the right edge with the same space the status bar
        // keeps there.
        let y = top + rows - 1 - row;
        let line = pad(&format!("   {}", candidate.label), width);
        grid.put_str(x, y, &line, style);
        if !candidate.detail.is_empty() {
            let detail = format!("{} ", candidate.detail);
            let columns = layout::line_width(&detail, PANEL_TAB_WIDTH);
            if let Some(at) = width.checked_sub(columns) {
                grid.put_str(x + at, y, &detail, style);
            }
        }
    }

    draw_field(grid, x, &picker.query, true, top + rows, width);
}

fn rule(grid: &mut Grid, x: usize, y: usize, width: usize) {
    grid.put_str(x, y, &"─".repeat(width), CellStyle::Rule);
}

/// The right half is the document — path, language, line and column — and
/// the left is whatever is going on. The right takes what the left does not
/// need, but never more than half the screen while the left has something to
/// say, and what does not fit is cut from its start: a path loses its first
/// directories, not the line number at the end (D11). It keeps a space so it
/// never touches the edge of the screen.
fn status_line(left: &str, right: &str, width: usize) -> String {
    let right = match right.is_empty() {
        true => String::new(),
        false => format!("{right} "),
    };
    let wanted = layout::line_width(left, PANEL_TAB_WIDTH);
    let allowed = match wanted {
        0 => width,
        _ => (width / 2).max(width.saturating_sub(wanted)),
    };
    let right = tail(&right, allowed);
    let columns = layout::line_width(&right, PANEL_TAB_WIDTH);
    let left = pad(left, width - columns);
    format!("{left}{right}")
}

/// The last `columns` of `text`, with an ellipsis where the rest was.
fn tail(text: &str, columns: usize) -> String {
    let width = layout::line_width(text, PANEL_TAB_WIDTH);
    if width <= columns {
        return text.to_owned();
    }
    if columns == 0 {
        return String::new();
    }
    // Skip whole clusters until the remainder fits beside the ellipsis.
    let keep = columns - 1;
    let from = layout::cells(text, PANEL_TAB_WIDTH)
        .find(|cell| width - cell.column <= keep)
        .map_or(text.len(), |cell| cell.byte);
    pad(&format!("…{}", &text[from..]), columns)
}

/// Each field is a labelled line with the caret drawn on it, since the real
/// terminal cursor stays hidden and we paint our own everywhere else.
fn draw_prompt(grid: &mut Grid, x: usize, prompt: &Prompt, top: usize, width: usize, height: usize) {
    for (index, field) in prompt.fields.iter().take(height).enumerate() {
        draw_field(grid, x, field, index == prompt.active, top + index, width);
    }
}

/// The caret and the selection are painted by us, since the terminal's own
/// cursor stays hidden everywhere in this editor.
fn draw_field(grid: &mut Grid, x: usize, field: &Field, active: bool, y: usize, width: usize) {
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
    for cell in layout::cells(&line, PANEL_TAB_WIDTH) {
        let glyph = match cell.glyph {
            Glyph::Text(text) => text,
            _ => " ",
        };
        let style = if caret == Some(cell.column) {
            CellStyle::Cursor
        } else if active && selected.contains(&cell.column) {
            CellStyle::Selected
        } else {
            CellStyle::Normal
        };
        grid.put(x + cell.column, y, glyph, cell.width, style);
    }
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
    use crate::editor::command::{Command, Edit, PanelCommand, Select};
    use crate::editor::panel::prompt::Purpose;
    use crate::text::movement::{Direction, Motion};
    use crate::text::selection::Selection;
    use crate::text::Text;

    fn editor_of(content: &str, width: usize, height: usize) -> Editor {
        let mut editor = Editor::new(Buffer::from_text(Text::from_str(content)));
        editor.resize(width, height);
        editor
    }

    fn composed(editor: &Editor, width: usize, height: usize) -> Grid {
        let mut grid = Grid::new(width, height);
        compose(&mut grid, editor);
        grid
    }

    fn style_at(grid: &Grid, x: usize, y: usize) -> CellStyle {
        grid.cell(x, y).unwrap().style
    }

    /// The `columns` of `line` drawn on a row, as it reads on the grid.
    fn drawn(line: &str, columns: Range<usize>) -> String {
        let mut grid = Grid::new(columns.len(), 1);
        draw_row(&mut grid, 0, 0, columns, line, 4, &LineHighlight::default());
        grid.row_text(0)
    }

    #[test]
    fn text_is_padded_to_the_full_width() {
        assert_eq!(drawn("ab", 0..8), "ab      ");
        assert_eq!(drawn("", 0..8), "        ");
    }

    #[test]
    fn tabs_are_expanded_in_place() {
        assert_eq!(drawn("a\tb", 0..8), "a   b   ");
    }

    #[test]
    fn a_row_starting_mid_line_clips_from_the_left() {
        assert_eq!(drawn("abcdefgh", 2..6), "cdef");
    }

    #[test]
    fn a_character_cut_by_an_edge_becomes_blanks() {
        // The wide character straddles the right edge, then the left one.
        assert_eq!(drawn("abc日", 0..4), "abc ");
        assert_eq!(drawn("日abc", 1..5), " abc");
    }

    #[test]
    fn control_characters_are_replaced() {
        assert_eq!(drawn("a\u{7}b", 0..3), "a·b");
    }

    /// The window a row is drawn through follows the view's scrolling.
    #[test]
    fn a_scrolled_view_draws_the_line_from_its_left_column() {
        let mut editor = editor_of("abcdefgh\n", 6, 3);
        let gutter = editor.view().gutter;
        assert_eq!(editor.view().width(), 6 - gutter);

        editor.execute(Command::Move {
            motion: Motion::LineEdge(Direction::Forward),
            extend: false,
        });
        assert!(editor.view().left_column > 0);
        let expected = &"abcdefgh "[editor.view().left_column..][..editor.view().width()];
        assert_eq!(&composed(&editor, 6, 3).row_text(0)[gutter..], expected);
    }

    #[test]
    fn a_caret_and_a_selection_style_their_cells() {
        let mut editor = editor_of("hello", 80, 5);
        editor.execute(Command::Select(Select::Set(
            Selection {
                anchor: 1,
                head: 3,
                goal_column: None,
            }
            .into(),
        )));
        let grid = composed(&editor, 80, 5);
        let gutter = editor.view().gutter;

        let styles: Vec<_> = (0..5).map(|column| style_at(&grid, gutter + column, 0)).collect();
        assert_eq!(
            styles,
            [
                CellStyle::Normal,
                CellStyle::Selected,
                CellStyle::Selected,
                CellStyle::Cursor,
                CellStyle::Normal,
            ]
        );
    }

    #[test]
    fn the_text_sits_in_the_band_with_the_status_bar_on_the_last_row() {
        let editor = editor_of("hello\n", 200, 4);
        let margin = editor.view().area.x;
        assert!(margin > 0);

        let grid = composed(&editor, 200, 4);
        let row = grid.row_text(0);
        assert_eq!(&row[..margin], " ".repeat(margin));
        assert!(row[margin..].starts_with("1 hello"));
        assert!(grid.row_text(1)[margin..].starts_with("2 "));
        assert_eq!(grid.row_text(2).trim(), "");
        assert!(grid.row_text(3).trim_end().ends_with("2 lines · 1:1"));
        assert_eq!(style_at(&grid, margin, 3), CellStyle::Status);

        // A terminal narrower than the cap keeps the editor at the left edge.
        let editor = editor_of("hello\n", 80, 4);
        assert_eq!(editor.view().area.x, 0);
        assert!(composed(&editor, 80, 4).row_text(0).starts_with("1 hello"));
    }

    /// The loop resizes before it draws, but nothing else guarantees the
    /// order: a view that has not been given an area draws no text yet,
    /// and must not fall over doing so.
    #[test]
    fn a_frame_can_be_composed_before_the_first_resize() {
        let editor = Editor::new(Buffer::from_text(Text::from_str("one\n")));
        let grid = composed(&editor, 30, 3);
        // No screen divided yet means no area for anything.
        assert!((0..3).all(|y| grid.row_text(y).trim().is_empty()));
    }

    #[test]
    fn only_the_current_line_paints_its_gutter() {
        let editor = editor_of("one\ntwo\n", 80, 5);
        let grid = composed(&editor, 80, 5);

        assert_eq!(style_at(&grid, 0, 0), CellStyle::GutterCurrent);
        assert_eq!(style_at(&grid, 2, 0), CellStyle::Cursor);
        assert_eq!(style_at(&grid, 3, 0), CellStyle::CurrentLine);
        assert_eq!(style_at(&grid, 0, 1), CellStyle::Gutter);
        assert_eq!(style_at(&grid, 3, 1), CellStyle::Normal);
    }

    #[test]
    fn a_prompt_takes_its_rows_from_the_text_and_carries_its_caret() {
        let mut editor = editor_of("one\ntwo\n", 40, 8);
        editor.execute(Command::Panel(PanelCommand::Prompt(Purpose::Search)));
        editor.execute(Command::Edit(Edit::Insert("tw".to_owned())));
        editor.resize(40, 8);
        let grid = composed(&editor, 40, 8);

        assert!(grid.row_text(4).starts_with("───"));
        assert_eq!(style_at(&grid, 0, 4), CellStyle::Rule);
        assert!(grid.row_text(5).starts_with(" Find: tw"));
        assert_eq!(style_at(&grid, 9, 5), CellStyle::Cursor);
        assert!(grid.row_text(6).starts_with("───"));
        // The status bar is composed from the editor as well: the search
        // reports there, in the left half, while it is open.
        assert!(grid.row_text(7).starts_with(" Found: 1/1"));
        assert_eq!(style_at(&grid, 0, 7), CellStyle::Status);

        // The hit the search landed on is selected in the text above.
        assert_eq!(style_at(&grid, 2, 1), CellStyle::Selected);
    }

    /// What was sent, with the escape sequences taken out.
    fn visible(sent: &[u8]) -> String {
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

    #[test]
    fn a_frame_after_a_keystroke_costs_its_difference() {
        let mut editor = editor_of("hello\n", 80, 5);
        let mut renderer = Renderer::new();
        let mut first = Vec::new();
        renderer.draw(&mut first, &editor, (80, 5)).unwrap();
        assert!(visible(&first).contains("1 hello"));

        editor.execute(Command::Edit(Edit::Insert("H".to_owned())));
        let mut second = Vec::new();
        renderer.draw(&mut second, &editor, (80, 5)).unwrap();

        // Only the cells that changed: the second `l` landed where an `l`
        // already was, so it is not among them. The status bar follows suit:
        // the modified mark pushes the name over, and the column advances.
        assert_eq!(visible(&second), "Hhelo[scratch]*2");
        assert!(second.len() * 4 < first.len(), "{} vs {}", second.len(), first.len());
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

    /// D11: a long path used to take the whole bar and leave the message no
    /// room at all.
    #[test]
    fn a_long_path_gives_way_to_the_message_and_loses_its_start() {
        let line = status_line(" changed", "/very/long/path/to/file.rs · Rust · 1:1", 40);
        assert_eq!(columns(&line), 40);
        assert!(line.starts_with(" changed"));
        assert!(line.ends_with("Rust · 1:1 "), "{line:?}");
        assert!(line.contains('…'));

        // With nothing to say on the left, the right may have it all.
        let line = status_line("", "/very/long/path/to/file.rs · Rust · 1:1", 40);
        assert!(line.starts_with("/very/long"));

        // The left never pushes the right below half the screen.
        let line = status_line(&"m".repeat(60), "/p/file.rs · 1:1", 40);
        assert_eq!(columns(&line), 40);
        assert!(line.ends_with("/p/file.rs · 1:1 "));
        assert!(line.starts_with("mmmm"));
    }

    #[test]
    fn a_tail_keeps_whole_clusters() {
        assert_eq!(tail("abcdef", 4), "…def");
        assert_eq!(tail("abc", 4), "abc");
        // `…日本` would be five columns; a wide cluster goes whole or not at all.
        assert_eq!(tail("ab日本", 4), "…本 ");
        assert_eq!(tail("abcdef", 1), "…");
        assert_eq!(tail("abcdef", 0), "");
    }
}
