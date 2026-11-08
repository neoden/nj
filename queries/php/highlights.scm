; PHP's highlights, after the query of tree-sitter-php 0.25.1 (MIT), with
; the changes marked below. The text outside `<?php ... ?>` is HTML's,
; read by `injections.scm`. The last pattern that names a node wins, as
; in tree-sitter's own highlighter, so for any node the general patterns
; come first and the specific ones after them.

[
  (php_tag)
  (php_end_tag)
] @tag

; Keywords

[
  "and"
  "as"
  "break"
  "case"
  "catch"
  "class"
  "clone"
  "const"
  "continue"
  "declare"
  "default"
  "do"
  "echo"
  "else"
  "elseif"
  "enddeclare"
  "endfor"
  "endforeach"
  "endif"
  "endswitch"
  "endwhile"
  "enum"
  "exit"
  "extends"
  "finally"
  "fn"
  "for"
  "foreach"
  "function"
  "global"
  "goto"
  "if"
  "implements"
  "include"
  "include_once"
  "instanceof"
  "insteadof"
  "interface"
  "match"
  "namespace"
  "new"
  "or"
  "print"
  "require"
  "require_once"
  "return"
  "switch"
  "throw"
  "trait"
  "try"
  "use"
  "while"
  "xor"
  "yield"
  "yield from"
  ; Added: `unset`, a statement as `echo` is, and the directives of
  ; `declare(strict_types=1)`.
  "unset"
  "strict_types"
  "ticks"
  "encoding"
  (abstract_modifier)
  (final_modifier)
  (readonly_modifier)
  (static_modifier)
  (visibility_modifier)
] @keyword

(function_static_declaration "static" @keyword)

; Namespace

(namespace_definition
  name: (namespace_name
    (name) @module))

(namespace_name
  (name) @module)

(namespace_use_clause
  [
    (name) @type
    (qualified_name
      (name) @type)
    alias: (name) @type
  ])

(namespace_use_clause
  type: "function"
  [
    (name) @function
    (qualified_name
      (name) @function)
    alias: (name) @function
  ])

(namespace_use_clause
  type: "const"
  [
    (name) @constant
    (qualified_name
      (name) @constant)
    alias: (name) @constant
  ])

(relative_name "namespace" @module.builtin)

; Variables

(relative_scope) @variable.builtin

(variable_name) @variable

(method_declaration name: (name) @constructor
  (#eq? @constructor "__construct"))

(object_creation_expression [
  (name) @constructor
  (qualified_name (name) @constructor)
  (relative_name (name) @constructor)
])

((name) @constant
 (#match? @constant "^_?[A-Z][A-Z\\d_]+$"))
; Changed: upstream writes `\d` with one backslash, which a query's string
; reads as a `d`, so that `__LINE2__` would not match; two are a digit.
((name) @constant.builtin
 (#match? @constant.builtin "^__[A-Z][A-Z\\d_]+__$"))
(const_declaration (const_element (name) @constant))
; Added: an enum's case is a constant, `Hearts` in `case Hearts = 'H';`,
; as an enum's member is in the other languages.
(enum_case name: (name) @constant)

; Types

(primitive_type) @type.builtin
(cast_type) @type.builtin
(named_type [
  (name) @type
  (qualified_name (name) @type)
  (relative_name (name) @type)
]) @type
(named_type (name) @type.builtin
  (#any-of? @type.builtin "static" "self"))

(scoped_call_expression
  scope: [
    (name) @type
    (qualified_name (name) @type)
    (relative_name (name) @type)
  ])

; Added: a class, an interface, a trait and an enum where they are
; declared, and the ones a class extends, implements and uses, which
; upstream names only in a type or a call: `UserController`, `Controller`
; and `HasRoutes` in `class UserController extends Controller implements
; HasRoutes`, `Loggable` in `use Loggable;`.
(class_declaration name: (name) @type)
(interface_declaration name: (name) @type)
(trait_declaration name: (name) @type)
(enum_declaration name: (name) @type)
(base_clause
  [
    (name) @type
    (qualified_name (name) @type)
  ])
(class_interface_clause
  [
    (name) @type
    (qualified_name (name) @type)
  ])
(use_declaration
  [
    (name) @type
    (qualified_name (name) @type)
  ])

; Added: the class before `::class`, `U` in `U::class`, and the word, and
; the class after `instanceof`.
(class_constant_access_expression
  .
  [
    (name) @type
    (qualified_name (name) @type)
  ])
((class_constant_access_expression
  (name) @keyword)
 (#eq? @keyword "class"))
(binary_expression
  operator: "instanceof"
  right: [
    (name) @type
    (qualified_name (name) @type)
  ])

; Functions

(array_creation_expression "array" @function.builtin)
(list_literal "list" @function.builtin)
(exit_statement "exit" @function.builtin "(")

(method_declaration
  name: (name) @function.method)

; Changed: a call is `function.call`, a method's `function.method.call`,
; a static one's too, told from where a function is defined, as Neovim's
; queries have it.
(function_call_expression
  function: [
    (qualified_name (name))
    (relative_name (name))
    (name)
  ] @function.call)

(scoped_call_expression
  name: (name) @function.method.call)

(member_call_expression
  name: (name) @function.method.call)

(function_definition
  name: (name) @function)

; Member

(property_element
  (variable_name) @property)

(member_access_expression
  name: (variable_name (name)) @property)
(member_access_expression
  name: (name) @property)

; Basic tokens
[
  (string)
  (string_content)
  (encapsed_string)
  (heredoc)
  (heredoc_body)
  (nowdoc_body)
] @string
; Added: an escape is a token of its own inside a string, `\n` in "a\n".
(escape_sequence) @escape
(boolean) @constant.builtin
(null) @constant.builtin
(integer) @number
(float) @number
(comment) @comment

; Added: an attribute, `#[Route('/x')]`, is an attribute, as Rust's is:
; its `#[` and `]` and its name; its arguments keep their colours.
(attribute_list) @attribute
(attribute
  [
    (name) @attribute
    (qualified_name (name) @attribute)
  ])

; Added: the constructs PHP spells as calls, `isset($x)` and `empty($x)`,
; are the language's own.
(function_call_expression
  function: (name) @function.builtin
  (#any-of? @function.builtin "isset" "empty" "eval" "die"))

((name) @variable.builtin
 (#eq? @variable.builtin "this"))

"$" @operator
