; A rebase's injections: what `exec` runs is Shell, `cargo test` in `exec
; cargo test`. Upstream has none; the command is captured for the
; predicate alone.

((operation
  (command) @_command
  (message) @injection.content)
 (#match? @_command "^(x|exec)$")
 (#set! injection.language "Shell"))
