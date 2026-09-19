//! How a cell is drawn, as the editor sees it. A closed list for now (R5):
//! syntax highlighting and themes will want something richer, and the
//! terminal layer that paints these does not care what.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellStyle {
    #[default]
    Normal,
    CurrentLine,
    Matched,
    Selected,
    Cursor,
    Gutter,
    GutterCurrent,
    Rule,
    Status,
}
