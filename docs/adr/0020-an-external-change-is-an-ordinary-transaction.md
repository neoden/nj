# 20. An external change is an ordinary transaction

Date: 2026-08-10

## Status

Accepted

## Context

The editor is one of two writers to the working tree. The other is whatever runs
in the terminal beside it: a formatter, a branch being switched, an agent
editing files. Under this way of working that is the ordinary case rather than a
rare accident, so losing the cursor, the scroll position or the history every
time a file changes underneath is not something to live with.

Until now the document remembered nothing about the file since it was read, and
saving overwrote whatever was there.

Reading the file again and replacing the buffer answers none of it. Every
position in the view is invalidated at once, and the history left behind
describes a text that no longer exists — which is the open question this record
closes.

## Decision

When a file changes on disk, the new contents are diffed against the buffer and
the result is applied as a transaction, like any other change.

Modification time and size are recorded when the file is read and whenever it is
saved, and checked immediately before writing, whether or not a watch is
running. A watch still leaves a race between the event and the write, so the
check before the rename is needed either way.

## Consequences

- Cursors, scroll positions, excerpts and every other anchor come through by the
  mechanism of ADR 18, with no special case for reading a file again.
- Reading again is undoable, and the history stays coherent with the text.
- A buffer holding unsaved edits still has to ask, because a diff cannot decide
  whose version was meant. That question needs three answers — overwrite, take
  what is on disk, cancel — where the panel today offers two.
- A file deleted underneath keeps its contents and is marked as absent from the
  disk rather than emptied.
- The diff engine of ADR 19 is needed here too. Three separate features use it,
  which is why it belongs in the core rather than in a git integration.
