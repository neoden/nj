; SCSS's highlights, after the query of tree-sitter-scss 1.0.0 (MIT), with
; the changes marked below. Read after CSS's `highlights.scm`, as
; upstream's is, so these patterns win over CSS's for the same node.

; Changed: upstream has these two after the names of mixins and
; functions, which they then name variables.
(identifier) @variable
(variable) @variable

[
  "@at-root"
  "@debug"
  "@error"
  "@extend"
  "@forward"
  "@mixin"
  "@use"
  "@warn"
  ; Added: the conditions, which upstream leaves out: `@if`, `@else` and
  ; the `if` of `@else if`.
  "@if"
  "@else"
  "if"
] @keyword

"@function" @keyword.function

"@return" @keyword.return

"@include" @keyword.import

[
  "@while"
  "@each"
  "@for"
  "from"
  "through"
  "in"
] @keyword.repeat

; Changed: upstream has `>=` and `<=` alone; the others compare in an
; `@if` too.
[
  ">="
  "<="
  "<"
  "=="
  "!="
] @operator

(mixin_statement
  name: (identifier) @function)

; Added: a variable declared, `$primary` in `$primary: #333;`, which the
; grammar reads as a property's declaration.
((property_name) @variable
 (#match? @variable "^\\$"))

; Changed: upstream names the parameter, and the variable inside it, the
; inner token, wins with its own name; here the variable.
(mixin_statement
  (parameters
    (parameter (variable) @variable.parameter)))

(function_statement
  name: (identifier) @function)

(function_statement
  (parameters
    (parameter (variable) @variable.parameter)))

(include_statement
  (identifier) @function)

; Added: a placeholder selector, `%button` in `@extend %button`, is a
; class to be extended, as `.button` is a type.
(placeholder) @type
(placeholder (identifier) @type)

; Changed: upstream names every plain value a string, `flex` in `display:
; flex`, and a media query's `screen` a function, so that the same rule
; looks one way in a .css file and another in a .scss one. Here they are
; as CSS's query has them. It also names a variable passed to a function a
; parameter; it is an argument, a variable like any other.

[
  "["
  "]"
] @punctuation.bracket
