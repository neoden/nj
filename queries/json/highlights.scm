; JSON's highlights, after the query of tree-sitter-json 0.24.8 (MIT),
; with the change marked below. The last pattern that names a node wins,
; as in tree-sitter's own highlighter, so for any node the general
; patterns come first and the specific ones after them.

(string) @string

; Changed: upstream has this before `(string)`, which then names every
; key a string.
(pair
  key: (_) @string.special.key)

(number) @number

[
  (null)
  (true)
  (false)
] @constant.builtin

(escape_sequence) @escape

(comment) @comment
