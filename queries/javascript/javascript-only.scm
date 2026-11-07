; The patterns of JavaScript's highlights that TypeScript cannot share:
; its grammar builds a parameter otherwise and names a class with a
; `type_identifier`, which its own query names a type. Read after
; `highlights.scm` and `jsx-highlights.scm`, by JavaScript alone.

; JavaScript's parameters where they are declared, after the query of
; tree-sitter-javascript 0.25.0 (MIT), its `highlights-params.scm`.
(formal_parameters
  [
    (identifier) @variable.parameter
    (array_pattern
      (identifier) @variable.parameter)
    (object_pattern
      [
        (pair_pattern value: (identifier) @variable.parameter)
        (shorthand_property_identifier_pattern) @variable.parameter
      ])
  ]
)

; Added: a parameter with a default, `a` in `(a = 1)` and `greeting` in
; `({ greeting = "hi" })`, and the rest of them, `rest` in `(...rest)`,
; which upstream leaves plain names.
(formal_parameters
  [
    (assignment_pattern
      left: (identifier) @variable.parameter)
    (object_pattern
      (object_assignment_pattern
        left: (shorthand_property_identifier_pattern) @variable.parameter))
    (rest_pattern
      (identifier) @variable.parameter)
  ]
)

; Added: a class's name where it is declared, whatever its case, as it is
; where it is used with a capital.
(class_declaration
  name: (identifier) @type)
(class
  name: (identifier) @type)
