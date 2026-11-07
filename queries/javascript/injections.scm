; JavaScript's injections, after the query of tree-sitter-javascript
; 0.25.0 (MIT): a template's tag names the language inside it, `css` in
; css`a { color: red }`. Upstream's others are for grammars this build
; does not have, regexes, JSDoc and Glimmer, and are left out.

(call_expression
  function: [
    (identifier) @injection.language
    (member_expression
      property: (property_identifier) @injection.language)
  ]
  arguments: (template_string (string_fragment) @injection.content)
  (#set! injection.include-children))
