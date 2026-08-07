# 7. Normalize keystrokes to physical keys

Date: 2026-08-06

## Status

Accepted

## Context

Under a Cyrillic layout, `Ctrl+A` arrives as `Char('ф')` and matches nothing in
the keymap. The kitty protocol does carry the answer: its key field is
`unicode-key-code:shifted-key:base-layout-key`, and the third component is the
key at that physical position on the reference layout.

crossterm 0.29 parses only the second component, and only when SHIFT is set
(`src/event/sys/unix/parse.rs`); the base layout key is discarded. It is not
reachable through the public API.

The alternatives were writing our own input parser, which drags in mouse
reporting, bracketed paste and the whole legacy escape-sequence zoo, or
patching crossterm and maintaining a fork for one field.

## Decision

Translate characters to their US-layout position with a static table, in a
single function that converts a `KeyEvent` into our own `Key` type. Translate
only when CTRL or ALT is held.

## Consequences

- `Ctrl+ф` reaches the command bound to `Ctrl+A`, in legacy terminals too,
  since the table needs no protocol support.
- Typing is untouched: combinations with CTRL or ALT never insert text, so
  plain Cyrillic input is never rewritten. Text insertion reads the raw event
  through `text_char`, not the normalized `Key`.
- The table covers one layout. Others need their own, which is a configuration
  problem, not a design one.
- `Key` is lossy by design: it drops key-release events and modifier state
  flags such as NUM_LOCK, folds `BackTab` into `Tab+SHIFT`, and moves letter
  case into the SHIFT flag.
- If the parser is ever written by hand, only this function changes.
