; JavaScript's highlights, after the query of tree-sitter-javascript 0.25.0
; (MIT), with the changes marked below. TypeScript and TSX read this one
; too, before their own. JSX is in `jsx-highlights.scm`, which TypeScript's
; grammar has not, and what only JavaScript's grammar has, its parameters
; and its classes' names, in `javascript-only.scm`. The last pattern that names a node wins, as in
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

; Changed: a call is `function.call`, a method's `function.method.call`,
; told from where a function is defined, as Neovim's queries have it.

(call_expression
  function: (identifier) @function.call)

(call_expression
  function: (member_expression
    property: [(property_identifier) (private_property_identifier)] @function.method.call))

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

; Added: a decorator is an attribute, as Python's is, whatever its name:
; its `@` and the last name of it, `@observable`, the call in
; `@Component({...})`, a member in `@mobx.observable`.
(decorator "@" @attribute)
(decorator (identifier) @attribute)
(decorator
  (member_expression
    property: (property_identifier) @attribute))
(decorator
  (call_expression
    function: (identifier) @attribute))
(decorator
  (call_expression
    function: (member_expression
      property: (property_identifier) @attribute)))

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
