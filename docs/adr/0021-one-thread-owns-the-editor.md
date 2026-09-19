# 21. One thread owns the editor; workers get snapshots and send messages

Date: 2026-09-19

## Status

Accepted. Resolves R3.

## Context

The main loop blocks in `event::read()`, and nothing runs anywhere else. A
panel reading its candidates gets around this with a busy loop that redraws
the whole frame per batch. Reading the system clipboard blocks the loop for up
to a quarter of a second. Everything the roadmap has next needs work done
while the user keeps typing: walking a project tree for the file picker (F3),
searching it (F18), noticing another writer on the open file (F1, ADR 20),
parsing for highlighting (R6), a language server (F8), a socket other tools
talk to (F21).

Three things make the choice easier than it looks. The text is a rope, so a
snapshot of it is a clone that costs nothing (ADR 2, 3) and stays valid
however long a worker holds it. The buffer keeps a journal, so a position a
worker found in its snapshot can be brought forward to the text as it is when
the result arrives (ADR 18); a result is late, never wrong. And the editor has
one entry point already (ADR 16), so "who may change what" is one rule rather
than many.

The alternatives:

- An async runtime. It buys a way to wait on several sources at once and
  nothing else: the editor has no thousands of connections, and function
  colouring would spread through code that is entirely synchronous today.
- Staying synchronous and polling `event::poll` with a short timeout between
  checks of the background work. Simplest, but it trades latency against CPU
  and does not extend to a socket or a language server, whose replies are not
  events the terminal delivers.
- Waiting on file descriptors directly. crossterm does this inside its event
  source, which is not exposed, and the descriptors that matter later — a
  socket, a watcher — would each need their own integration.

## Decision

One thread, the main one, owns the editor and is the only one that changes
it. Everything else is a worker: a plain `std::thread` that owns nothing but
what it was given — a snapshot of a text, a path, a query — and reports back
through a channel.

The loop waits on a single channel carrying one enum: terminal input,
forwarded by a thread that does nothing but `event::read()`, and reports from
workers. There is no select over several sources because there is only one.
Whatever has arrived by the time the loop wakes is handled before the next
frame, as a burst of keystrokes is today — input first, then reports, so a
keystroke never queues behind results that happened to arrive with it.

A report enters the editor through `receive(Report)`, a second entry point
beside `execute(Command)`. The two are not the same thing and are not merged:
a command is something a person or a script asks for — nameable, listable in
a palette, serializable for a socket — while a report is the editor's own
work coming back to it. A report is never a command, and a command never
carries a report.

A worker that computes positions carries the version of the text it computed
them in; the editor resolves them through the journal on arrival, and drops
what the journal no longer reaches. A job can be cancelled through a flag the
worker checks between units of work; the editor drops reports from jobs it
has cancelled or replaced. Workers are spawned per job; a pool waits until
there is a reason for one.

Nothing in the editor blocks. Reading the clipboard, waiting on a helper
process, walking a directory — anything that may take a while is a job.

## Consequences

- The busy loop in `app::run` goes: a panel's candidates arrive as reports.
  That is the first consumer, and the file picker gets the mechanism it was
  built for.
- Watching the open file needs no `notify` dependency to begin with: a worker
  comparing the stamp on an interval and reporting a change is enough for the
  editor to say so before `Ctrl+S` is pressed (F1, variant b).
- ADR 18 stops being machinery in search of a use: every report with a
  position in it is an anchor.
- `Editor` gains a handle to spawn jobs and a place to keep the ones in
  flight, which is what ADR 16 meant by the context carrying "the background
  jobs".
- The input thread must start after the terminal has been set up: the query
  for the keyboard protocol reads from the same source, and a thread already
  blocked in `event::read()` would swallow its answer.
- A keystroke waits for at most one report: the one being handled when it
  arrives. Its latency is therefore bounded by what `receive` does with a
  single report, and workers batch to keep that within a frame — the
  picker's 4096 candidates are the measure. What a report costs to receive is
  the picker's or the buffer's concern, not the queue's: candidates go into
  the ranking one by one, as `pull` inserts them, never through a refilter of
  everything seen so far (P9).
- The main thread has no guard against itself. Nothing stops `execute` or
  `receive` from doing a second's work between two frames; the rule that only
  cheap things happen there is discipline, and anything that may take long —
  a diff of a large file, laying out a hundred thousand hits — is a job that
  hands back something ready to apply.
- Jobs are threads, and nothing limits how many. A key held down can start a
  tree walk per repeat; cancellation is checked between units of work, so a
  worker inside a slow `read_dir` finishes that first. A pool and a queue of
  jobs are the answer when this bites.
- A worker that panics takes only itself down. Its job is reported as ended
  by a guard that speaks when the worker is dropped, so the editor never waits
  on a job that will not come back.
- Tests keep driving the editor directly: a report can be fed to `receive`
  without a thread in sight, and a job's worker is a function that can be
  called on a snapshot in the test itself.
- The rule "only the main thread mutates the editor" is not enforced by the
  type system — `Editor` is not `Send`, which stops a worker from touching it
  by accident, but not from being handed a clone of something it should not
  have. It is enforced by workers being given values, never references into
  the editor.
