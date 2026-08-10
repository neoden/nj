# 16. A command layer as the editor's only boundary

Date: 2026-08-10

## Status

Accepted. Completes ADR 8, which described the shape of a command but stopped
at the keyboard.

## Context

The real question ADR 8 left open is not how keys are bound but where the
editor's boundary is, and at the moment there is none. The event loop writes to
the view's selections directly. Key handling is several hundred lines of
hand-written matching spread across three functions, which is the situation ADR
8 opens by warning against. Tests reach into the fields they mean to exercise,
so they cement a structure that ADR 15 and ADR 17 are about to change.

Naming the boundary is also what ADR 14 rests on: without command values there
is nothing for anything outside the editor to send.

## Decision

One entry point, `execute(&mut Cx, Command)`, where `Cx` carries the workspace,
the state of the interface and the background jobs. Fields become private the
same day, or the boundary is decoration.

Commands stay an enum with fields, as ADR 8 decided, nested by subsystem so the
outer match routes and the modules do the work. Command values are
serializable, which makes the palette, the configuration file and the external
socket one mechanism rather than three.

The keymap becomes the ordered table of bindings with context predicates that
ADR 8 specified.

## Consequences

- A test is a sequence of commands, and therefore stops depending on where the
  state it exercises happens to live.
- Every command is nameable, which the palette needs anyway.
- A panel handles its own keys and reports what it wants done, so ADR 12 holds
  again: a new panel kind touches its height, its drawing and its key handling
  and nothing else.
- The enum is large. Exhaustiveness checking is the reason to keep it rather
  than a table of functions; flat names would multiply exactly as ADR 8 said.
