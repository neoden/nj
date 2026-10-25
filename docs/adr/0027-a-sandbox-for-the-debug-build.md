# 27. A sandbox, in the debug build only

Date: 2026-10-25

## Status

Accepted. Extends ADR 25 to the live editor and completes ADR 26.

## Context

A scenario of ADR 25 means something only under conditions the headless
run provides and a live editor does not: the clipboard is a fake, files go
to a directory made for the run, and the frame has the size the scenario
says. Replaying a scenario over the socket of ADR 26 in an ordinary editor
would overwrite the person's clipboard, scatter files where the editor was
started, and show a frame of another size than the one in the file.

These conditions are a mode of the editor, separate from the socket: an
editor can run in them without a socket, and a socket can drive an editor
that is not in them. They are also test machinery, and the editor built for
use has no business carrying it.

## Decision

- `--sandbox` starts the editor with the fake clipboard and the run
  directory of `term::sandbox`, so that a scenario means in it what it
  means in the test.
- In the sandbox the socket accepts the testing instructions. `size` sets
  the frame: the loop draws it in the top-left corner and the rest of the
  terminal stays blank, so that the screen and the frame in the file agree
  cell for cell. A terminal too small for the frame is refused — by `size`
  when it is set, by `---` when the terminal has shrunk since — rather than
  shown in part, since a part would not agree with the file. `---` answers
  with the frame, dumped as the test would dump it, ended by a blank line.
  `wait`, `file` and `clipboard` do what they do in a run.
- The sandbox exists in the debug build only, under
  `cfg(debug_assertions)`, and with it the headless run of a scenario and
  the dump of a frame, which are the sandbox's own. The language itself —
  reading a line into an instruction — stays in every build, since the
  socket of ADR 26 speaks it. In a release build `--sandbox` is an unknown
  argument and the code is absent.

## Consequences

- A replay tool is a script that reads a scenario, sends it line by line
  with a pause between, and shows the frame the editor answered beside the
  frame in the file. It needs the sandbox and the socket together, and
  neither alone.
- Testing instructions reaching an editor outside the sandbox are refused,
  so no replay can touch what belongs to the person.
- `debug_assertions` is a property of the profile, not of the binary, and
  can be turned on in a release profile by hand. For keeping test
  machinery out of the build that is shipped, that is enough.
- The release binary contains the parser and not the runner or the dump.
  The tests and the drive example build in the debug profile and have
  them; `cargo test --release` does not build.
