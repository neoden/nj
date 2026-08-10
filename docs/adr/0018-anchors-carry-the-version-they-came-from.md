# 18. Anchors: a position and the version it came from

Date: 2026-08-10

## Status

Accepted

## Context

A byte offset addresses a position in one version of the text. After any
insertion above it, it addresses something else. Selections survive this because
the editor holds them and displaces them through each transaction as it is
applied.

That model requires everyone holding a position to be reachable and awake at the
moment of the edit, and the holders that matter are neither: a parser working on
a snapshot taken seconds ago, a diagnostic answering a request made five
keystrokes back, an excerpt that is not on screen, the selections of a second
view onto the buffer being edited (ADR 15), the boundaries of a hunk in a review
(ADR 17).

ropey offers nothing here. It is a rope, not a marker store, and its metrics are
closed to extension. A registry of markers inside the buffer would cost an
update to every marker on every edit and a lifecycle for removing dead ones, and
it would still not reach a holder that is asleep, because what such a holder has
is a value rather than a registered handle.

## Decision

Turn it around: nobody is updated, and whoever needs a real offset asks for it.

A buffer carries a version that rises with every applied change, and an
append-only journal of what those changes were — for each, the range replaced
and the length that replaced it. An anchor is that version, an offset and a
bias. Resolving an anchor runs its offset forward through the journal from its
version to the current one, which is what `transaction::displace` already does
for a single set of changes.

Bias settles the one ambiguous case, an insertion exactly at the anchor, and has
two values: the anchor stays before the inserted text or after it. A cursor
wants the second, the start of an excerpt the first. A deletion covering an
anchor collapses it to where the deletion began.

The journal holds no text, so an entry is a range and a length. Undo and redo
are ordinary changes: they append and raise the version rather than rewinding
it. A buffer's text is private, and the only way to change it is the call that
writes the journal.

## Consequences

- An anchor is a plain value. It can be copied, sent to a worker and forgotten
  for ten seconds.
- The common case costs nothing: an anchor already at the current version
  resolves to itself.
- A bounded journal means an anchor older than its head cannot be resolved.
  Saying so is honest; the alternative is answering with a position that is
  wrong.
- Resolving many anchors at once should sort them and walk the journal once
  rather than once each.
- The journal is not the undo stack and cannot be: that one holds the removed
  text, shrinks as it is used, and is trimmed at a limit.
- ADR 2 and ADR 3 are untouched. Storage is still addressed in bytes; an anchor
  is that address and a time.
- This was the missing piece of the addressing model. Storage in bytes, movement
  in graphemes, display in columns and the human address in lines and columns
  were all separated from one another; time was not.
