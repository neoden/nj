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
| [12](0012-one-panel-and-a-pinned-status-bar.md) | One panel and a pinned status bar | Accepted |
| [13](0013-scope-a-full-editor-not-an-ide.md) | Scope: a full editor, not an IDE | Accepted |
| [14](0014-no-scripting-engine.md) | No scripting engine; commands are the automation surface | Accepted, refines 8 |
| [15](0015-a-workspace-of-buffers-and-views.md) | A workspace of buffers and views | Accepted, completes 9 |
| [16](0016-a-command-layer-as-the-only-boundary.md) | A command layer as the editor's only boundary | Accepted, completes 8 |
| [17](0017-a-view-is-a-sequence-of-blocks.md) | A view is a sequence of blocks | Accepted, amends 10 |
| [18](0018-anchors-carry-the-version-they-came-from.md) | Anchors: a position and the version it came from | Accepted |
| [19](0019-provenance-is-a-diff-against-a-named-baseline.md) | Provenance is a diff against a named baseline | Accepted |
| [20](0020-an-external-change-is-an-ordinary-transaction.md) | An external change is an ordinary transaction | Accepted |
| [21](0021-one-thread-owns-the-editor.md) | One thread owns the editor; workers get snapshots and send messages | Accepted, resolves R3 |
| [22](0022-a-search-outlives-its-prompt.md) | A search outlives its prompt | Accepted, amends 12 |

[nygard]: https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions
