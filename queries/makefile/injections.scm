; Makefile's injections: the shell's text is Shell. A recipe's line, the
; command of `$(shell ...)` and of `!=`. What make expands in it, `$(CC)`
; and `$@`, are its children, left out of the piece and make's own.

((shell_text) @injection.content
  (#set! injection.language "Shell"))

((shell_command) @injection.content
  (#set! injection.language "Shell"))
