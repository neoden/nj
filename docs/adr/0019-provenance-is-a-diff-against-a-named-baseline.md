# 19. Provenance is a diff against a named baseline

Date: 2026-08-10

## Status

Accepted

## Context

A review has to say where text came from, and there is more than one answer at
the same time: what the branch changed, and what the reviewer has just typed
into it while reading.

The obvious approach marks ranges with an author and carries those marks through
every transaction. They then have to survive undo, an edit arriving from outside
(ADR 20) and a file being read again — three places to lose them, and losing
them is silent.

But these are not one attribute measured twice. They are different comparisons:
against the blob the branch started from, and against the file as it stood when
the review was opened.

## Decision

A buffer holds several named baselines, each a snapshot of its text. Provenance
is computed as a diff against the baseline in question rather than stored. The
change gutter, the review sheet, the reviewer's own edits and the
unsaved-changes indicator are one mechanism with different baselines.

A diff engine is therefore core machinery, not part of a git integration.

## Consequences

- Baselines are nearly free. ADR 2 records that cloning the rope is O(1) and
  that later edits copy only the path from the root to the touched leaf, so a
  baseline costs a pointer and diverges only by what was edited.
- Nothing has to be invalidated, so nothing can be lost. A recomputed answer
  cannot drift out of agreement with the text.
- The cost moves into the diff, which is bounded by what changed and can be run
  when the buffer settles rather than on every keystroke.
- How the git baseline is obtained is left open here. It is a question about
  reaching git, not about what provenance means.
