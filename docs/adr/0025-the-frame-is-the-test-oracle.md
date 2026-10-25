# 25. The frame is the test oracle; a scenario is a sequence of events

Date: 2026-10-25

## Status

Accepted. Builds on ADR 16, which made a test a sequence of commands, and on
ADR 21, which kept tests driving the editor without a thread or a terminal.

## Context

The editor can be driven without a terminal: `app::handle` takes a crossterm
event, `render::compose` draws the whole frame into a `Grid`. The tests in
`tests/dispatch.rs` use the first half of this. Their oracle is internal:
byte offsets, selection ranges, which panel variant is open. Nothing checks
what a user would see, so a frame that wraps, scrolls or lays a panel out
wrongly passes every test as long as the offsets are right.

Trying the editor in a real terminal shows the frame but costs the opposite:
a scenario is a hand-driven pty session, its timing is not reproducible, and
its output is a stream of escape sequences that has to be decoded before it
can be read. Each scenario written in Rust costs a compile and a reading of
the test to understand what it asserts.

Events and commands are not the same thing (ADR 16). The layer between them —
`translate`, the keymap and its contexts, which panel owns the keyboard — is
where behaviour is decided, and a scenario made of commands skips it.

## Decision

- The oracle is the frame. A scenario's expectations are about the text of
  the `Grid` that `compose` produces, dumped as one line per screen row.
  Under a row, when there is anything to show, a second line gives the
  style of each cell as a letter. Every `CellStyle` has a letter, assigned
  in one exhaustive `match`, so a new style cannot be invisible to a dump.
  Trailing blanks are cut; runs of empty rows collapse into a count.
- A scenario is a sequence of events, not commands: keys by their printed
  name (`Ctrl+Shift+L`, the form `Key` already displays), typed text,
  pastes, mouse actions, resizes, and a wait for background jobs. The
  events go through `app::handle`, so a scenario crosses everything a real
  keystroke crosses except the terminal itself. Commands may later appear
  in a scenario to set a state up, never as the thing being checked.
- One driver, an example binary, reads a scenario from standard input and
  prints dumps. The first dump of a run is the whole frame; every later one
  is the rows that changed since the previous dump, with their numbers.
- A scenario kept for regression is a file beside its expected output, and
  the expected output is exactly what the driver prints. One test runs them
  all and shows a line diff on a mismatch. Expected files are plain text,
  never compressed: they are read in review and in `git blame`, and git
  compresses its objects anyway.
- The pty is kept for the layer under the `Grid` only: the keyboard protocol
  negotiation, bracketed paste, resize, the escape stream. That is a smoke
  run, not a test suite.

## Consequences

- An expected file reads as a record of behaviour: these keys, this screen.
  It is what the owner checks when the question is "what does this do".
- The frame changes when the chrome changes. A new gutter width or status
  layout fails every scenario that shows it; expected files are regenerated
  on request and the regeneration is reviewed as a diff, like any change.
- A scenario that opens or saves files needs a directory of its own, and one
  that uses the clipboard needs a fake `Desktop`. The driver provides both.
- Only what reaches the `Grid` is checked. The caret is a cell style, so it
  is covered; a scrollbar or a second window will be covered the day it is
  drawn, under the one rule that its style has a letter.
- The dump is shaped by the screen size the scenario sets. Small screens are
  the norm: they are cheaper to read and they exercise wrapping and
  scrolling that a 80×24 frame with three lines of text never shows.
- Key names get a parser, the inverse of `Key`'s `Display`. The keymap
  overrides of ADR 8 need the same parser.
