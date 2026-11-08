; Helm's highlights: the `{{ ... }}` of Go's templates, after the query
; of tree-sitter-go-template (MIT) and Helix's, with the changes marked
; below. The text around them is not this query's.

; Changed: every function is a function, with no builtins among them.
; Upstream names Go's own, `len` and `printf`, builtins; a chart calls
; Sprig's and Helm's as often, `include` and `toYaml`, and none of them is
; the chart's own.
(function_call
  function: (identifier) @function)

(method_call
  method: (selector_expression
    field: (field_identifier) @function))

[
  (field)
  (field_identifier)
] @property

(variable) @variable

; Added: `.` alone, what the template is given, as `self` is to a method.
(dot) @variable.builtin

[
  "if"
  "else"
  "end"
  "range"
  "with"
  "template"
  "define"
  "block"
  "break"
  "continue"
] @keyword

[
  "|"
  ":="
  "="
] @operator

; Changed: the `.` of a field and of a selector only, not `.` alone,
; which is a node of its own around a `.` of its own.
(field
  "." @punctuation.delimiter)

(selector_expression
  "." @punctuation.delimiter)

"," @punctuation.delimiter

[
  "{{"
  "}}"
  "{{-"
  "-}}"
  "("
  ")"
] @punctuation.bracket

[
  (interpreted_string_literal)
  (raw_string_literal)
  (rune_literal)
] @string

(escape_sequence) @escape

[
  (int_literal)
  (float_literal)
  (imaginary_literal)
] @number

[
  (true)
  (false)
] @boolean

(nil) @constant.builtin

(comment) @comment
