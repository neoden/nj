# 22. A search outlives its prompt

Date: 2026-10-03

## Status

Accepted. Amends ADR 12, which accepted that a search cannot stay open while
the text is edited.

## Context

ADR 12 made the panel modal: an open panel owns the keyboard, so nothing has
to say where the keyboard is. The price it named was the search. `Esc`, edit,
`Ctrl+F` again — and the search itself was gone the moment its prompt closed:
no highlighted hits, no count, only a remembered query for `F3` to scan for
again.

The usual answer is the find widget of VS Code and Zed, which stays on screen
while the text is edited. It is harder to use than it looks, and the reason is
that it draws two things as one. There is the search — a query, its hits, the
highlighting, a count — and there is the field the query is typed into. With
both in one widget there are four states: no search, a search with the
keyboard in the field, a search with the keyboard in the text, and a closed
widget whose query `F3` still remembers. `Enter`, `Esc`, `Ctrl+Z` and the
arrows mean different things in each, and what tells them apart is where a
caret happens to blink. The count goes stale when the cursor moves in the
text. Coming back to the field replaces the query with whatever is selected.

Keeping the panel modal and giving up the search was one way out. Building a
panel that shares the keyboard with the text was the other, and it is the
start of focus as a piece of state of its own.

## Decision

The search and the field are two things with two lifetimes.

The field stays as modal as any other prompt: it opens, a query is typed,
`Esc` closes it. The search it started stays. Its hits remain highlighted, the
status bar counts them, `F3` walks them, and the text can be edited in the
meantime. No panel is open while this goes on, so the keyboard is in the text
and ADR 12's rule holds as it stood.

- The hits are offsets into the text, so they are found again whenever the
  text changes: after a transaction, an undo, a redo, and when another file
  takes the buffer's place.
- Which hit is current is not kept. It is the hit the primary cursor stands
  at the start of, or none. The status bar shows `3/17 "query"` in the first
  case and `17 "query"` in the second; the count comes first and the query is
  cut to a fixed width, so that a long query costs neither the count nor the
  document's half of the bar. While the field is open the query is in it, and
  the bar shows the count alone.
- `Esc` in the text gives up one thing at a time: the search, then the extra
  cursors, then the selection. Ending a search puts out its hits and keeps
  its query, and `F3` brings the search back.
- `Ctrl+F` opens the field holding the selected fragment, or else the last
  query, selected so that typing replaces it. Opening moves no cursor; the
  first keystroke or `Enter` is what goes to a hit.
- A field closed with nothing in it started no search. The query there was
  before stays the last one, so wiping the field and leaving does not cost
  `F3` what it would go back to.
- The hit gone to is a selection, as it was. Replacing hits one at a time
  therefore needs nothing of its own: `F3`, type over the selection, `F3`.
  Replacing all of them puts a cursor on each, as before.
- While a search is going on, `Ctrl+Shift+L` puts a cursor on every hit of
  it, in the text as in the field: what is lit is what gets the cursors,
  wherever the cursor stands. Only with no search does it go by the word
  under the cursor.
- A command that turns the hits into cursors — one on every hit, or
  replacing them all — ends the search, as `Esc` would. Otherwise the next
  `Esc` goes to a search whose hits are already in hand, and after a
  replacement the bar counts hits of something no longer in the text.

## Consequences

- There are two states, not four: a search is going on or it is not. The
  field being open is the panel's state and was there already.
- The query cannot be edited in place while the text has the keyboard, and
  only its start is on screen when it is long. `Ctrl+F` shows it whole.
- While a search is going on, every edit scans the whole text again. An edit
  can make or break a hit anywhere, so shifting the old hits is not enough.
  This is unnoticeable in ordinary files and will not be in very large ones;
  the answer then is a job (ADR 21) or a scan limited to what the edit
  touched. An ended search keeps no hits and costs nothing.
- Typing with a hit selected replaces it. That is the price of the hit being
  a selection, and one undo takes it back.
- A search that finds nothing is still a search: the bar says `0`, and `Esc`
  ends it.
- The keymap gains a context, the text with a search going on, which is what
  lets `Esc` mean three things in order by being listed three times (ADR 8).
- The search belongs to the editor, not to a view or a buffer. Opening
  another file carries it over, which is what `F3` did before. A second view
  (ADR 15) will have to say whether the two share one search.
- A stepwise replace with a question at each hit is not built. If typing over
  the selection turns out not to be enough, it is a `Confirm`, not a second
  kind of panel.
- This is the first thing on screen that is neither the text nor a panel: a
  state of the text with its own keys and its own part of the status bar. A
  completion list will be the second, and will want what the search did not
  need — a place of its own on screen and only some of the keys.
