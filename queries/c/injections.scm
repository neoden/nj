; C's injections, as Zed has them; tree-sitter-c has none of its own. The body of a `#define` is C the grammar keeps as
; raw text; the text after `#pragma` or `#error` is not C, and is left so.

(preproc_def
  value: (preproc_arg) @injection.content
  (#set! injection.language "c"))

(preproc_function_def
  value: (preproc_arg) @injection.content
  (#set! injection.language "c"))
