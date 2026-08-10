# 13. Scope: a full editor, not an IDE

Date: 2026-08-10

## Status

Accepted

## Context

The editor has to be finished by one person, and every feature list written for
"an editor like VS Code" is really a list for an IDE. Most of it is never used.
Which parts are in and which are out is not a backlog question: an embedded
terminal wants a second grid and a second input path, a debugger wants a second
interface of its own, and both would shape the architecture rather than sit on
top of it.

The distribution model narrows this further. The editor is built from source by
the person using it, so it does not have to serve anyone whose habits are
unknown.

## Decision

Implement the part of VS Code that is actually used daily, and say plainly what
is left out.

In: multiple cursors, soft wrap, a language server client, syntax highlighting,
project-wide search that can be edited in place, git diffs, blame and the
current branch, file and command pickers, configuration and themes.

Out, deliberately: an embedded terminal emulator, a debugger, an interface for
staging and committing, and collaboration between people.

## Consequences

- The finished editor is on the order of thirty thousand lines, and none of
  them are framework: no plugin engine, no UI toolkit, no replicated data type.
- Interactive shell work and git beyond reading happen in the terminal the
  editor was started from. The editor still has to run a process and show what
  it printed, which is a fraction of the cost of emulating a terminal.
- Each exclusion is a feature not written rather than a door nailed shut, with
  one exception: the terminal would arrive with its own grid, its own parser and
  its own input path, and adding it later is not the same as leaving room for it.
- The subset is one person's. This is not a general-purpose editor, and the
  keymap already said as much (ADR 1).
