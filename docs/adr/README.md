# Architecture decision records

Decisions that were not obvious, with the reasoning that produced them. Each
record is immutable: when a decision changes, a new record supersedes the old
one rather than editing it.

Format: context, decision, consequences ([Nygard][nygard]).

| # | Decision | Status |
|---|---|---|
| [1](0001-non-modal-terminal-editor-in-rust.md) | A non-modal terminal editor in Rust | Accepted |
| [2](0002-rope-buffer-with-byte-offsets.md) | A rope buffer addressed in byte offsets | Accepted |
| [3](0003-encapsulate-the-rope-instead-of-a-trait.md) | Encapsulate the rope instead of defining a buffer trait | Accepted |
| [4](0004-utf8-only-lf-normalized-buffers.md) | UTF-8 only, with LF-normalized buffers | Accepted |
| [5](0005-crossterm-with-a-hand-written-grid-renderer.md) | crossterm plus a hand-written grid renderer | Accepted |
| [6](0006-require-the-kitty-keyboard-protocol.md) | Require the kitty keyboard protocol, degrade with a warning | Accepted |
| [7](0007-normalize-keystrokes-to-physical-keys.md) | Normalize keystrokes to physical keys | Accepted |
| [8](0008-keymap-structure.md) | Keymap structure: defaults in code, overrides in TOML | Accepted |
| [9](0009-transactions-and-multiple-cursors.md) | Transactions and multiple cursors in the core model | Accepted |
| [10](0010-rows-as-the-unit-of-display.md) | Rows, not lines, as the unit of display | Accepted |
| [11](0011-clipboard-through-external-helpers.md) | Reach the clipboard through external helpers | Accepted |

[nygard]: https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions
