; HTML's injections, after the query of tree-sitter-html 0.23.2 (MIT), with
; the change marked below: a script is JavaScript, a style CSS. A script
; of JSON, `<script type="application/ld+json">`, is JavaScript too: the
; pattern for a script without a type would take it as well, and a query
; cannot ask for the lack of an attribute.

((script_element
  (raw_text) @injection.content)
 (#set! injection.language "javascript"))

((style_element
  (raw_text) @injection.content)
 (#set! injection.language "css"))

; Added: an event handler's value is JavaScript too, `go()` in
; `onclick="go()"`. The name is captured for the predicate alone.
((attribute
  (attribute_name) @_name
  (quoted_attribute_value
    (attribute_value) @injection.content))
 (#match? @_name "^on[a-z]+$")
 (#set! injection.language "javascript"))
