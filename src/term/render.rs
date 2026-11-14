//! Composes a frame into the grid. Nothing here writes to the
//! terminal: the grid is filled from the editor's state, and the renderer
//! flushes what changed since the frame before.

use std::io::Write;
use std::ops::Range;

use anyhow::Result;
use crossterm::queue;
use crossterm::terminal::{Clear, ClearType};

use super::grid::{CONTROL_PLACEHOLDER, Grid, based};
use crate::editor::Editor;
use crate::editor::panel::Panel;
use crate::editor::panel::field::Field;
use crate::editor::panel::picker::Picker;
use crate::editor::style::{CellStyle, Ink, Mark};
use crate::editor::view::{LineHighlight, Window};
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
    /// the previous frame away, since nothing on it is where it was, and
    /// clears the terminal, since the frame may now cover less of it.
    pub fn draw(
        &mut self,
        out: &mut impl Write,
        editor: &Editor,
        (width, height): (usize, usize),
    ) -> Result<()> {
        if self.scratch.width() != width || self.scratch.height() != height {
            self.scratch = Grid::new(width, height);
            self.previous = None;
            queue!(out, Clear(ClearType::All))?;
        }
        self.scratch.clear();
        compose(&mut self.scratch, editor);
        self.scratch.flush(out, self.previous.as_ref())?;

        let spare = self
            .previous
            .take()
            .unwrap_or_else(|| Grid::new(width, height));
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
    let ui_layout = editor.ui_layout();
    let x = view.area.x;
    let width = view.area.width;

    for (row, y) in view.rows(buffer).zip(view.area.y..) {
        let left = row.start_column + view.left_column;
        let columns = left..left + view.width();
        let window = Window::of(buffer, row.line, columns.clone());
        // Only the tokens that show are read: a line may be far longer than
        // the screen is wide. An empty range is not asked about, since to
        // tree-sitter a range ending at zero has no end.
        let tokens = match buffer.syntax() {
            Some(syntax) if !window.bytes().is_empty() => {
                syntax.tokens(buffer.text(), window.bytes())
            }
            _ => Vec::new(),
        };
        let highlight = view.highlight_line(buffer, &window, editor.matches(), &tokens);
        // Wrapped continuations leave the gutter empty: the number belongs to
        // the logical line, not to every row it spans.
        let number = match row.index {
            0 => format!("{:>1$} ", row.line + 1, view.gutter - 1),
            _ => " ".repeat(view.gutter),
        };
        let gutter = CellStyle {
            ink: Ink::Gutter,
            mark: match highlight.is_current() {
                true => Mark::CurrentLine,
                false => Mark::None,
            },
        };
        grid.put_str(x, y, &number, gutter);
        draw_row(
            grid,
            x + view.gutter,
            y,
            columns,
            window.shown(),
            buffer.tab_width(),
            &highlight,
        );
    }

    // What the panel asked for is not what it got: only the screen knows how
    // much was left over, and it may have granted nothing at all.
    let body = ui_layout.panel_height.saturating_sub(2);
    if let Some(panel) = editor.panel().filter(|_| body > 0) {
        let top = ui_layout.panel_top() + 1;
        rule(grid, x, ui_layout.panel_top(), width);
        match panel {
            Panel::Find(find) => draw_field(grid, x, &find.query, true, top, width),
            Panel::GotoLine(goto_line) => draw_field(grid, x, &goto_line.line, true, top, width),
            Panel::OpenFile(open_file) => draw_field(grid, x, &open_file.path, true, top, width),
            Panel::Replace(replace) => {
                let fields = [
                    (&replace.query, !replace.on_replacement),
                    (&replace.replacement, replace.on_replacement),
                ];
                for (row, (field, active)) in fields.into_iter().take(body).enumerate() {
                    draw_field(grid, x, field, active, top + row, width);
                }
            }
            Panel::SaveAs(save_as) => draw_field(grid, x, &save_as.path, true, top, width),
            Panel::Picker(picker) => draw_picker(grid, x, picker, top, width, body),
            Panel::Confirm(confirm) => {
                for (row, line) in confirm.rows().iter().take(body).enumerate() {
                    grid.put_str(x, top + row, &pad(line, width), CellStyle::default());
                }
            }
            Panel::Replacing(replacing) => {
                grid.put_str(x, top, &pad(&replacing.line(), width), CellStyle::default());
            }
        }
        rule(grid, x, ui_layout.status_row() - 1, width);
    }

    let (left, right) = status(editor);
    grid.put_str(
        x,
        ui_layout.status_row(),
        &status_line(&left, &right, width),
        CellStyle::of(Ink::Status),
    );
}

/// The right side is the document, always in the same place. The left side is
/// whatever is going on right now, so the eye only goes there when something
/// is happening.
fn status(editor: &Editor) -> (String, String) {
    let (view, buffer) = (editor.view(), editor.buffer());
    let line = buffer.text().line_of(view.selections().primary().head);
    let column = editor.cursor_column();

    let right = format!(
        "{}{} · {} · {} lines · {}:{}",
        buffer.name(),
        if buffer.is_modified() { "*" } else { "" },
        buffer.language().name,
        buffer.text().line_count(),
        line + 1,
        column + 1,
    );

    let left = match editor.message() {
        Some(message) => message.to_owned(),
        None => {
            let cursors = view.selections().len();
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

    (left, right)
}

/// The display `columns` of a line, drawn on one row of the grid starting at
/// `x`, blank where the line has nothing. A cluster cut by either edge
/// becomes blanks too, because half a character cannot be drawn. The line
/// is given as `(part, column)`: from a cell at or before the left edge
/// on, with the column that cell starts in — the whole line and zero, or
/// less of it where the line is long.
fn draw_row(
    grid: &mut Grid,
    x: usize,
    y: usize,
    columns: Range<usize>,
    (part, from): (&str, usize),
    tab_width: usize,
    highlight: &LineHighlight,
) {
    let Range {
        start: left,
        end: right,
    } = columns;
    let mut painter = Painter {
        grid,
        x,
        y,
        left,
        highlight,
    };
    let mut column = left;

    for cell in layout::cells_from(part, from, tab_width) {
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
                Glyph::Bare(text) => painter.put(cell.column, &based(text), 1),
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
        let style = CellStyle {
            ink: self.highlight.ink_at(column),
            mark: self.highlight.mark_at(column),
        };
        self.grid
            .put(self.x + column - self.left, self.y, cluster, width, style);
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
fn draw_picker(
    grid: &mut Grid,
    x: usize,
    picker: &Picker,
    top: usize,
    width: usize,
    height: usize,
) {
    // The query keeps the bottom row; candidates take what is left of a strip
    // that may be shorter than the list, best matches nearest the query.
    let rows = picker.visible(height - 1).count();

    for (row, (index, candidate)) in picker.visible(rows).enumerate() {
        let style = CellStyle::marked(match index == picker.selected {
            true => Mark::Selected,
            false => Mark::None,
        });
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
    grid.put_str(x, y, &"─".repeat(width), CellStyle::of(Ink::Rule));
}

/// The right half is the document — path, language, line and column — and
/// the left is whatever is going on. The right takes what the left does not
/// need, but never more than half the screen while the left has something to
/// say, and what does not fit is cut from its start: a path loses its first
/// directories, not the line number at the end. A column at either
/// edge keeps both off the screen's edges, and one between them, while the
/// left has something to say, keeps the two apart.
fn status_line(left: &str, right: &str, width: usize) -> String {
    let message = layout::line_width(left, PANEL_TAB_WIDTH);
    let gap = match message {
        0 => "",
        _ => " ",
    };
    let room = width.saturating_sub(2 + gap.len());
    // The document's half of the bar includes its margin.
    let half = (width / 2).saturating_sub(1);
    let allowed = match message {
        0 => room,
        _ => half.max(room.saturating_sub(message)),
    };
    let right = tail(right, allowed);
    let columns = layout::line_width(&right, PANEL_TAB_WIDTH);
    let left = pad(left, room - columns);
    format!(" {left}{gap}{right} ")
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

/// A field is a one-line text drawn through a window, the way a row of the
/// text is: the same painter, with the window sliding so that the caret
/// stays in it. The caret and the selection are painted by us, since the
/// terminal's own cursor stays hidden everywhere in this editor.
fn draw_field(grid: &mut Grid, x: usize, field: &Field, active: bool, y: usize, width: usize) {
    let contents = field.contents();
    let label = format!(" {} ", field.label);
    let offset = layout::line_width(&label, PANEL_TAB_WIDTH).min(width);
    grid.put_str(x, y, &pad(&label, offset), CellStyle::default());

    let room = width - offset;
    let column_of = |byte| layout::column_of_byte(&contents, byte, PANEL_TAB_WIDTH);
    let caret = column_of(field.cursor);
    let left = (caret + 1).saturating_sub(room);

    let selection = field.selection();
    let highlight = match active {
        true => LineHighlight::field(caret, column_of(selection.start)..column_of(selection.end)),
        false => LineHighlight::default(),
    };
    draw_row(
        grid,
        x + offset,
        y,
        left..left + room,
        (&contents, 0),
        PANEL_TAB_WIDTH,
        &highlight,
    );
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
            Glyph::Bare(text) => out.push_str(&based(text)),
            _ => out.push(' '),
        }
        column = cell.end_column();
    }

    out.extend(std::iter::repeat_n(' ', width - column));
    out
}

#[cfg(test)]
mod tests {
    use super::super::grid::visible;
    use super::*;
    use crate::buffer::Buffer;
    use crate::editor::command::{Command, Edit, PanelCommand, Select};
    use crate::text::Text;
    use crate::text::movement::{Direction, Motion};
    use crate::text::selection::Selection;

    fn editor_of(content: &str, width: usize, height: usize) -> Editor {
        let mut editor = Editor::new(Buffer::from_text(Text::from_str(content)));
        editor.resize(width, height);
        editor
    }

    #[test]
    #[cfg(feature = "rust")]
    fn tokens_colour_the_text_under_the_marks() {
        let mut buffer = Buffer::from_text(Text::from_str("fn main() {}\n"));
        buffer.set_language(crate::buffer::language::detect(std::path::Path::new(
            "x.rs",
        )));
        let mut editor = Editor::new(buffer);
        editor.resize(80, 3);
        let grid = composed(&editor, 80, 3);
        let gutter = editor.view().gutter;

        // `fn` is a keyword on the current line, under the cursor's mark too.
        assert_eq!(
            style_at(&grid, gutter, 0),
            CellStyle {
                ink: Ink::Keyword,
                mark: Mark::Cursor,
            }
        );
        assert_eq!(
            style_at(&grid, gutter + 1, 0),
            CellStyle {
                ink: Ink::Keyword,
                mark: Mark::CurrentLine,
            }
        );
        assert_eq!(style_at(&grid, gutter + 3, 0).ink, Ink::Function);
        assert_eq!(style_at(&grid, gutter + 7, 0).ink, Ink::Text);
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
        let highlight = LineHighlight::default();
        draw_row(&mut grid, 0, 0, columns, (line, 0), 4, &highlight);
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
        assert_eq!(drawn("a\u{7}b", 0..3), "a\u{fffd}b");
    }

    /// A mark with no base of its own gets one, so that the terminal draws
    /// a cell for it where the layout counted one.
    #[test]
    fn a_bare_mark_is_drawn_on_a_base() {
        assert_eq!(drawn("\u{301}z", 0..2), "\u{25cc}\u{301}z");
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

        let styles: Vec<_> = (0..5)
            .map(|column| style_at(&grid, gutter + column, 0))
            .collect();
        assert_eq!(
            styles,
            [
                CellStyle::default(),
                CellStyle::marked(Mark::Selected),
                CellStyle::marked(Mark::Selected),
                CellStyle::marked(Mark::Cursor),
                CellStyle::default(),
            ]
        );
    }

    #[test]
    fn the_text_starts_at_the_left_edge_with_the_status_bar_on_the_last_row() {
        let editor = editor_of("hello\n", 200, 4);
        assert_eq!(editor.view().area.x, 0);

        let grid = composed(&editor, 200, 4);
        assert!(grid.row_text(0).starts_with("1 hello"));
        assert!(grid.row_text(1).starts_with("2 "));
        assert_eq!(grid.row_text(2).trim(), "");
        assert!(grid.row_text(3).trim_end().ends_with("2 lines · 1:1"));
        assert_eq!(style_at(&grid, 0, 3), CellStyle::of(Ink::Status));
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

        assert_eq!(
            style_at(&grid, 0, 0),
            CellStyle {
                ink: Ink::Gutter,
                mark: Mark::CurrentLine,
            }
        );
        assert_eq!(style_at(&grid, 2, 0), CellStyle::marked(Mark::Cursor));
        assert_eq!(style_at(&grid, 3, 0), CellStyle::marked(Mark::CurrentLine));
        assert_eq!(style_at(&grid, 0, 1), CellStyle::of(Ink::Gutter));
        assert_eq!(style_at(&grid, 3, 1), CellStyle::default());
    }

    #[test]
    fn a_prompt_takes_its_rows_from_the_text_and_carries_its_caret() {
        let mut editor = editor_of("one\ntwo\n", 40, 8);
        editor.execute(Command::Panel(PanelCommand::Find));
        editor.execute(Command::Edit(Edit::Insert("tw".to_owned())));
        // `Enter` starts the search and closes the strip; opened again, it
        // holds the hit that is selected.
        editor.execute(Command::Panel(PanelCommand::Search(Direction::Forward)));
        editor.execute(Command::Panel(PanelCommand::Find));
        editor.resize(40, 8);
        let grid = composed(&editor, 40, 8);

        assert!(grid.row_text(4).starts_with("───"));
        assert_eq!(style_at(&grid, 0, 4), CellStyle::of(Ink::Rule));
        assert!(grid.row_text(5).starts_with(" Find: tw"));
        assert_eq!(style_at(&grid, 9, 5), CellStyle::marked(Mark::Cursor));
        assert!(grid.row_text(6).starts_with("───"));
        // The status bar is composed from the editor as well: a search going
        // on reports there, in the left half.
        assert!(grid.row_text(7).starts_with(" 1/1 "));
        assert_eq!(style_at(&grid, 0, 7), CellStyle::of(Ink::Status));

        // The hit the search landed on is selected in the text above.
        assert_eq!(style_at(&grid, 2, 1), CellStyle::marked(Mark::Selected));
    }

    /// A field slides under its caret, as the text does: what is typed past
    /// the right edge stays in view with the caret on the last column.
    #[test]
    fn a_field_longer_than_the_strip_keeps_its_caret_in_view() {
        let mut editor = editor_of("one\n", 20, 8);
        editor.execute(Command::Panel(PanelCommand::Find));
        editor.execute(Command::Edit(Edit::Insert(
            "abcdefghijklmnopqrstuvwxyz".to_owned(),
        )));
        editor.resize(20, 8);
        let grid = composed(&editor, 20, 8);

        assert_eq!(grid.row_text(5), " Find: opqrstuvwxyz ");
        assert_eq!(style_at(&grid, 19, 5), CellStyle::marked(Mark::Cursor));

        editor.execute(Command::Move {
            motion: Motion::LineEdge(Direction::Backward),
            extend: false,
        });
        let grid = composed(&editor, 20, 8);
        assert_eq!(grid.row_text(5), " Find: abcdefghijklm");
        assert_eq!(style_at(&grid, 7, 5), CellStyle::marked(Mark::Cursor));
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
        let line = status_line("changed", "/very/long/path/to/file.rs · Rust · 1:1", 40);
        assert_eq!(columns(&line), 40);
        assert!(line.starts_with(" changed "));
        assert!(line.ends_with("Rust · 1:1 "), "{line:?}");
        assert!(line.contains('…'));

        // With nothing to say on the left, the right may have it all but
        // the margins.
        let line = status_line("", "/very/long/path/to/file.rs · Rust · 1:1", 41);
        assert_eq!(line, " /very/long/path/to/file.rs · Rust · 1:1 ");

        // A message and a name that would fill the bar between them to the
        // last column are still kept apart.
        let line = status_line("Copied to copy.txt", "[stdin] · Text · 2 lines · 1:5", 50);
        assert_eq!(columns(&line), 50);
        assert!(line.contains("copy.txt …"), "{line:?}");

        // The left never pushes the right below half the screen.
        let line = status_line(&"m".repeat(60), "/p/file.rs · 1:1", 40);
        assert_eq!(columns(&line), 40);
        assert!(line.ends_with("/p/file.rs · 1:1 "));
        assert!(line.starts_with(" mmmm"));
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
