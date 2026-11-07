; JSX's highlights, after the query of tree-sitter-javascript 0.25.0
; (MIT). Read after `highlights.scm`, by JavaScript and by TSX, whose
; grammars have JSX. A tag spelt in small letters is HTML's, `<div>`; one
; with a capital, `<Button>`, is a component, a class or a function, and
; is named as one by the pattern for a capital in `highlights.scm`.

(jsx_opening_element (identifier) @tag (#match? @tag "^[a-z][^.]*$"))
(jsx_closing_element (identifier) @tag (#match? @tag "^[a-z][^.]*$"))
(jsx_self_closing_element (identifier) @tag (#match? @tag "^[a-z][^.]*$"))

(jsx_attribute (property_identifier) @attribute)
(jsx_opening_element (["<" ">"]) @punctuation.bracket)
(jsx_closing_element (["</" ">"]) @punctuation.bracket)
(jsx_self_closing_element (["<" "/>"]) @punctuation.bracket)
