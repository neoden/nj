; The highlights of the text inside a Markdown block, after the inline
; query of tree-sitter-md 0.5.4 (MIT), which comes from nvim-treesitter.
; The last pattern that names a node wins, as in tree-sitter's own
; highlighter, so for any node the general patterns come first and the
; specific ones after them.
;
; Changed throughout: upstream uses nvim-treesitter's old names,
; `text.emphasis`, `text.literal`, `text.uri`; these are the `markup`
; names, as in the block query.

; Changed: upstream names emphasis and strong emphasis, which want a
; face, italic or bold, not a colour; the asterisks around them say it
; well enough. Named, they would be plain text inside a heading too.

(code_span) @markup.raw.inline
(link_title) @string

[
  (emphasis_delimiter)
  (code_span_delimiter)
] @punctuation.delimiter

; Changed: upstream leaves an address out, so that `<me@example.com>` is
; plain beside `<https://example.com>`.
[
  (link_destination)
  (uri_autolink)
  (email_autolink)
] @markup.link.url

[
  (link_label)
  (link_text)
  (image_description)
] @markup.link.label

[
  (backslash_escape)
  (hard_line_break)
] @escape

(image
  [
    "!"
    "["
    "]"
    "("
    ")"
  ] @punctuation.delimiter)

(inline_link
  [
    "["
    "]"
    "("
    ")"
  ] @punctuation.delimiter)

(shortcut_link
  [
    "["
    "]"
  ] @punctuation.delimiter)
