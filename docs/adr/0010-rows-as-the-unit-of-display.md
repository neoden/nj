# 10. Rows, not lines, as the unit of display

Date: 2026-08-07

## Status

Accepted

## Context

Soft wrap is wanted. Without it, a view can assume that one logical line
occupies one row of the screen, and that assumption spreads: the scroll anchor
is a line number, the renderer loops over lines, a click maps a screen row to a
line, and vertical movement steps by lines.

Adding soft wrap later would mean changing all of those at once. Introducing
the distinction now costs almost nothing, because with wrapping disabled every
line still yields exactly one row.

## Decision

The view produces `Row { line, index, start_column }`: the part of a logical
line drawn on one screen row. Rendering, click mapping and cursor placement all
go through rows. Rows are produced by `rows_in_line` and `row_start_column`,
the only two functions that soft wrap will have to change.

Horizontal scrolling is kept separate from `start_column`. Scrolling is a
property of the window; wrapping is a property of the line's layout. Folding
them together would make `goal_column` relative to a moving origin.

## Consequences

- Enabling soft wrap becomes a change to two functions plus a break-column
  algorithm, rather than a change spread across four modules.
- Wrapped continuation rows draw an empty gutter, since the line number
  belongs to the line rather than to each of its rows.
- Vertical movement has to become a view operation rather than a buffer one:
  moving down a row depends on the width of the window. `movement.rs` keeps the
  motions that depend only on the buffer.
- Absolute row numbering across the document is deliberately not maintained.
  Rows are walked from the scroll anchor with `next_row`/`previous_row`, which
  avoids the cache of per-line row counts that absolute numbering would need,
  at the cost of an approximate scrollbar position later.
