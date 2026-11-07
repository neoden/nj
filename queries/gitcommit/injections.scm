; A commit message's injections, after the query of tree-sitter-gitcommit
; 0.5.0 (WTFPL): the diff `git commit -v` writes below the scissors, and
; the commands a rebase lists in its comments, `pick a1b2c3 …`.

((diff) @injection.content
  (#set! injection.language "Diff"))

((rebase_command) @injection.content
  (#set! injection.language "Git rebase"))
