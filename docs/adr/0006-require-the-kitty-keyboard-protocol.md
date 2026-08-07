# 6. Require the kitty keyboard protocol, degrade with a warning

Date: 2026-08-06

## Status

Accepted

## Context

A traditional terminal cannot distinguish `Ctrl+Shift+K` from `Ctrl+K`, and
never delivers `Ctrl+/`, `Ctrl+Enter` or `Shift+Enter` at all. A VS Code keymap
is unreachable without the kitty keyboard protocol, which kitty, foot, ghostty
and wezterm all support.

Supporting legacy terminals fully would mean a second, chord-based binding for
every command, and a permanent tax on every command added afterwards.

## Decision

Request `DISAMBIGUATE_ESCAPE_CODES` at startup. Where it is unavailable, run
anyway and say so in the status bar rather than emulating the missing keys.
Chords are implemented from the start because they are part of the VS Code
keymap regardless, and they double as the escape hatch for unreachable keys.

## Consequences

- The default keymap can use the full modifier space.
- On a legacy terminal some commands are reachable only through chords or the
  command palette. This is stated, not hidden.
- Other enhancement flags are deliberately not requested. In particular,
  `REPORT_ALTERNATE_KEYS` makes crossterm replace the key code with the shifted
  character and clear the SHIFT flag, which would change how every binding
  matches.
- Terminals intercept some combinations before any protocol applies. foot 1.27
  takes `Ctrl+0`, `Ctrl+±`, `Ctrl+Shift+C/V/N/O/R/U/Z`, `Shift+Insert` and
  `Shift+PageUp/PageDown`. The default keymap therefore avoids those where it
  can, and always offers an alternative for `Ctrl+Shift+Z` (redo).
- Restoring the terminal must pop the flags only if they were pushed, or the
  next application inherits a mode it did not set.
