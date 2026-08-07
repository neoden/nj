# 9. Transactions and multiple cursors in the core model

Date: 2026-08-06

## Status

Accepted

## Context

Multiple cursors and undo are the two features that cannot be added to an
editor afterwards without rewriting its edit path. If a cursor is a single
offset, every command and every movement assumes it, and widening that
assumption later touches everything. If edits reach the buffer directly, undo
has nothing to record.

## Decision

- A view holds a `SelectionSet`: non-overlapping selections with an anchor, a
  head and a goal column, normalized after every change. A single cursor is the
  case where the set has one element.
- Every change to a document is a transaction: a set of replacements applied
  atomically, plus the selections before and after it. Typing, pasting,
  multi-cursor edits and formatting are all the same operation.
- Selections belong to the view, not to the document, so one file open in two
  views has independent cursors.

## Consequences

- Undo and redo are a stack of transactions and need no separate machinery.
- The same shape is what collaborative editing would need later, should it ever
  be wanted.
- Every command has to be written against a set of selections rather than one
  cursor, including the very first one.
