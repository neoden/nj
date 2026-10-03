# 24. A form for each case

Date: 2026-10-03

## Status

Accepted. Refines ADR 12, which made panel kinds variants of one enum, and
amends ADR 23, which had the replacement ask its question in a `Confirm`.

## Context

The panel has three kinds, and each serves several cases. A `Prompt` is the
search field, the replace fields, the line number, the path to open and the
path to save under; a `Purpose` says which, and one `match` on it decides
what `Enter` does. A `Confirm` is five questions, a `Picker` two lists.

Sharing a kind is cheap as long as the cases differ only in a label. They
stopped doing so. Search and replace wanted behaviour the path prompts have
no use for, and it went into the shared kind behind `Purpose::searches()`.
The replacement that asks at each hit was put into `Confirm`, and at once
needed three things a `Confirm` is not: a question with the replacement in
it, a panel that stays open after an answer, and a single line where a
`Confirm` takes one row per answer. Each was a problem only because the form
was borrowed.

## Decision

Each case has a form of its own.

- The prompts become `Find`, `Replace`, `GotoLine`, `Open` and `SaveAs`, each
  a panel kind with its own fields and its own meaning of `Enter`. `Purpose`
  and the `match` on it go.
- Walking the hits of a replacement is a form too, `Replacing`: one line
  with the question and its keys, holding the replacement and what it needs
  to know across answers. `Replace` gives way to it on `Enter`. It is not a
  `Confirm`.
- What forms share is a part, not a kind: `Field`, a labelled line of input,
  and the one function that draws such lines.
- The forms are moved out of `Prompt` one at a time, and `Prompt` is deleted
  with the last of them.
- `Confirm` and `Picker` stay as they are for now. Whether their cases want
  forms of their own is looked at after `Prompt`.

## Consequences

- There are more panel kinds, and each place that lists them grows an arm
  per kind: height, drawing, the field with the keyboard, the keymap's
  contexts, the click that dismisses a field. ADR 12 named the first three
  as the whole price of a kind, and that still holds per kind.
- A behaviour of one case has one place to live. Nothing asks a shared kind
  which case it is today.
- What two forms do alike is written twice unless it is a part they share.
  That is the pressure to watch: a second shared part should be as plain as
  `Field`, or it is the borrowed form again.
- `Enter` in a form is that form's own command, so the keymap can bind it
  per context without the editor choosing by purpose afterwards.
- A form that leads to another, as `Replace` leads to `Replacing`, hands
  over what the next one needs when it closes. Nothing else outlives it.
