# 14. No scripting engine; commands are the automation surface

Date: 2026-08-10

## Status

Accepted. Refines ADR 8, which made commands nameable for the keymap; this
record makes that the whole extension and automation story.

## Context

Editors buy extensibility with an embedded language. The language is the small
part. The rest is a sandbox, a value model at the boundary, recovery from
errors in foreign code, a lifecycle for hooks, and above all an API that can no
longer be changed freely, because something written against it now exists.

The distribution model here removes the reason to pay any of that. The editor
is built from source by the person using it, so recompilation already is the
extension mechanism. An agent asked to extend the editor writes Rust; the
result is typed, tested and permanent, where a script would be none of the
three.

What is genuinely needed is narrower: a way to drive the editor from outside.
An agent working in the same tree wants to say "open this file at this line",
the way `nvim --remote` and `code -g` are told.

## Decision

No embedded language. Command values are the only automation surface: a
keystroke, the command palette, a configuration file and a socket all produce
the same values, which ADR 16 makes serializable for exactly this reason.

Keymap and theme stay in configuration files. Rebuilding to move a binding is a
poor trade even for the person who wrote the editor.

## Consequences

- No sandbox, no foreign-error recovery, and no obligation to keep an API
  stable. Signatures stay free to change, which every remaining decision in this
  registry depends on.
- Extending the editor costs a rebuild, measured in seconds while the crate
  stays modular.
- The external socket is a few hundred lines, because the command type already
  exists and nothing about a provider or a protocol leaks into the editor.
- Nothing can be shared with someone who does not build from source. Accepted:
  there is no such person.
