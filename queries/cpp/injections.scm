; C++'s injections: a macro's body as C's are, but read as C++, and
; tree-sitter-cpp 0.23.4's own (MIT): a raw string's delimiter names its
; language, `R"json(...)json"`.

(preproc_def
  value: (preproc_arg) @injection.content
  (#set! injection.language "c++"))

(preproc_function_def
  value: (preproc_arg) @injection.content
  (#set! injection.language "c++"))

(raw_string_literal
  delimiter: (raw_string_delimiter) @injection.language
  (raw_string_content) @injection.content)
