; TypeScript's highlights, after the query of tree-sitter-typescript
; 0.23.2 (MIT), with the changes marked below. Read after JavaScript's
; `highlights.scm`, and for TSX after its `jsx-highlights.scm` too, as
; upstream has it, so these patterns win over JavaScript's for the same
; node. JavaScript's `javascript-only.scm` is not read: a parameter here
; is built otherwise, and named below, and a class's name is a type.

; Types

(type_identifier) @type
(predefined_type) @type.builtin
; Added: `void` as a type is a token JavaScript's query names a keyword,
; the inner token, which wins over the type around it.
(predefined_type "void" @type.builtin)
; Added: `this` as a type, in `(): this`, is the language's, as it is as a
; value.
(this_type) @type.builtin

; Changed: upstream names every identifier with a capital a type, after
; JavaScript's patterns, which then lose to it: `function App()` would be
; a type again. JavaScript's query names such an identifier a constructor
; already, which looks the same.

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)
; Added: the brackets of type parameters, `<T>` in `function f<T>()`.
(type_parameters
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

; Variables

; Changed: upstream names any identifier in a parameter, its default's
; too, so that `y` in `(x = y)` is a parameter. Here the parameter's
; pattern, of every form JavaScript's has: `x`, `{ a, b = 1 }`, `[c]`,
; `...rest`.
(required_parameter
  pattern: [
    (identifier) @variable.parameter
    (rest_pattern (identifier) @variable.parameter)
    (array_pattern (identifier) @variable.parameter)
    (object_pattern
      [
        (shorthand_property_identifier_pattern) @variable.parameter
        (pair_pattern value: (identifier) @variable.parameter)
        (object_assignment_pattern
          left: (shorthand_property_identifier_pattern) @variable.parameter)
      ])
  ])
(optional_parameter
  pattern: (identifier) @variable.parameter)

; Added: what an interface or an object type declares, a method a function
; and the rest properties, as in a class: `greet(): void` and `name:
; string`. A function declared without a body, an overload, is a function
; too.
(property_signature
  name: (property_identifier) @property)
(method_signature
  name: (property_identifier) @function.method)
(abstract_method_signature
  name: (property_identifier) @function.method)
(function_signature
  name: (identifier) @function)

; Added: an enum is a type and its members constants, declared and with a
; value, as an enum's are in C: `enum Color { Red, Green = 2 }`.
(enum_declaration
  name: (identifier) @type)
(enum_body
  (property_identifier) @constant)
(enum_assignment
  name: (property_identifier) @constant)

; Added: a namespace's name is a module, `Shapes` in `namespace Shapes {}`.
(internal_module
  name: (identifier) @module)
(internal_module
  name: (nested_identifier (identifier) @module))
(internal_module
  name: (nested_identifier (property_identifier) @module))

; Keywords

[ "abstract"
  "declare"
  "enum"
  "export"
  "implements"
  "interface"
  "keyof"
  "namespace"
  "private"
  "protected"
  "public"
  "type"
  "readonly"
  "override"
  "satisfies"
  ; Added: the other words TypeScript adds, which upstream leaves plain:
  ; `x is string`, `infer U`, `asserts x`, `accessor`, `declare module`,
  ; `declare global`.
  "is"
  "infer"
  "asserts"
  "accessor"
  "module"
  "global"
] @keyword
