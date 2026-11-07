; YAML's highlights, after the query of tree-sitter-yaml 0.7.2 (MIT), with
; the changes marked below. The last pattern that names a node wins, as in
; tree-sitter's own highlighter, so for any node the general patterns come
; first and the specific ones after them.

; Changed: upstream names these `boolean`; they are the language's own
; words, as `true` is in JSON.
(boolean_scalar) @constant.builtin

(null_scalar) @constant.builtin

[
  (double_quote_scalar)
  (single_quote_scalar)
  (block_scalar)
  (string_scalar)
] @string

; Added: an escape is a token of its own inside a double-quoted string.
(escape_sequence) @escape

[
  (integer_scalar)
  (float_scalar)
] @number

; A date is a string: the grammar is built for YAML 1.2's core schema,
; which has no dates, as it has no `yes` and `on`; only its legacy one
; has `timestamp_scalar`.

(comment) @comment

; Changed: upstream names the name alone, and the `&` or `*` before it
; punctuation; here the anchor and the alias are one token each, `&base`
; and `*base`.
[
  (anchor)
  (alias)
] @label
[
  (anchor_name)
  (alias_name)
] @label

(tag) @type

[
  (yaml_directive)
  (tag_directive)
  (reserved_directive)
] @attribute

; Keys. Changed: upstream names a key only if it is a string, so that
; `1:` is a number and `true:` a constant; a key is a key whatever it
; looks like.
(block_mapping_pair
  key: (flow_node
    [
      (double_quote_scalar)
      (single_quote_scalar)
    ] @property))

(block_mapping_pair
  key: (flow_node
    (plain_scalar
      (_) @property)))

(flow_mapping
  (_
    key: (flow_node
      [
        (double_quote_scalar)
        (single_quote_scalar)
      ] @property)))

(flow_mapping
  (_
    key: (flow_node
      (plain_scalar
        (_) @property))))

[
  ","
  "-"
  ":"
  ">"
  "?"
  "|"
] @punctuation.delimiter

[
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

; Changed: upstream has `&` and `*` here too, the inner token, which wins
; over the anchor's and the alias's.
[
  "---"
  "..."
] @punctuation.special
