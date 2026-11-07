; Makefile's highlights, after the query of tree-sitter-make 1.1.1 (MIT),
; with the changes marked below. Upstream's is written for an older
; Neovim: names such as `@conditional` and `@include`, which are plain
; text here, and extra captures that only feed a predicate, which here
; would be tokens of their own. This one keeps upstream's choices where
; they hold. The last pattern that names a node wins, as in tree-sitter's
; own highlighter, so for any node the general patterns come first and
; the specific ones after them.

(comment) @comment

[
  (text)
  (string)
  (raw_text)
] @string

(escape) @escape

; A target is what `make` is asked to build, as a function is called.
; Changed: upstream names only the standard ones, `all` and `clean`, and
; them constants; any target is a target.
(targets (word) @function)

; Changed: upstream names these constants. They are make's own words,
; `.PHONY` and its kind, as a directive is.
((targets (word) @keyword)
 (#match? @keyword "^\\.(PHONY|SUFFIXES|DEFAULT|PRECIOUS|INTERMEDIATE|SECONDARY|SECONDEXPANSION|DELETE_ON_ERROR|IGNORE|LOW_RESOLUTION_TIME|SILENT|EXPORT_ALL_VARIABLES|NOTPARALLEL|ONESHELL|POSIX)$"))

; A word with `%`, `*` or `?` is a pattern, a target's or a file's:
; `%.o: %.c`.
((word) @string.special
 (#match? @string.special "[%*?]"))

; A variable, assigned or defined, and referred to. Changed: upstream
; names the word inside a reference, and its `$(` and `)` punctuation;
; here `$(CC)` is one token.
(variable_assignment
  name: (word) @constant)
; Added: a variable given a command's output, `NOW != date`.
(shell_assignment
  name: (word) @constant)
(define_directive
  name: (word) @constant)
(variable_reference) @constant
(variable_reference
  (word) @constant)

; The variables make sets itself and the ones its implicit rules read.
((variable_assignment
  name: (word) @constant.builtin)
 (#match? @constant.builtin "^(AR|AS|CC|CXX|CPP|FC|M2C|PC|CO|GET|LEX|YACC|LINT|MAKEINFO|TEX|TEXI2DVI|WEAVE|CWEAVE|TANGLE|CTANGLE|RM|ARFLAGS|ASFLAGS|CFLAGS|CXXFLAGS|COFLAGS|CPPFLAGS|FFLAGS|GFLAGS|LDFLAGS|LDLIBS|LFLAGS|YFLAGS|PFLAGS|RFLAGS|LINTFLAGS|MAKE|MAKEFLAGS|SHELL|MAKEFILE_LIST|MAKE_RESTARTS|MAKE_TERMOUT|MAKE_TERMERR|\\.DEFAULT_GOAL|\\.RECIPEPREFIX|\\.EXTRA_PREREQS|\\.VARIABLES|\\.FEATURES|\\.INCLUDE_DIRS|\\.LOADED)$"))
; A reference to one of them, its `$(` and `)` with it and the name
; inside, which is a token of its own.
((variable_reference) @constant.builtin
 (#match? @constant.builtin "^\\$[({](AR|AS|CC|CXX|CPP|FC|M2C|PC|CO|GET|LEX|YACC|LINT|MAKEINFO|TEX|TEXI2DVI|WEAVE|CWEAVE|TANGLE|CTANGLE|RM|ARFLAGS|ASFLAGS|CFLAGS|CXXFLAGS|COFLAGS|CPPFLAGS|FFLAGS|GFLAGS|LDFLAGS|LDLIBS|LFLAGS|YFLAGS|PFLAGS|RFLAGS|LINTFLAGS|MAKE|MAKEFLAGS|SHELL|MAKEFILE_LIST|MAKE_RESTARTS|MAKE_TERMOUT|MAKE_TERMERR|\\.DEFAULT_GOAL|\\.RECIPEPREFIX|\\.EXTRA_PREREQS|\\.VARIABLES|\\.FEATURES|\\.INCLUDE_DIRS|\\.LOADED)[)}]$"))
((variable_reference
  (word) @constant.builtin)
 (#match? @constant.builtin "^(AR|AS|CC|CXX|CPP|FC|M2C|PC|CO|GET|LEX|YACC|LINT|MAKEINFO|TEX|TEXI2DVI|WEAVE|CWEAVE|TANGLE|CTANGLE|RM|ARFLAGS|ASFLAGS|CFLAGS|CXXFLAGS|COFLAGS|CPPFLAGS|FFLAGS|GFLAGS|LDFLAGS|LDLIBS|LFLAGS|YFLAGS|PFLAGS|RFLAGS|LINTFLAGS|MAKE|MAKEFLAGS|SHELL|MAKEFILE_LIST|MAKE_RESTARTS|MAKE_TERMOUT|MAKE_TERMERR|\\.DEFAULT_GOAL|\\.RECIPEPREFIX|\\.EXTRA_PREREQS|\\.VARIABLES|\\.FEATURES|\\.INCLUDE_DIRS|\\.LOADED)$"))
[
  "VPATH"
  ".RECIPEPREFIX"
] @constant.builtin

; Changed: upstream names the `$` and the sign apart, punctuation; here
; `$@`, `$<`, `$^` are one token each, the variables make sets for a
; rule.
(automatic_variable) @variable.builtin

; A call of one of make's functions, `$(wildcard *.c)`: the call is the
; function's, its arguments their own.
(function_call) @function.builtin
(shell_function) @function.builtin

; Changed: upstream names these `@conditional`, `@repeat`, `@include` and
; `@exception`, old names that are plain text here.
[
  "ifeq"
  "ifneq"
  "ifdef"
  "ifndef"
  "else"
  "endif"
  "define"
  "endef"
  "vpath"
  "undefine"
  "export"
  "unexport"
  "override"
  "private"
  "include"
  "sinclude"
  "-include"
] @keyword

[
  "="
  ":="
  "::="
  "?="
  "+="
  "!="
] @operator
