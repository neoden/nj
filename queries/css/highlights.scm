; CSS's highlights, after the query of tree-sitter-css 0.25.0 (MIT), with
; the changes marked below. SCSS reads this query too, before its own;
; what only CSS's grammar has is in `css-only.scm`. The last pattern that names a node wins, as in
; tree-sitter's own highlighter, so for any node the general patterns come
; first and the specific ones after them.

(comment) @comment
; Added: a `//` comment, which the grammar takes for PostCSS's sake.
(js_comment) @comment

(tag_name) @tag
(nesting_selector) @tag
(universal_selector) @tag

"~" @operator
">" @operator
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"=" @operator
"^=" @operator
"|=" @operator
"~=" @operator
"$=" @operator
"*=" @operator

"and" @operator
"or" @operator
"not" @operator
"only" @operator
; Added: `selector` in `@supports selector(...)`.
"selector" @operator

(attribute_selector (plain_value) @string)

; Changed: upstream names a class and an id properties, as it does a
; property's name, so that a selector and a declaration read alike. A
; class names a kind of element, as a type does, and an id names one, as
; a constant does.
(class_name) @type
(id_name) @constant
(namespace_name) @property
(property_name) @property
(feature_name) @property

; Changed: upstream has these before the property's name, which then wins
; over them, so that `--gap` is a property like `gap`. A custom property
; is a variable, declared and used.
((property_name) @variable
 (#match? @variable "^--"))
((plain_value) @variable
 (#match? @variable "^--"))

; Changed: upstream names every tag in a pseudo-element an attribute, the
; element before `::` too, so that `p` in `p::first-line` is not a tag.
; The pseudo-element's own name is the one right after `::`.
(pseudo_element_selector "::" . (tag_name) @attribute)
(pseudo_class_selector (class_name) @attribute)
(attribute_name) @attribute

(function_name) @function

"@media" @keyword
"@import" @keyword
"@charset" @keyword
"@namespace" @keyword
"@supports" @keyword
"@keyframes" @keyword
(at_keyword) @keyword
(to) @keyword
(from) @keyword
(important) @keyword

(string_value) @string
(color_value) @string.special

(integer_value) @number
(float_value) @number
(unit) @type

[
  "#"
  ","
  "."
  ":"
  "::"
  ";"
] @punctuation.delimiter

[
  "{"
  ")"
  "("
  "}"
] @punctuation.bracket

; Added: the signs that belong to what they stand in, named after the
; general patterns above, which name the same tokens: the `*` of `#main *`
; is the selector's, not an operator, and the `#` of `#fff` the colour's.
(universal_selector "*" @tag)
(color_value "#" @string.special)
