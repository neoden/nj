; Shell's highlights, after the query of tree-sitter-bash 0.25.1 (MIT),
; with the changes marked below. The last pattern that names a node wins,
; as in tree-sitter's own highlighter, so for any node the general
; patterns come first and the specific ones after them.

; Changed: upstream calls a variable a property; it is a variable, as in
; the other languages.
(variable_name) @variable

; Added: a variable expanded is a variable even inside a string, so that
; `$name` in `"hello $name"` stands out of it, as `{x}` of a Python
; f-string does. What the expansion holds keeps its own colours.
(simple_expansion) @variable
(expansion) @variable

; Added: the variables the shell sets, `$?`, `$#`, `$@`, `$1`, with their
; `$`.
(special_variable_name) @variable.builtin
(simple_expansion
  (special_variable_name)) @variable.builtin
; The arguments, `$1` and `$0`, are plain names to the grammar, named by
; digits; known by them, the `$` and the digits each.
((simple_expansion) @variable.builtin
 (#match? @variable.builtin "^\\$[0-9]+$"))
((variable_name) @variable.builtin
 (#match? @variable.builtin "^[0-9]+$"))

[
  (string)
  (raw_string)
  (heredoc_body)
  (heredoc_start)
  ; Added: the other strings, `$'a\n'` and `$"..."`, the word that ends a
  ; heredoc as the one that starts it, and a regex after `=~`.
  (ansi_c_string)
  (translated_string)
  (heredoc_end)
  (regex)
] @string

; Changed: upstream has this before the strings and the expansions; a
; command inside `"$(date)"` is a command still. A command run is a
; call, `function.call`, told from a function the script defines, as
; Neovim's queries have it.
(command_name) @function.call

; Added: the commands the shell itself carries out, `cd` and `echo`, as
; `len` is Python's; and the four that leave a loop, a function or the
; shell, which read as `return` does in other languages.
((command_name (word) @function.builtin)
 (#any-of? @function.builtin
   "." ":" "alias" "bg" "bind" "builtin" "caller" "cd" "command" "compgen"
   "complete" "compopt" "dirs" "disown" "echo" "enable" "eval" "exec" "fc"
   "fg" "getopts" "hash" "help" "history" "jobs" "kill" "let" "logout"
   "mapfile" "popd" "printf" "pushd" "pwd" "read" "readarray" "set"
   "shift" "shopt" "source" "suspend" "test" "times" "trap" "true" "false"
   "type" "ulimit" "umask" "unalias" "wait"))
((command_name (word) @keyword)
 (#any-of? @keyword "break" "continue" "exit" "return"))

[
  "case"
  "do"
  "done"
  "elif"
  "else"
  "esac"
  "export"
  "fi"
  "for"
  "function"
  "if"
  "in"
  "select"
  "then"
  "unset"
  "until"
  "while"
  ; Added: the other words that declare, `local x=1`.
  "declare"
  "local"
  "readonly"
  "typeset"
  "unsetenv"
] @keyword

(comment) @comment

(function_definition name: (word) @function)

(file_descriptor) @number
; Added: a number, in `$((n + 1))` or `${a[0]}`.
(number) @number

; Changed: upstream names `$` an operator too, and the inner token wins,
; so that the `$` of `$HOME` is not the variable's and that of `"$x"` not
; the string's.
[
  "&&"
  ">"
  ">>"
  "<"
  "|"
] @operator

; A flag of a command, `-la` in `ls -la`.
(
  (command (_) @constant)
  (#match? @constant "^-")
)
; Added: the flags of `declare` and `unset`, which the grammar does not
; count as commands: `-a` in `declare -a list`.
((declaration_command (word) @constant)
 (#match? @constant "^-"))
((unset_command (word) @constant)
 (#match? @constant "^-"))
; Added: a test's flag, `-f` in `[ -f x ]`, as it is in `test -f x`.
((test_operator) @constant
 (#match? @constant "^-"))
