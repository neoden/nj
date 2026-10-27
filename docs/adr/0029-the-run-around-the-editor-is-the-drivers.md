# 29. The run around the editor is the sandbox's, a process of its own

Date: 2026-10-27

## Status

Accepted. Supersedes ADR 27; keeps the socket of ADR 26 and the language
of ADR 25 and ADR 28.

## Context

ADR 27 put the conditions a scenario assumes — the fake clipboard, the
directory of the run, the frame of the scenario's size — into the editor
itself, as a `--sandbox` mode of the debug build, and let the socket take
the testing instructions in that mode. ADR 28 then added `start` and
`stdin` to the language, and those a running editor cannot take: its
standard input was read once, its command line was given once, its working
directory is the process's. A live editor that took `start` over the socket
would have to make a new editor inside the old process — a job queue that
outlives the editor that spawned its jobs, job ids unique for the process
rather than the queue, an exit that is reported but never happens. That
is work the operating system does for free when the editor is a process:
it starts with its arguments and its pipe, and it ends with an exit code
and what it wrote downstream.

The library already lets a binary be built from the same code with another
`main`. The headless run keeps the conditions of a scenario outside the
editor and starts the editor inside them; a process can do the same with
a child.

The language did not show which of its lines are the editor's. `file` and
`type` looked alike, and the socket told them apart by a list of verbs
that every new verb had to be added to.

## Decision

- A line that sets the run up is written with a leading `%`: `%file`,
  `%clipboard`, `%stdin`, `%start`. The editor never sees one: its socket
  refuses any line that starts with `%` without reading further. `size`
  is not among them, since the editor takes it in both runs — as its
  frame in the headless one, as the frame to draw in the live one.
- The editor's socket takes the rest: input, `wait`, `size`, and, in the
  debug build, `---`, answered with the frame dumped as the test dumps
  it. That part of ADR 27 stands; `--sandbox` and the sandbox inside the
  editor do not.
- The sandbox is a binary of this crate, built from the library, that
  runs in a terminal of its own. It makes a directory for the run under
  the system's temporary directory, listens on a socket of its own and
  takes the whole language. The `%` lines it carries out itself: `%file`
  in the run's directory; `%clipboard` into one file, which the editor
  reaches through helper scripts put first on its `PATH` under the names
  of the desktop's clipboard tools, so the desktop's clipboard is never
  touched; `%stdin` kept for the start; `%start ARGS` the unchanged
  editor started as a child process in that terminal with those
  arguments, the pipe on its standard input and its standard output kept.
  Every other line it hands to the editor's socket and answers as the
  editor did. When the editor exits, the sandbox waits for the process
  and answers with its exit code and, for a pipeline, what it wrote.
- A connection to the sandbox is one scenario. Where the last one left
  the editor exited, the next connection begins before an editor, as a
  scenario does; so scenarios run one after another in one sandbox.
- The replay is a third binary, a client of the sandbox's socket: it
  shows each line of the file and sends it on Enter, and at `---`
  compares the frame answered with the frame in the file, printing both
  only where they differ.
- Both binaries are test machinery and exist in the debug build only,
  like the headless run. On Ctrl+C the sandbox leaves as it would on any
  other end: the directory, the socket and an editor still running go
  with it.

## Consequences

- The editor built for use carries the parser, the socket and `size`;
  the debug build carries `dump` for `---`; the editor carries no
  sandbox, no clipboard knob and no queue that outlives it.
- The process's own start and end — the arguments, the pipe read, the
  exit code, what the downstream receives — are exercised by every live
  run, which the headless run cannot do.
- `tools/replay.py` goes.
- The scenario files that set a run up are edited once, `file` to `%file`
  and so on, and the socket's refusal is one test of a prefix instead of a
  list of verbs.
- The fake clipboard depends on which tool the editor picks and on the
  helpers ignoring its arguments; a change in `system_clipboard.rs` is a
  change to the helpers.
- `size` is refused when the sandbox's terminal is too small for the
  frame, and the replay prints the refusal instead of a frame shown in
  part.
