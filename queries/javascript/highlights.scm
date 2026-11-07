; JavaScript's highlights, after the query of tree-sitter-javascript 0.25.0
; (MIT), with the changes marked below. Its JSX and its parameters are in
; files of their own, read after this one, as upstream has them:
; TypeScript reads this one too, and its grammar has neither JSX nor the
; same parameters. The last pattern that names a node wins, as in
; tree-sitter's own highlighter, so for any node the general patterns
; come first and the specific ones after them.

; Variables
;----------

(identifier) @variable

; Properties
;-----------

(property_identifier) @property
; Added: a private field, `#count` in `this.#count`, is a property too.
(private_property_identifier) @property

; Function and method calls
;--------------------------

(call_expression
  function: (identifier) @function)

(call_expression
  function: (member_expression
    property: [(property_identifier) (private_property_identifier)] @function.method))

; Added: a decorator is a function, as Python's is: `@observable` and the
; call in `@Component({...})`.
(decorator (identifier) @function)
(decorator
  (call_expression
    function: (identifier) @function))

; Special identifiers
;--------------------

((identifier) @constructor
 (#match? @constructor "^[A-Z]"))

; Changed: upstream wants two letters at least, which makes `const N` a
; constructor; one is enough, as in Rust's and Python's queries.
([
    (identifier)
    (shorthand_property_identifier)
    (shorthand_property_identifier_pattern)
 ] @constant
 (#match? @constant "^[A-Z_][A-Z\\d_]*$"))

; Added: a class's name where it is declared, whatever its case, as it is
; where it is used with a capital.
(class_declaration
  name: (identifier) @type)
(class
  name: (identifier) @type)

; Function and method definitions
;--------------------------------

; Changed: upstream has these before the special identifiers, so that
; `function App()`, a React component, is a constructor by its capital.
; A function declared is a function whatever its name; called, its look
; decides, as `new Counter()` and `Counter()` both make one.

(function_expression
  name: (identifier) @function)
(function_declaration
  name: (identifier) @function)
(method_definition
  name: [(property_identifier) (private_property_identifier)] @function.method)

(pair
  key: (property_identifier) @function.method
  value: [(function_expression) (arrow_function)])

(assignment_expression
  left: (member_expression
    property: (property_identifier) @function.method)
  right: [(function_expression) (arrow_function)])

(variable_declarator
  name: (identifier) @function
  value: [(function_expression) (arrow_function)])

(assignment_expression
  left: (identifier) @function
  right: [(function_expression) (arrow_function)])

; Changed: upstream adds `(#is-not? local)` to these two, for a highlighter
; that tracks local names, which this one does not: a local `console`
; is the builtin's colour too.
((identifier) @variable.builtin
 (#match? @variable.builtin "^(arguments|module|console|window|document)$"))

((identifier) @function.builtin
 (#eq? @function.builtin "require"))

; Added: the two numbers JavaScript names, which look like classes.
((identifier) @constant.builtin
 (#any-of? @constant.builtin "NaN" "Infinity"))

; Literals
;---------

(this) @variable.builtin
(super) @variable.builtin

[
  (true)
  (false)
  (null)
  (undefined)
] @constant.builtin

(comment) @comment
; Added: the `#!` line that starts a script, and the `<!--` comment old
; browsers still accept.
(hash_bang_line) @comment
(html_comment) @comment

[
  (string)
  (template_string)
] @string
; Added: an escape is a token of its own inside a string.
(escape_sequence) @escape

(regex) @string.special
(number) @number

; Added: a label, where it stands and where `break` names it.
(statement_identifier) @label

; Tokens
;-------

[
  ";"
  (optional_chain)
  "."
  ","
] @punctuation.delimiter

[
  "-"
  "--"
  "-="
  "+"
  "++"
  "+="
  "*"
  "*="
  "**"
  "**="
  "/"
  "/="
  "%"
  "%="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "==="
  "!"
  "!="
  "!=="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "~"
  "^"
  "&"
  "|"
  "^="
  "&="
  "|="
  "&&"
  "||"
  "??"
  "&&="
  "||="
  "??="
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
]  @punctuation.bracket

(template_substitution
  "${" @punctuation.special
  "}" @punctuation.special) @embedded

; Added: the slashes of a regex are the regex's, `/ab+c/gi`, not the
; operator the pattern above names them.
(regex "/" @string.special)

[
  "as"
  "async"
  "await"
  "break"
  "case"
  "catch"
  "class"
  "const"
  "continue"
  "debugger"
  "default"
  "delete"
  "do"
  "else"
  "export"
  "extends"
  "finally"
  "for"
  "from"
  "function"
  "get"
  "if"
  "import"
  "in"
  "instanceof"
  "let"
  "new"
  "of"
  "return"
  "set"
  "static"
  "switch"
  "target"
  "throw"
  "try"
  "typeof"
  "var"
  "void"
  "while"
  "with"
  "yield"
  ; Added: `using`, a declaration disposed of at the end of its block.
  "using"
] @keyword
