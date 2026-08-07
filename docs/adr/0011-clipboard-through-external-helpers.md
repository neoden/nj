# 11. Reach the clipboard through external helpers

Date: 2026-08-07

## Status

Accepted

## Context

On Wayland and X11 the clipboard is not storage. The application that copies
owns the selection and must stay alive to serve it on request. Something has to
keep running after a copy.

`arboard` is the usual crate for this. On Wayland it uses `wl-clipboard-rs`,
which forks a helper exactly as `wl-copy` does; on X11 it serves the selection
from a thread for as long as its `Clipboard` value lives. It costs 38
additional packages, and its X11 backend loses the copied text when the editor
exits, where a forked `xclip` keeps it.

Over ssh there is no local display at all, and neither approach can reach the
clipboard of the machine in front of the user. OSC 52 can: the terminal is
asked to do the copying.

## Decision

Copy and paste through whichever helper is installed — `wl-copy`/`wl-paste`,
`xclip` or `xsel` — chosen by which display server is running. When no helper
answers, fall back to OSC 52 for copying.

Reading the clipboard over OSC 52 is not attempted: terminals disable it by
default, and the reply would have to be parsed out of the input stream by hand.

## Consequences

- No wayland or X11 client library in the dependency tree; the only cost is
  `base64`, pulled by crossterm's `osc52` feature.
- A copy survives the editor exiting, because the helper forks.
- Copying works over ssh, reaching the user's own clipboard.
- Pasting foreign text with `Ctrl+V` needs a helper installed. Without one the
  terminal's own paste still works, arriving as a bracketed-paste event.
- Each copy or paste spawns a process, costing about a millisecond.
- The system clipboard is off unless `with_system_clipboard` is called, so
  tests never touch the real one.
