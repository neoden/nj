; C#'s highlights, after the query of tree-sitter-c-sharp 0.23.5 (MIT),
; with the changes marked below. The last pattern that names a node wins,
; as in tree-sitter's own highlighter, so for any node the general
; patterns come first and the specific ones after them.

(identifier) @variable

;; Members

; Added: a member reached through a dot, `user.Name`, is a property; a
; call through one is a method, below.
(member_access_expression
  name: (identifier) @property)

;; Methods

(method_declaration name: (identifier) @function)
(local_function_statement name: (identifier) @function)

;; Types

(interface_declaration name: (identifier) @type)
(class_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)
(struct_declaration (identifier) @type)
(record_declaration (identifier) @type)
; Added: a delegate is a type, as an interface is.
(delegate_declaration name: (identifier) @type)

; Changed: upstream names the name of a namespace a module only when it
; is one word; `A.B` nests a name in another to any depth, each name a
; child of one. Added: a file-scoped namespace, `namespace A.B;`, and the
; namespaces a `using` brings in, `using System.Text;`. Four names deep.
(namespace_declaration
  name: [
    (identifier) @module
    (qualified_name (identifier) @module)
    (qualified_name (qualified_name (identifier) @module))
    (qualified_name (qualified_name (qualified_name (identifier) @module)))
  ])
(file_scoped_namespace_declaration
  name: [
    (identifier) @module
    (qualified_name (identifier) @module)
    (qualified_name (qualified_name (identifier) @module))
    (qualified_name (qualified_name (qualified_name (identifier) @module)))
  ])
(using_directive
  [
    (identifier) @module
    (qualified_name (identifier) @module)
    (qualified_name (qualified_name (identifier) @module))
    (qualified_name (qualified_name (qualified_name (identifier) @module)))
  ])

(generic_name (identifier) @type)
; Changed: upstream names a type parameter and the name in its constraint
; a property's definition, plain text here; `T` in `class Box<T> where T :
; new()` is a type where it is declared and where it is bound.
(type_parameter (identifier) @type)
(parameter type: (identifier) @type)
(type_argument_list (identifier) @type)
(as_expression right: (identifier) @type)
(is_expression right: (identifier) @type)

(constructor_declaration name: (identifier) @constructor)
(destructor_declaration name: (identifier) @constructor)

(_ type: (identifier) @type)

(base_list (identifier) @type)

(predefined_type) @type.builtin

(type_parameter_constraints_clause (identifier) @type)

;; Enum

; Changed: upstream names an enum's member a property's definition; it is
; a constant, as an enum's member is in C, TypeScript and Java.
(enum_member_declaration (identifier) @constant)

;; Literals

[
  (real_literal)
  (integer_literal)
] @number

[
  (character_literal)
  (string_literal)
  (raw_string_literal)
  (verbatim_string_literal)
  (interpolated_string_expression)
  (interpolation_start)
  (interpolation_quote)
 ] @string

; Changed: upstream names this `string.escape`, which is a string here,
; the longest start of the name known.
(escape_sequence) @escape

[
  (boolean_literal)
  (null_literal)
] @constant.builtin

;; Comments

(comment) @comment

;; Tokens

[
  ";"
  "."
  ","
] @punctuation.delimiter

[
  "--"
  "-"
  "-="
  "&"
  "&="
  "&&"
  "+"
  "++"
  "+="
  "<"
  "<="
  "<<"
  "<<="
  "="
  "=="
  "!"
  "!="
  "=>"
  ">"
  ">="
  ">>"
  ">>="
  ">>>"
  ">>>="
  "|"
  "|="
  "||"
  "?"
  "??"
  "??="
  "^"
  "^="
  "~"
  "*"
  "*="
  "/"
  "/="
  "%"
  "%="
  ":"
  ".."
] @operator

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
  (interpolation_brace)
]  @punctuation.bracket

;; Keywords

[
  (modifier)
  "this"
  (implicit_type)
] @keyword

[
  "add"
  "alias"
  "as"
  "base"
  "break"
  "case"
  "catch"
  "checked"
  "class"
  "continue"
  "default"
  "delegate"
  "do"
  "else"
  "enum"
  "event"
  "explicit"
  "extern"
  "finally"
  "for"
  "foreach"
  "global"
  "goto"
  "if"
  "implicit"
  "interface"
  "is"
  "lock"
  "namespace"
  "notnull"
  "operator"
  "params"
  "return"
  "remove"
  "sizeof"
  "stackalloc"
  "static"
  "struct"
  "switch"
  "throw"
  "try"
  "typeof"
  "unchecked"
  "using"
  "while"
  "new"
  "await"
  "in"
  "yield"
  "get"
  "set"
  "when"
  "out"
  "ref"
  "from"
  "where"
  "select"
  "record"
  "init"
  "with"
  "let"
  ; Added: the rest of a query's words, `orderby x descending`, `join ...
  ; on ... equals`, `group ... by ... into`; a pattern's, `is not null`,
  ; `> 0 and < 10`; and the statements that are words of their own.
  "orderby"
  "ascending"
  "descending"
  "join"
  "on"
  "equals"
  "group"
  "by"
  "into"
  "and"
  "or"
  "not"
  "fixed"
  "unsafe"
  "scoped"
  "managed"
  "unmanaged"
] @keyword

; Added: what an attribute is for, `return:` in `[return: NotNull]`; and
; `assembly:` in `[assembly: X]`, which the grammar reads as an attribute
; of its own kind, its word a token of it.
(attribute_target_specifier) @keyword
(global_attribute
  ["assembly" "module"] @keyword)

;; Preprocessor

; Added: the directives, which upstream leaves plain: `#if DEBUG`,
; `#region Setup`, `#nullable enable`, `#pragma warning disable`.
[
  "#if"
  "#elif"
  "#else"
  "#endif"
  "#region"
  "#endregion"
  "#define"
  "#undef"
  "#warning"
  "#error"
  "#line"
  "#nullable"
  "#pragma"
  "enable"
  "disable"
  "restore"
  "warning"
  "warnings"
  "annotations"
  "checksum"
  "hidden"
] @keyword

;; Labels

; Added: a label where it stands and where `goto` names it.
(labeled_statement (identifier) @label)
(goto_statement (identifier) @label)

;; Attribute

(attribute name: (identifier) @attribute)

;; Parameters

(parameter
  name: (identifier) @variable.parameter)
; Added: a `params` parameter, `xs` in `params int[] xs`, which the
; grammar puts in the list itself, not in a parameter.
(parameter_list
  name: (identifier) @variable.parameter)

;; Method calls

; Changed: a call is `function.call`, a method's `function.method.call`,
; told from where a method is declared, as Neovim's queries have it.
(invocation_expression (member_access_expression name: (identifier) @function.method.call))
; Added: a call by name, `Run()`, and of a generic, `Parse<int>()`, which
; upstream leaves a variable and a type.
(invocation_expression
  function: (identifier) @function.call)
(invocation_expression
  function: (generic_name (identifier) @function.call))
(invocation_expression
  (member_access_expression
    name: (generic_name (identifier) @function.method.call)))

; Added: `nameof`, a word of the language spelt as a call.
((invocation_expression
  function: (identifier) @function.builtin)
 (#eq? @function.builtin "nameof"))
