# 30. The sandbox and the replay

Date: 2026-10-28

## Status

Accepted. Supersedes ADR 27, ADR 28 and ADR 29, which arrived at this
arrangement in three steps; this record states it whole. Keeps the oracle
and the language of ADR 25 and the socket of ADR 26.

## Context

A scenario of ADR 25 is a sequence of events and the frames they produce.
It means something only under conditions it does not state: an empty
directory of its own, so a path typed into a prompt lands nowhere else; a
clipboard that holds only what the scenario put there; a frame of the size
the scenario says; and an editor started the way a launch starts it, with
a command line and, for a pipeline, a pipe on standard input.

The headless run provides those conditions in-process. A live replay —
the same scenario, in a real terminal, with a person deciding when each
line goes — needs them around a real editor. Three records got there one
step at a time. ADR 27 put the conditions inside the editor as a debug
mode. ADR 28 added starting and ending the editor to the language, which
a running editor cannot take about itself. ADR 29 moved the conditions
out again, into a process of its own that starts the editor as a child,
and marked the lines that are the run's rather than the editor's.

Two things ADR 29 decided wrongly. It made a connection to the sandbox
mean a scenario, with the next connection starting afresh if the last
editor had exited — but the sandbox only knew of an exit it had seen on
its socket, and an editor quit by hand left it believing the editor was
still there, so the next scenario was refused. And it ended an editor,
when it had to, by killing it, which left the shared terminal as the
editor had set it: raw mode, the alternate screen, a frame that nothing
would redraw. The terminal is the editor's to change and to restore, and
no process should restore what it did not change.

## Decision

### The language

The language of ADR 25 has two kinds of line, told apart by a prefix.

- A line that sets the run up around the editor begins with `%`: `%file`,
  `%clipboard`, `%stdin`, `%start`, `%reset`. The editor never sees one.
  Its socket refuses any line that begins with `%` without reading
  further.
- Every other line is the editor's: input, `wait`, `size`, and `---`.
  `size` is the editor's even though it is a condition of the run, since
  the editor takes it in both runs — as its frame in the headless one, as
  the frame to draw in the live one.

`%start ARGS` starts the editor with `ARGS` as its command line, read by
the same parser a launch uses; `%stdin "TEXT"` before it is the pipe.
Without `%start`, the first editor's line starts the editor as `ned` alone
would. After the editor has exited, a snapshot shows the outcome — `exit
N`, and for a pipeline a `stdout "TEXT"` line — and the editor's own lines
are an error naming the line, so a scenario cannot go on past a quit by
mistake. `%start` starts the next editor, in the directory as the last
one left it.

`%reset` is the run as it was before anything: an editor still running is
ended, the directory emptied, a `%stdin` given and not yet used forgotten.
It is the same line whatever the stage — before an editor, with one, after
one — and the only way back to the start.

### The headless run

`term::sandbox::run` carries a scenario out in-process, against an editor
with a fake clipboard, in a directory made for the run under `target` and
removed after it, with a default frame of 40×10 until `size` says
otherwise. Runs in one process take turns, so no two share the directory.

### The sandbox

The sandbox is a binary of this crate, built from the library, that runs
in a terminal of its own. It makes a directory for the run under the
system's temporary directory, enters it, and listens on a socket of its
own. It takes the whole language:

- The `%` lines it carries out itself. `%file` writes into the directory.
  `%clipboard` writes one file, which the editor reaches through helper
  scripts put first on its `PATH` under the names of the desktop's
  clipboard tools, the editor told it is under Wayland whatever the
  sandbox runs under; the desktop's clipboard is never touched. `%stdin`
  is kept for the start. `%start` starts the unchanged editor binary as a
  child in the same terminal, with its arguments, `--socket` pointing at a
  socket in the directory, the pipe on standard input if `%stdin` gave
  one, and standard output kept in a file for the outcome.
- Every other line it hands to the editor's socket and answers as the
  editor did. When the line makes the editor exit, the sandbox waits for
  the process and answers with its exit code and, for a pipeline, what it
  wrote. A snapshot after that is the outcome.
- `%reset` ends an editor still running by sending it SIGTERM and waiting
  for it — the same request the system makes — and then renews the
  directory and the helpers.

A connection changes nothing by itself. One client is served at a time,
to the end of its connection, and the next finds the run where the last
one left it: an editor still open, a directory with files in it. The
sandbox does not watch the editor to see whether it has gone; a line sent
to an editor that was quit by hand fails as a line to a closed socket
fails, and `%reset` is the answer to that as to anything else.

The sandbox leaves on SIGINT or SIGTERM — Ctrl+C while no editor runs, or
a `kill` — and on its way out ends an editor still running the same way
`%reset` does. While an editor runs it holds the terminal in raw mode, and
Ctrl+C reaches it as a key, not the sandbox as a signal.

### The editor's part

The editor's socket, in the debug build, takes the testing lines: `size`
sets the frame, which the loop draws in the top-left corner with the rest
of the terminal blank, so the screen and the frame in the file agree cell
for cell; `---` answers with the frame dumped as the test dumps it; `wait`
finishes the jobs. A terminal too small for the frame is refused — by
`size` when it is set, by `---` when the terminal has shrunk since —
rather than shown in part.

A signal that asks the process to end — SIGTERM, SIGHUP, SIGINT — ends the
editor in order: as `q` to the quit question would, with no question
asked, since there is nobody to ask and the terminal may already be gone.
Nothing is written; what was last saved is what stands, and a pipeline
receives it. The exit code is 0, the one of a quit. The signal is one more
sender on the loop's queue (ADR 21), taken before any input that came
with it, and it is caught before the terminal is touched, so that none can
leave the terminal in raw mode; a signal arriving before the loop reads
them is kept, not acted on by default. Ctrl+C in raw mode is a key and
reaches the editor as one; `kill -INT` is still the signal.

The terminal is changed by the editor and restored by the editor, on
every exit including this one. Nothing else writes to it. That is what
lets `%reset` and the sandbox's leaving ask the editor to go and do
nothing more.

### The replay

The replay is a third binary, a client of the sandbox's socket. It reads a
scenario file, connects, sends `%reset` first — a replay is the scenario
from its first line, whatever the last replay left — and then shows each
line of the file and sends it on Enter; `a` sends the rest without
stopping, `q` stops. At `---` it compares the frame the sandbox answered
with the frame in the file and prints both only where they differ. The
scripts in `tools/` start the sandbox in one terminal and the replay in
another; neither starts a terminal, the person opens the windows.

### What is built where

The parser of the language is in every build, since the socket of ADR 26
speaks it. The headless run, the frame dump, the sandbox and the replay
are test machinery and exist in the debug build only, under
`cfg(debug_assertions)`; in a release build the two binaries say so and
exit. `debug_assertions` is a property of the profile, not of the binary,
which is enough to keep test machinery out of what is shipped.

## Consequences

- Scenarios run one after another in one sandbox, whatever the last one
  left: an editor still open is ended by the next replay's `%reset`, and
  the person need not quit it by hand, though they may.
- After `%reset` the sandbox's terminal shows the terminal, not the last
  frame of an editor that is gone, because the editor restored it on its
  way out as it does on any exit.
- A `kill -9` of the editor still leaves the terminal as it was: that is
  the operating system's doing and no process can answer for it. A signal
  that ends the editor before it has exited in order — which SIGKILL is
  the only one of — leaves the sandbox's run without an editor and is
  reported as an error.
- The signal behaviour is the editor's, seen by every user: a `kill` or a
  closing terminal ends the editor without saving. Editors with a recovery
  file — vim's swap, nano's `.save` — write it on the way out; this editor
  has none to write, and does what helix and kakoune do.
- The language has no line for a signal. The headless run does not need
  one, and the sandbox sends SIGTERM only on its own account.
- A scenario file may contain `%reset` and continue after it, with the
  same meaning in the headless run as over the socket; the test for it
  does.
- The process layer below the editor — the escape stream, the keyboard
  protocol negotiation, a panic on the alternate screen — stays with the
  pty smoke run of ADR 25. A scenario that expected a panic would be a
  panic kept as behaviour.
- A watch that fires on its own clock (ADR 20) is still out of reach of a
  scenario, and that wants a line of its own, not decided here.
