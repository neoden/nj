# Working rules for this repository

Hard rule, no exceptions: **one change per turn.** Make one focused edit,
show the code, explain in plain words what was broken (with a concrete
example: this text, these cursors, this key), what changed and why, then
stop and wait. "Go ahead" / "приступай" means "start with the first one",
not "do all of them".

The owner wants to understand every line of this editor. Code that arrives
in bulk, or a summary written in the reviewer's own labels, is not
understood and has to be redone. Big-bang refactors are worse than useless
here even when the tests pass.

- Discuss a design change before making it; never present it as done.
- No shell one-liners (sed, awk) for editing files.
- Code and comments in English; comments only where the why is not obvious.
- The owner commits. `docs/*.md` stay uncommitted working notes.
