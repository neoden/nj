; PHP's injections, after the queries of tree-sitter-php 0.25.1 (MIT), its
; `injections.scm` and `injections-text.scm` in one. Upstream's injection
; of a comment into PHPDoc, a grammar this build has not, is left out.

; The text outside `<?php ... ?>` is HTML, all of a file's pieces of it
; one document: in `<a href="<?= $url ?>" class="x">` the tag goes on past
; the PHP, which a piece read alone would end it at.
((text) @injection.content
 (#set! injection.language "html")
 (#set! injection.combined))

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
