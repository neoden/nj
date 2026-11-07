; Python's highlights, after the query of tree-sitter-python 0.25 (MIT),
; with the changes marked below. The last pattern that names a node wins,
; as in tree-sitter's own highlighter, so for any node the general
; patterns come first and the specific ones after them.

(identifier) @variable

; Added: parameters where they are declared, as in Rust's query.
(parameters (identifier) @variable.parameter)
(parameters (list_splat_pattern (identifier) @variable.parameter))
(parameters (dictionary_splat_pattern (identifier) @variable.parameter))
(lambda_parameters (identifier) @variable.parameter)
(typed_parameter (identifier) @variable.parameter)
(typed_parameter (list_splat_pattern (identifier) @variable.parameter))
(typed_parameter (dictionary_splat_pattern (identifier) @variable.parameter))
(default_parameter name: (identifier) @variable.parameter)
(typed_default_parameter name: (identifier) @variable.parameter)

; Added: `self` and `cls` are names like any other to the grammar, the
; first parameter of a method by convention; known by their text.
((identifier) @variable.builtin
 (#any-of? @variable.builtin "self" "cls"))

; Changed: upstream has these two after the calls, so that a method
; called would be a property.
(attribute attribute: (identifier) @property)
(type (identifier) @type)
; Added: the generic in an annotation, `list` in `list[int]`.
(generic_type (identifier) @type)

; Function calls

(decorator) @function
(decorator
  (identifier) @function)

(call
  function: (attribute attribute: (identifier) @function.method))
(call
  function: (identifier) @function)

; Builtin functions

((call
  function: (identifier) @function.builtin)
 (#match?
   @function.builtin
   "^(abs|all|any|ascii|bin|bool|breakpoint|bytearray|bytes|callable|chr|classmethod|compile|complex|delattr|dict|dir|divmod|enumerate|eval|exec|filter|float|format|frozenset|getattr|globals|hasattr|hash|help|hex|id|input|int|isinstance|issubclass|iter|len|list|locals|map|max|memoryview|min|next|object|oct|open|ord|pow|print|property|range|repr|reversed|round|set|setattr|slice|sorted|staticmethod|str|sum|super|tuple|type|vars|zip|__import__)$"))

; Function definitions

(function_definition
  name: (identifier) @function)

; Added: the name in a class definition.
(class_definition
  name: (identifier) @type)

; Identifier naming conventions
; Changed: upstream has these before the calls, so that `Greeter()`, which
; makes a `Greeter`, is a function. Here a name's look has the last word
; over where it stands, as in Rust's query: `Greeter()`, `ValueError("no")`
; and `isinstance(x, Greeter)` are all the class.

; Changed: upstream calls this a constructor; it is a class.
((identifier) @type
 (#match? @type "^[A-Z]"))

; Changed: upstream leaves digits out, so that `HTTP2` is a class.
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

; Literals

[
  (none)
  (true)
  (false)
] @constant.builtin

; Added: `...` is a constant of the language too, `Ellipsis`.
(ellipsis) @constant.builtin

[
  (integer)
  (float)
] @number

(comment) @comment
(string) @string
(escape_sequence) @escape

; Added: a string standing first in a module, a class or a function is
; its docstring.
(module . (expression_statement (string) @string.documentation))
(class_definition
  body: (block . (expression_statement (string) @string.documentation)))
(function_definition
  body: (block . (expression_statement (string) @string.documentation)))

(interpolation
  "{" @punctuation.special
  "}" @punctuation.special) @embedded

; Operators. The ones spelt as words are keywords here: `in` is the same
; token in `for x in xs` as in `x in xs`, and reads as one.

[
  "-"
  "-="
  "!="
  "*"
  "**"
  "**="
  "*="
  "/"
  "//"
  "//="
  "/="
  "&"
  "&="
  "%"
  "%="
  "^"
  "^="
  "+"
  "->"
  "+="
  "<"
  "<<"
  "<<="
  "<="
  "<>"
  "="
  ":="
  "=="
  ">"
  ">="
  ">>"
  ">>="
  "|"
  "|="
  "~"
  "@="
] @operator

; Added: `@` between two values, matrix multiplication. Not the `@` of a
; decorator, which is the decorator's.
(binary_operator "@" @operator)

[
  "and"
  "in"
  "is"
  "not"
  "or"
  "is not"
  "not in"
  "as"
  "assert"
  "async"
  "await"
  "break"
  "class"
  "continue"
  "def"
  "del"
  "elif"
  "else"
  "except"
  "exec"
  "finally"
  "for"
  "from"
  "global"
  "if"
  "import"
  "lambda"
  "nonlocal"
  "pass"
  "print"
  "raise"
  "return"
  "try"
  "while"
  "with"
  "yield"
  "match"
  "case"
] @keyword

; Added: `type` in `type Point = ...`, a keyword only there; elsewhere it
; is the builtin `type`.
(type_alias_statement "type" @keyword)
