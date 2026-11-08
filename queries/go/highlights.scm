; Go's highlights, after the query of tree-sitter-go 0.25.0 (MIT), with
; the changes marked below. The last pattern that names a node wins, as in
; tree-sitter's own highlighter, so for any node the general patterns come
; first and the specific ones after them.

; Identifiers

; Changed: upstream has these three after the functions, so that every
; call and every function is named a variable, every method a property.
(type_identifier) @type
(field_identifier) @property
(identifier) @variable

; Added: a package where the grammar names it, in a type, a `package`
; clause and an import's alias. In `config.Load()` it is an identifier,
; not told from a variable.
(package_identifier) @module

; Added: a type parameter is a type where it is declared, as it is where
; it is used: `T` in `func Map[T any](xs []T)`.
(type_parameter_declaration
  name: (identifier) @type)

; Added: a constant where it is declared, `MaxSize` in `const MaxSize =
; 10`; Go does not spell constants in capitals, so their look does not
; tell. Used later it is a name like any other.
(const_spec
  name: (identifier) @constant)

; Added: a key in a composite literal is a field, `cfg` in `App{cfg: c}`.
; In a map's literal, `M{k: v}`, it is a variable, and the grammar cannot
; tell the two apart when the type is a name; a struct is the likelier.
(keyed_element
  key: (literal_element
    (identifier) @property))

; Added: a label, where it stands and where `break` or `goto` names it.
(label_name) @label

; Function calls
; Changed: a call is `function.call`, a method's `function.method.call`,
; told from where a function is declared, as Neovim's queries have it.

(call_expression
  function: (identifier) @function.call)

; Changed: upstream's list stops before Go 1.21, which added `clear`,
; `max` and `min`.
(call_expression
  function: (identifier) @function.builtin
  (#match? @function.builtin "^(append|cap|clear|close|complex|copy|delete|imag|len|make|max|min|new|panic|print|println|real|recover)$"))

; Added: a conversion to a predeclared type is the type, though the
; grammar reads it as a call: `int64` in `int64(x)`. Known by its text; a
; conversion to a type of one's own, `Celsius(x)`, stays a call.
(call_expression
  function: (identifier) @type
  (#match? @type "^(any|bool|byte|complex64|complex128|error|float32|float64|int|int8|int16|int32|int64|rune|string|uint|uint8|uint16|uint32|uint64|uintptr)$"))

(call_expression
  function: (selector_expression
    field: (field_identifier) @function.method.call))

; Function definitions

(function_declaration
  name: (identifier) @function)

(method_declaration
  name: (field_identifier) @function.method)

; Added: a method an interface asks for, `Read` in `interface { Read(p
; []byte) }`, which is a property by the field pattern above.
(method_elem
  name: (field_identifier) @function.method)

; Operators

[
  "--"
  "-"
  "-="
  ":="
  "!"
  "!="
  "..."
  "*"
  "*"
  "*="
  "/"
  "/="
  "&"
  "&&"
  "&="
  "%"
  "%="
  "^"
  "^="
  "+"
  "++"
  "+="
  "<-"
  "<"
  "<<"
  "<<="
  "<="
  "="
  "=="
  ">"
  ">="
  ">>"
  ">>="
  "|"
  "|="
  "||"
  "~"
] @operator

; Keywords

[
  "break"
  "case"
  "chan"
  "const"
  "continue"
  "default"
  "defer"
  "else"
  "fallthrough"
  "for"
  "func"
  "go"
  "goto"
  "if"
  "import"
  "interface"
  "map"
  "package"
  "range"
  "return"
  "select"
  "struct"
  "switch"
  "type"
  "var"
] @keyword

; Literals

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
  (nil)
  (iota)
] @constant.builtin

(comment) @comment
