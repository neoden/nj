# 5. crossterm plus a hand-written grid renderer

Date: 2026-08-06

## Status

Accepted

## Context

The text view is a custom widget in any case: it virtualizes by visible line,
draws its own cursors and selections, and expands tabs itself. What a TUI
toolkit would provide is the surrounding chrome and a diffing renderer.

`ratatui` offers both, but its cell model would have to be bent around tabs,
double-width characters and combining marks, which is where most of the work
actually is.

## Decision

Use `crossterm` for input and terminal control, and write the renderer: a
shadow grid of styled cells, flushed as a diff against the previous frame.

## Consequences

- Full control over the cell model, which is the part that matters for correct
  Unicode rendering.
- The chrome (status bar, command palette, file picker) has to be written too,
  on a small immediate-mode layer over the same grid.
- Frames are queued with `queue!` and flushed once, never `execute!`, to avoid
  tearing and syscalls per drawing operation.
- If the hand-written renderer turns out to be a poor use of effort, moving to
  `ratatui` is still possible: it would replace the renderer, not the view.
