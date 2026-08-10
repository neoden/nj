# 15. A workspace of buffers and views

Date: 2026-08-10

## Status

Accepted. Completes ADR 9, which put selections in the view so that one file
open in two views would have independent cursors, without giving the structure
room for a second view.

## Context

There is one document and one view, both fields of the editor. Everything
written since assumes it: quitting consults a single modified flag, the picker
was built as a streaming source but has nowhere to open a file, and setting the
language synchronizes three places by hand because there is no answer yet to
what belongs to the file and what belongs to the window onto it.

ADR 17 removes the last way to avoid this: a view showing excerpts from several
files at once cannot belong to one of them.

## Decision

A workspace owns a registry of buffers and a registry of views, addressed by
`BufferId` and `ViewId`, with one of the views focused.

A buffer owns the text, the history, the identity of the file on disk and the
language. A view owns the selections, the scroll position and the area it was
given. Geometry is assigned to a view from outside rather than worked out by
it, and values derived from the buffer — the tab width among them — are read
from the buffer rather than copied into the view.

## Consequences

- ADR 9 becomes reachable rather than merely stated.
- Splits and tabs turn into a question about layout rather than a change to the
  editor's shape.
- A vector and an index are enough for the registries. Generational handles wait
  until buffers are closed and reopened often enough to need them, and no
  dependency is added for this.
- Every command written before this would have assumed a single buffer, so the
  price of the change rises with the number of commands. That is the reason for
  taking it early rather than when a second file is first opened.
