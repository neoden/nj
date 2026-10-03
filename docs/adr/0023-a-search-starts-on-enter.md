# 23. A search starts on Enter; a replacement asks at each hit

Date: 2026-10-03

## Status

Accepted. Amends ADR 22, which had the first keystroke in the field go to a
hit and had a replacement change every hit at once.

## Context

The search field searched as it was typed. Every keystroke in it found the
hits of the query so far, lit them, and put the selection on the first one at
or after where the cursor stood when the field opened.

What that bought was news before `Enter`: a mistyped query showed `0` at
once, and the count stood on the status bar before a replacement was made.

What it cost was more than it looked:

- The screen moved at every keystroke. Typing `f` on the way to `foo` went to
  the first `f` in the file.
- The field had to remember where the cursors stood when it opened, to search
  from there at every keystroke and to put them back when nothing was found.
- The hits were sifted as the query grew, which is a second way of finding
  them that exists only for typing letter by letter. The first letter was the
  dearest: one letter in a large file is hundreds of thousands of hits.
- Typing in the `Replace:` field ran the same machinery, though the query had
  not changed, and threw the selection back to the first hit.

The replacement had one answer, all of them at once. Going one at a time was
left to `F3` and typing over the selection, which means typing the
replacement again at every hit. What a replacement usually offers is a choice
at each hit, with all of them as one of the answers.

## Decision

Typing in a search field does nothing to the text. `Enter` starts the search.

- `Ctrl+F`, a query, `Enter`: the field closes, the hits light up, and the
  selection goes to the first hit at or after the cursor. `Shift+Enter` goes
  to the first one before it. From there on it is the search of ADR 22: `F3`
  walks it, the status bar counts it, `Esc` ends it.
- A query that finds nothing closes the field all the same, leaves the
  cursors where they were, and says `Not found`.
- Until `Enter`, a search already going on goes on. `Esc` in the field leaves
  it as it was, and so does `Enter` on an empty field.
- The field still opens holding the selected fragment, or else the last
  query, selected so that typing replaces it.
- `Ctrl+Shift+L` on every hit is a command of the text with a search going
  on. A field has no hits of its own to give it.

A replacement asks at each hit.

- `Ctrl+H`, a query, `Tab`, a replacement, `Enter`: the field closes, the
  hits light up, the selection goes to the first hit at or after the cursor,
  and a question takes the field's place:
  `Replace with "bar"?  [y] Replace  [n] Skip  [a] Replace all  [q] Stop`.
- `y` replaces the selected hit and goes to the next one. `n` goes to the
  next one. `a` replaces this hit and every one not asked about yet. `q` and
  `Esc` stop.
- The walk goes forward to the end of the text, comes round to its top, and
  stops where it began: every hit is asked about once. Then the question
  closes and the status bar says how many were replaced.
- Each `y` is an undo step of its own. `a` is one step for all it replaces.
- The question is a `Confirm`, the panel that already asks before
  overwriting a file. It is modal like any other: a click in the text does
  not dismiss it.
- When the question closes, however it closes, the search is over, as it was
  after replacing all.

## Consequences

- There is one way to find the hits, a scan of the text, and it runs when
  `Enter` is pressed and when the text changes under a search. The sifting
  goes, and so does the one-letter query nobody meant to run.
- A prompt no longer remembers where the cursors stood. Nothing moves them
  until `Enter`, so there is nothing to put back.
- A field edit no longer drives a search, whichever field it is in. The
  `Replace:` field dragging the selection to the first hit cannot happen.
- A mistyped query is found out one keystroke later, at `Enter`, and costs
  opening the field again. It opens holding what was typed.
- The count is on screen before the first replacement, because the hits are
  lit before the question is asked. Replacing blind is not possible.
- `a` at the first question is every hit in the text, wherever the cursor
  stood. After some answers it is the ones not asked about yet: a hit skipped
  with `n` stays as it is.
- The walk has to remember where it began, and that place moves when a
  replacement is made before it, which happens once the walk has come round.
- A `Confirm` so far closed for good on any answer. This one comes back for
  the next hit, so the question and what it is asked about have to live
  across answers.
- The hits are found again after each `y`, as after any edit (ADR 22). A
  replacement that contains the query is not offered to itself: the next hit
  is looked for after the text just put in.
- `Enter` in the search field no longer walks the hits; with the field closed
  that is `F3`, and nothing else.
