; Python's highlights, after the query of tree-sitter-python 0.25 (MIT).
; The first pattern that names a node wins, so for any node the specific
; patterns come before the general ones.

; Definitions

(function_definition
  name: (identifier) @function)

(class_definition
  name: (identifier) @type)

(decorator) @function
(decorator
  (identifier) @function)

; Calls, a builtin before any other function

((call
  function: (identifier) @function.builtin)
 (#match?
   @function.builtin
   "^(abs|all|any|ascii|bin|bool|breakpoint|bytearray|bytes|callable|chr|classmethod|compile|complex|delattr|dict|dir|divmod|enumerate|eval|exec|filter|float|format|frozenset|getattr|globals|hasattr|hash|help|hex|id|input|int|isinstance|issubclass|iter|len|list|locals|map|max|memoryview|min|next|object|oct|open|ord|pow|print|property|range|repr|reversed|round|set|setattr|slice|sorted|staticmethod|str|sum|super|tuple|type|vars|zip|__import__)$"))

(call
  function: (attribute attribute: (identifier) @function.method))
(call
  function: (identifier) @function)

(attribute attribute: (identifier) @property)
(type (identifier) @type)

; Naming conventions, for a name nothing above has claimed: all capitals
; is a constant, a capital first a class

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z_]*$"))

((identifier) @constructor
 (#match? @constructor "^[A-Z]"))

(identifier) @variable

; Literals

[
  (none)
  (true)
  (false)
] @constant.builtin

[
  (integer)
  (float)
] @number

(comment) @comment
(string) @string
(escape_sequence) @escape

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
