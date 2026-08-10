# 17. A view is a sequence of blocks

Date: 2026-08-10

## Status

Accepted. Amends ADR 10, which made the row the unit of display; the row now
names its block as well as its line.

## Context

Project-wide search is only worth having if what it found can be edited where
it was found. Reviewing a branch is the same picture from another angle:
excerpts from many files one after another, each of them a working editor
rather than a picture of a diff, so that one can look around, search, and add a
correction on the spot. A view tied to a single buffer shows neither.

A diff makes the difficulty precise. It is two texts, and only one of them is a
file that can be edited. Assembling a third, synthetic text of `-` and `+` lines
produces something that cannot be edited at all, because a change made in it has
nowhere to go.

## Decision

A view is a sequence of blocks.

An excerpt is an anchored range of a buffer and behaves like any other text:
what is on screen is the real file, and saving writes it. A static block is rows
belonging to no buffer — a file heading, a hunk heading, the lines a diff
removed. Deleted lines are drawn rather than stored, which is what keeps the
excerpt around them editable.

A row therefore names its block as well as its line. This is the seam ADR 10
opened for soft wrap, on a second axis, and it is introduced now for the same
reason: an ordinary view is the degenerate case with one excerpt covering one
buffer, so it costs almost nothing today.

## Consequences

- Editable project search, review of a branch, a rename preview from a language
  server and a replacement across many files become one surface rather than
  four separate interfaces, which is how they arrived in the editors that grew
  them one at a time.
- Static blocks are also where inline diagnostics, inline blame and collapsed
  regions belong, so there is one decoration primitive instead of several.
- The list of blocks is derived, not authored: editing inside a hunk moves the
  boundaries around it. It is recomputed when the buffer settles, and the
  anchors of ADR 18 are what keep the screen still while that happens.
- An excerpt must be able to name a buffer that is not open yet, or reviewing
  two hundred files would open two hundred buffers.
- Search takes a domain — a buffer, the excerpts of a view, the project — rather
  than a text.
- Once soft wrap exists a row has three parts: its block, its line, and which
  row of that line it is.
