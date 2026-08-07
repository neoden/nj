# 4. UTF-8 only, with LF-normalized buffers

Date: 2026-08-06

## Status

Accepted

## Context

Handling CRLF throughout the editor means every movement, selection and edit
has to treat a two-byte sequence as one position. Handling arbitrary encodings
means either a conversion layer or lossy decoding.

A file must survive a load-and-save round trip byte for byte, whether or not it
was edited. Anything less silently damages the user's file.

## Decision

The buffer holds UTF-8 with LF breaks and never contains CR. On load, CRLF is
normalized to LF and a byte-order mark is stripped; both are recorded in the
document and restored on save. Files that are not valid UTF-8 are refused with
an error rather than decoded lossily.

## Consequences

- Every offset in the editor addresses a real position; there are no two-byte
  line breaks to step over.
- `LineType::LF` in ADR 2 is exact rather than an approximation.
- Mixed-ending files are saved with whichever ending appeared first. This is a
  deliberate simplification; the alternative is preserving each line's ending
  individually, which no editor the keymap imitates does either.
- Non-UTF-8 files cannot be opened at all until an encoding layer exists. A
  clear refusal is better than a corrupted save.
- Encoding a document currently materializes it as a `String`. Streaming
  through `Rope::write_to` is left for when large-file support is addressed.
