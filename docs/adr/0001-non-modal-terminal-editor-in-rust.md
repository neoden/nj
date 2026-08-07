# 1. A non-modal terminal editor in Rust

Date: 2026-08-06

## Status

Accepted

## Context

The editor is written for daily use by someone whose muscle memory comes from
Sublime Text and VS Code, not from vi or Emacs. It has to open files named on
the command line and run in a terminal rather than as a GUI application.

Rust is chosen for the same reasons other recent editors chose it: predictable
performance without a garbage collector, and a mature ecosystem for the pieces
we will need later (tree-sitter, LSP clients, Unicode segmentation).

## Decision

Build a non-modal, terminal-based editor in Rust. Editing keys work exactly as
they do in VS Code; there is no command mode and no leader key. Files are opened
from `argv`.

## Consequences

- Every keystroke either runs a command or inserts text. There is no mode to
  fall back on, so the keymap must resolve conflicts by context instead
  (see ADR 8).
- The terminal, not the editor, owns the font, the cell grid and part of the
  keyboard. This constrains both input (ADR 6, ADR 7) and rendering (ADR 5) in
  ways a GUI editor never faces.
- Cross-platform support is largely free through `crossterm`, but is not a goal
  in itself; Linux is the development target.
