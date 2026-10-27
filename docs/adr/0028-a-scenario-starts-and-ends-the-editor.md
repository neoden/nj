# 28. A scenario starts and ends the editor

Date: 2026-10-26

## Status

Accepted. Extends ADR 25 and ADR 27.

## Context

A scenario of ADR 25 begins with an editor that already exists, holding an
empty scratch buffer, and goes on for as long as the file has lines. What
happens before that editor and after it is in `main.rs` and out of reach:
the command line is parsed there, the buffer is chosen there — the file
named, else standard input when it is a pipe, else nothing — and when the
editor is done, the exit code and what a pipeline downstream receives are
decided there.

Two scenarios in the plan could not be written for that reason. A pipeline
buffer, `cat x | ned`, has rules of its own — its name, what `Ctrl+S` does,
that `Ctrl+O` refuses to replace it — and the headless run had no way to
start with one. And quitting is invisible: the headless run ignores
`exit()`, so a scenario that answers `q` to the quit question shows the
same frame as one that answers `Esc`, and goes on sending keys to an editor
that has left.

The language has a convention for setting the world up before the editor
acts on it: `size`, `file` and `clipboard` are lines that prepare the run.
Starting the editor fits the same shape.

## Decision

- The command line is parsed, and the starting buffer chosen, in the
  library rather than in `main.rs`, so that a scenario runs the same code
  a launch runs and not a copy of it.
- `start ARGS` starts the editor with `ARGS` as its command line, read by
  that parser: `start notes.txt` is `ned notes.txt`, `start` alone is
  `ned`. The options the sandbox cannot honour — `--socket`, `--sandbox` —
  are refused on the line. A scenario without a `start` line begins as
  before, with a scratch buffer.
- `stdin "TEXT"` gives the editor `TEXT` on standard input, as a pipe
  would; it belongs before `start`, with `file`. `size` is the editor's
  line, like a key: it starts the editor where nothing has, and resizes
  it where it runs. Without `stdin`,
  standard input is a terminal. A file on the `start` line still wins over
  the pipe, as it does on the command line.
- Once the editor has exited, a snapshot shows the outcome instead of a
  frame: `exit 0` or `exit 1`, and, for a pipeline, a `stdout "TEXT"` line
  with what the downstream received. The editor's own lines — input,
  `wait`, `size` — are an error naming the line from then on, there being
  no editor to take them, so a scenario cannot go on past a quit by
  mistake. The lines that set the run up go on, and `start` starts the
  next editor, in the run's directory as the last one left it.
- Over the socket, the line that makes the editor exit is answered with
  the same outcome, and the connection closes with the editor.

## Consequences

- `files/a-piped-stdin-is-a-scratch-buffer` and the `q` of the quit
  question become writable, and the quit question's scenario stops lying
  by omission.
- The argument parser gets its tests through scenarios, which is where
  its mistakes would show.
- What the program is called does not appear in a scenario: `start` is a
  verb of the language, and the binary can be renamed without touching
  the files or the wire format.
- The process layer stays out: the terminal restored after a panic, a
  signal, standard output claimed for the pipe. Those belong to the pty
  smoke run of ADR 25, and a scenario that expected a panic would be a
  panic kept as behaviour.
- A watch that fires on its own clock (ADR 20) is still out of reach of a
  scenario; that wants a line of its own and is not decided here.
