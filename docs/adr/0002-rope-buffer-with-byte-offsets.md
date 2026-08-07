# 2. A rope buffer addressed in byte offsets

Date: 2026-08-06

## Status

Accepted

## Context

The buffer has to support edits anywhere in O(log n), map offsets to line
numbers and back, and hand a stable snapshot to background work (syntax
parsing, search, LSP) without blocking editing.

Two families were considered. A piece table (or the piece tree VS Code uses)
opens large files without copying and makes undo nearly free, but it has to
carry its own line index in the tree nodes; an earlier attempt at writing one
stalled on exactly that. A rope solves the same problem with the line index
already built in.

Within ropey, 1.6 indexes by character while 2.0 indexes by byte. Both indices
are `usize`, so mixing them up is a mistake the compiler cannot catch.

## Decision

Use `ropey` 2.0.0-beta.1 and address the buffer in byte offsets everywhere.
Enable only the `simd` and `metric_lines_lf` features.

## Consequences

- No byte-to-char conversion on every edit, and no way to pass the wrong kind
  of index, because only one kind exists.
- `Rope::clone` is O(1): nodes are `Arc<Children>`/`Arc<Text>` mutated through
  `Arc::make_mut`, so a snapshot shares storage and later edits copy only the
  path from the root to the touched leaf.
- Each enabled line metric adds counters to every tree node, so only LF is
  tracked. This is honest because the buffer never contains CR (ADR 4).
- `byte_to_utf16_idx` is available if an LSP server refuses UTF-8 positions.
- Depending on a beta is a real risk. It is contained by ADR 3: ropey is
  visible in exactly one file.
