; A rebase's list of commands, `git-rebase-todo`, after the query of
; tree-sitter-git-rebase (MIT), from its repository at the commit
; `Cargo.toml` names. Upstream's own notes on what each name stands for:
; * constant.builtin - git hash
; * constant - a git label
; * keyword - command that acts on commits commits
; * function - command that acts only on labels
; * comment - discarded commentary on a command, has no effect on the rebase
; * string - text used in the rebase operation
; * operator - a 'switch' (used in fixup and merge), either -c or -C at time of writing

(((command) @keyword
  (label) @constant.builtin
  (message)? @comment)
 (#match? @keyword "^(p|pick|r|reword|e|edit|s|squash|d|drop)$"))

(((command) @function
  (label) @constant
  (message)? @comment)
 (#match? @function "^(l|label|t|reset)$"))

((command) @keyword
 (#match? @keyword "^(x|exec|b|break)$"))

(((command) @attribute
  (label) @constant.builtin
  (message)? @comment)
 (#match? @attribute "^(f|fixup)$"))

; Added: with `-C` or `-c` the grammar reads a fixup as a merge, the
; first word of the message a second label: `Tidy` in `fixup -C a1b2c3d
; Tidy up`. Git writes such lines itself for an `amend!` commit. The
; message is the message, as it is without the option.
(((command) @attribute
  (option)
  (label) @constant.builtin
  (label) @comment
  (message)? @comment)
 (#match? @attribute "^(f|fixup)$"))

(((command) @keyword
  (label) @constant.builtin
  (label) @constant
  (message) @string)
 (#match? @keyword "^(m|merge)$"))

(option) @operator

(comment) @comment
