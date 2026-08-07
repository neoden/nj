# 8. Keymap structure: defaults in code, overrides in TOML

Date: 2026-08-06

## Status

Accepted

## Context

A non-modal editor resolves every keystroke against one table, so that table
decides how much the editor can express. Three questions had to be answered
before writing it.

Bindings differ by situation: `Esc` closes the search panel or collapses
cursors, `Tab` indents a selection or inserts a tab. Without a mechanism for
this, the logic disperses into the commands and the keymap stops being the
source of truth.

Commands take parameters. Flat command names produce `MoveWordLeftExtend`,
`MoveWordRightExtend` and dozens more; the combinations multiply.

Defaults have to exist even when no configuration file does.

## Decision

- Bindings are an ordered list, each with an optional context predicate; the
  first match wins. Predicates start as a small enum, not an expression
  language.
- Commands are an enum with fields, e.g. `Move { by, direction, extend }`.
  `serde` parses that shape from configuration without a hand-written parser.
- The default keymap is a static table in Rust. A user TOML file overrides it.
- Chords (`Ctrl+K Ctrl+B`) are supported, with the pending prefix shown in the
  status bar and no timeout.

## Consequences

- The keymap remains readable as data instead of dissolving into command code.
- The compiler checks the default table, and the editor starts with no
  configuration and cannot be bricked by a typo in one.
- Every command is nameable, which the command palette needs anyway.
- An expression language for contexts can be added later; it cannot be removed
  once bindings depend on it.
