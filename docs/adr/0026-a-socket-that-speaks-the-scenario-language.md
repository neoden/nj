# 26. A socket that speaks the scenario language

Date: 2026-10-25

## Status

Accepted. Realizes the socket that ADR 14 named as the automation surface
and ADR 21 planned for (F21), and takes its wire format from ADR 25.

## Context

ADR 14 decided that the editor is driven from outside through command
values, over a socket, and ADR 16 made commands serializable for that. The
socket itself was never built. ADR 25 then gave the editor a small language
for scenarios — keys by name, typed text, pastes, mouse actions — with a
parser in the library, and a way to run such a scenario against a headless
editor.

Two wants now meet. One is the socket of ADR 14: a tool in the same tree
says "open this file at this line". The other is to watch a scenario run in
the real editor, in a real terminal, step by step, with a person deciding
when the next step goes: for debugging what the headless run cannot show,
and for seeing a behaviour rather than reading a frame of it. The second
wants keystrokes delivered from outside, not commands.

Keystrokes over a socket are not an automation surface in the sense ADR 14
refuses: they depend on the keymap and on what has the keyboard, and a tool
that drove the editor with them would break with every rebinding. They are
what a terminal sends, and a socket carrying them is a second terminal.

The loop of ADR 21 waits on one channel, and a thread that reads the
terminal is already one of its sources. A thread that reads a socket is
another of the same kind.

## Decision

- With `--socket PATH` the editor listens on a Unix socket at `PATH`. A
  connection sends lines, one instruction per line, in the scenario
  language of ADR 25. Every line is answered, in order, and every answer
  has the same shape: one or more lines ended by a blank one — `ok`, or the
  error, or a frame. A client reads to the blank line without having to
  remember what it asked.
- The input instructions — `key`, `type`, `paste`, `click`, `drag`,
  `scroll` — are parsed by the one parser in `term::scenario`, become the
  events a terminal would have sent, and enter the loop as input, through
  the same translation as the keyboard. Nothing in the editor can tell the
  two apart.
- Commands come later as a `cmd` line carrying a serialized command (ADR
  16), parsed by the same parser and entering through `execute`. That is
  the automation surface of ADR 14; the input instructions are not, and the
  record says so.
- The instructions that belong to testing — `size`, `---`, `wait`, `file`,
  `clipboard` — are refused by the socket unless the editor runs in the
  sandbox of ADR 27.
- The listener is a worker in the sense of ADR 21: it owns the socket and
  nothing of the editor, and what it has to say goes through the channel.

## Consequences

- Scenarios can be replayed in a live editor, instruction by instruction,
  and the socket needs no second parser and no second input path.
- A line of the scenario language is now a wire format, so its spelling
  is harder to change than it was: the drive example, the test runner and
  the socket all read it.
- The socket is a file with the user's permissions, and whoever can write
  to it types into the editor. That is the same trust as the terminal's.
- The language stays at the level of what a person did: a key by its
  name, text as typed. Raw terminal bytes have nowhere to enter, since the
  decoder is crossterm's, and the pty smoke run is where that layer is
  exercised; raw key events would put crossterm's types on the wire, which
  ADR 5 keeps inside `term`, and would only show that `translate` drops
  what it drops, which its unit tests show already. A `raw` instruction
  can be added the day a need appears that none of those meets.
- The command form is the one tools should use, and it is not there yet;
  until it is, the socket is for replay and remote keyboards, not for
  automation.
