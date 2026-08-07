# 3. Encapsulate the rope instead of defining a buffer trait

Date: 2026-08-06

## Status

Accepted

## Context

The buffer implementation may change: ropey 2.0 is a beta (ADR 2), and writing
a piece tree remains an open interest. The obvious way to keep that door open
is a `Buffer` trait with the rope as one implementation.

But a trait with a single implementation buys nothing at compile time and costs
generics or dynamic dispatch at every use site. What actually makes an
implementation replaceable is that no caller can see it.

## Decision

Expose a concrete `Text` type in `src/text.rs`. `Rope` appears nowhere else in
the crate. No trait until there is a second implementation to justify one.

## Consequences

- Swapping the buffer means rewriting one file, which is the same freedom a
  trait would have given.
- The rest of the editor reads as plain method calls with no type parameters.
- One deliberate leak: `Text::line` and `Text::slice` return `RopeSlice`.
  Defining an equivalent slice type would cost hundreds of lines and buy
  nothing, so this is accepted as the seam's rough edge.
