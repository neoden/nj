; TOML's highlights, after the query of tree-sitter-toml-ng 0.7.0 (MIT),
; with the changes marked below. The last pattern that names a node wins,
; as in tree-sitter's own highlighter, so for any node the general
; patterns come first and the specific ones after them.

; Keys
;-----

; Changed: upstream names a quoted key a string and a bare one a type, so
; that the same key looks two ways by how it is written. Here what a key
; is decides, not its quotes: a table's name is a type, `dependencies` in
; `[dependencies]` and `bin` in `[[bin]]`, and a key in a pair a property.
; A dotted key is named whole, as `a.b.c` nests one in another to any
; depth; its dots keep their own token.
(table [(bare_key) (quoted_key) (dotted_key)] @type)
(table_array_element [(bare_key) (quoted_key) (dotted_key)] @type)

; Changed: upstream names the whole pair a property, its `=` and its
; spaces too; here the key alone.
(pair [(bare_key) (quoted_key) (dotted_key)] @property)

; Literals
;---------

; Changed: upstream names these `boolean`; they are the language's own
; words, as `true` is in JSON.
(boolean) @constant.builtin

(comment) @comment

(string) @string
; Added: an escape is a token of its own inside a string.
(escape_sequence) @escape

[
  (integer)
  (float)
] @number

[
  (offset_date_time)
  (local_date_time)
  (local_date)
  (local_time)
] @string.special

; Punctuation
;------------

[
  "."
  ","
] @punctuation.delimiter

"=" @operator

[
  "["
  "]"
  "[["
  "]]"
  "{"
  "}"
] @punctuation.bracket
