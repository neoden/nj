; PHP's injections, after the queries of tree-sitter-php 0.25.1 (MIT), its
; `injections.scm` and `injections-text.scm` in one. Upstream's injection
; of a comment into PHPDoc, a grammar this build has not, is left out.

; The text outside `<?php ... ?>` is HTML. Upstream reads all of a file's
; pieces of it as one document; here each piece is read alone, so an
; element that a PHP tag cuts in two is coloured in its two halves.
((text) @injection.content
 (#set! injection.language "html"))

; A heredoc's marker names its language: `<<<SQL ... SQL`. Changed: its
; body is made of pieces of string, children a piece leaves out unless
; told otherwise, which leaves nothing to read; here they are read.
((heredoc
  (heredoc_body) @injection.content
  (heredoc_end) @injection.language)
 (#set! injection.include-children))

((nowdoc
  (nowdoc_body) @injection.content
  (heredoc_end) @injection.language)
 (#set! injection.include-children))
