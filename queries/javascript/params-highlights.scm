; JavaScript's parameters where they are declared, after the query of
; tree-sitter-javascript 0.25.0 (MIT), with the change marked below. Read
; after `highlights.scm` and `jsx-highlights.scm`, by JavaScript alone:
; TypeScript's parameters are built otherwise.

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
