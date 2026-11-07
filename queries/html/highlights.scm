; HTML's highlights, after the query of tree-sitter-html 0.23.2 (MIT), with
; the changes marked below. The last pattern that names a node wins, as in
; tree-sitter's own highlighter, so for any node the general patterns come
; first and the specific ones after them.

(tag_name) @tag
(erroneous_end_tag_name) @tag.error
(doctype) @constant
(attribute_name) @attribute
(attribute_value) @string
; Changed: upstream names the value inside the quotes alone, so that the
; quotes of `class="a"` are plain beside it; the quoted value is one
; string.
(quoted_attribute_value) @string
(comment) @comment
; Added: an entity is an escape, `&amp;` and `&#169;`, a character spelt
; another way.
(entity) @escape

[
  "<"
  ">"
  "</"
  "/>"
] @punctuation.bracket
