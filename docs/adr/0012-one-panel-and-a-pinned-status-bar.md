# 12. One panel and a pinned status bar

Date: 2026-08-08

## Status

Accepted

## Context

A search box, a question, a command palette and a filtered list all have to
live somewhere on screen, and something has to say where the keyboard is
pointing at any moment.

## Decision

The screen is three regions stacked vertically: the text, an optional panel of
variable height, and a status bar pinned to the last row.

- Everything that is not the text happens in the panel.
- An open panel owns the keyboard. Focus is therefore not a separate piece of
  state that can disagree with what is on screen — the panel being open *is*
  the focus.
- `Esc` closes the panel and returns to the text. With no panel open it
  collapses the selections.
- Panel kinds are variants of one enum. A new kind touches its height, its
  drawing and its key handling, and nothing else.
- The status bar's right half belongs to the document and never moves. Its left
  half belongs to whatever is happening: the search counter, the cursor count,
  messages.
- A filtered list puts its query at the bottom, next to the status bar, and
  grows upwards, so that neither the query nor the best match moves as the list
  changes size.

## Consequences

- A search cannot stay open while the text is edited, as it can in VS Code.
  `Esc`, edit, and `Ctrl+F` reopens with the previous query.
- The panel takes rows from the text when it opens. The status bar does not
  move, which is what the eye anchors on.
- Nothing needs to be said about where the keyboard is, because there is only
  one place it can be.
