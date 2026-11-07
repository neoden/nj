; Java's highlights, after the query of tree-sitter-java 0.23.5 (MIT), with
; the changes marked below. The last pattern that names a node wins, as in
; tree-sitter's own highlighter, so for any node the general patterns come
; first and the specific ones after them.

; Variables

(identifier) @variable

; Methods

(method_declaration
  name: (identifier) @function.method)
(method_invocation
  name: (identifier) @function.method)
(super) @function.builtin

; Annotations

(annotation
  name: (identifier) @attribute)
(marker_annotation
  name: (identifier) @attribute)

"@" @operator
; Added: the `@` of an annotation is the annotation's, `@Override` one
; token, not an operator before a name.
(annotation "@" @attribute)
(marker_annotation "@" @attribute)

; Modules

; Added: a name a path goes through is a package, a module: `java` and
; `util` in `import java.util.List;`, every name of `package com.acme;`.
; The grammar nests a path in its scope to any depth, and each name but
; the first is the name of a path that is the scope of the next.
(scoped_identifier
  scope: (identifier) @module)
(scoped_identifier
  scope: (scoped_identifier
    name: (identifier) @module))
(package_declaration
  (scoped_identifier
    name: (identifier) @module))
(package_declaration
  (identifier) @module)
; Added: the modules and packages `module-info.java` names, their last name
; too: `app` in `module com.acme.app`, `sql` in `requires java.sql`.
(module_declaration
  name: [
    (identifier) @module
    (scoped_identifier name: (identifier) @module)
  ])
(requires_module_directive
  module: [
    (identifier) @module
    (scoped_identifier name: (identifier) @module)
  ])
(exports_module_directive
  [
    package: (identifier) @module
    package: (scoped_identifier name: (identifier) @module)
    modules: (identifier) @module
    modules: (scoped_identifier name: (identifier) @module)
  ])
(opens_module_directive
  [
    package: (identifier) @module
    package: (scoped_identifier name: (identifier) @module)
    modules: (identifier) @module
    modules: (scoped_identifier name: (identifier) @module)
  ])

; Types

(type_identifier) @type

(interface_declaration
  name: (identifier) @type)
(class_declaration
  name: (identifier) @type)
(enum_declaration
  name: (identifier) @type)
; Added: a record is a class, as an enum is, and so is an annotation's
; declaration, `Tag` in `@interface Tag`; a record taken apart in a
; pattern, `Point` in `case Point(int x, int y)`, is its class.
(record_declaration
  name: (identifier) @type)
(annotation_type_declaration
  name: (identifier) @type)
(record_pattern
  (identifier) @type)

((field_access
  object: (identifier) @type)
 (#match? @type "^[A-Z]"))
((scoped_identifier
  scope: (identifier) @type)
 (#match? @type "^[A-Z]"))
; Added: a class in the middle of a path, `Math` in `import static
; java.lang.Math.max;`, which the pattern for a package names a module.
((scoped_identifier
  scope: (scoped_identifier
    name: (identifier) @type))
 (#match? @type "^[A-Z]"))
((method_invocation
  object: (identifier) @type)
 (#match? @type "^[A-Z]"))
((method_reference
  . (identifier) @type)
 (#match? @type "^[A-Z]"))
; Added: the class an import names at its end, `List` in `import
; java.util.List;`; a method a static import names stays a name.
((import_declaration
  (scoped_identifier
    name: (identifier) @type))
 (#match? @type "^[A-Z]"))

(constructor_declaration
  name: (identifier) @type)

[
  (boolean_type)
  (integral_type)
  (floating_point_type)
  (floating_point_type)
  (void_type)
] @type.builtin

; Added: `var` in `var x = 1;` is a word of the language, which the grammar
; reads as a type's name.
((type_identifier) @keyword
 (#eq? @keyword "var"))
; Added: `default` after `case null,` is the grammar's name, though the
; word is reserved and can name nothing.
((identifier) @keyword
 (#eq? @keyword "default"))

; Constants

; Changed: upstream wants two letters at least, which makes `static final
; int N` a variable; one is enough, as in the other languages' queries.
((identifier) @constant
 (#match? @constant "^_*[A-Z][A-Z\\d_]*$"))

; Builtins

(this) @variable.builtin
; Added: `super` in a wildcard's bound, `? super V`, is a keyword, as
; `extends` is there; the pattern for `super` above names it a builtin.
(wildcard (super) @keyword)

; Literals

[
  (hex_integer_literal)
  (decimal_integer_literal)
  (octal_integer_literal)
  (decimal_floating_point_literal)
  (hex_floating_point_literal)
] @number
; Added: a number in binary, `0b1010`, which upstream leaves out.
(binary_integer_literal) @number

[
  (character_literal)
  (string_literal)
] @string
; Changed: upstream names this `string.escape`, which is a string here,
; the longest start of the name known.
(escape_sequence) @escape

[
  (true)
  (false)
  (null_literal)
] @constant.builtin

[
  (line_comment)
  (block_comment)
] @comment

; Added: a label, where it stands and where `break` or `continue` names it.
(labeled_statement
  (identifier) @label)
(break_statement
  (identifier) @label)
(continue_statement
  (identifier) @label)

; Keywords

[
  "abstract"
  "assert"
  "break"
  "case"
  "catch"
  "class"
  "continue"
  "default"
  "do"
  "else"
  "enum"
  "exports"
  "extends"
  "final"
  "finally"
  "for"
  "if"
  "implements"
  "import"
  "instanceof"
  "interface"
  "module"
  "native"
  "new"
  "non-sealed"
  "open"
  "opens"
  "package"
  "permits"
  "private"
  "protected"
  "provides"
  "public"
  "requires"
  "record"
  "return"
  "sealed"
  "static"
  "strictfp"
  "switch"
  "synchronized"
  "throw"
  "throws"
  "to"
  "transient"
  "transitive"
  "try"
  "uses"
  "volatile"
  "when"
  "while"
  "with"
  "yield"
  ; Added: the word that declares an annotation, which upstream leaves out.
  "@interface"
] @keyword
