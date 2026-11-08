; C's highlights, after the query of tree-sitter-c 0.24.2 (MIT), with the
; changes marked below. The last pattern that names a node wins, as in
; tree-sitter's own highlighter, so for any node the general patterns
; come first and the specific ones after them.

(identifier) @variable

((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

; Added: an enum's member where it is declared, whatever its case: `Green`
; in `enum color { RED, Green }`. Used later it is a name like any other.
(enumerator
  name: (identifier) @constant)

"break" @keyword
"case" @keyword
"const" @keyword
"continue" @keyword
"default" @keyword
"do" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"for" @keyword
"if" @keyword
"inline" @keyword
"return" @keyword
"sizeof" @keyword
"static" @keyword
"struct" @keyword
"switch" @keyword
"typedef" @keyword
"union" @keyword
"volatile" @keyword
"while" @keyword
; Added: the rest of the standard's keywords the grammar has; upstream
; leaves `goto` and `restrict` plain. Not `auto`: C++ reads this query
; too, and its grammar has `auto` as a node of its own, not this token.
"_Alignas" @keyword
"_Alignof" @keyword
"_Atomic" @keyword
"_Generic" @keyword
"_Noreturn" @keyword
"alignas" @keyword
"alignof" @keyword
"asm" @keyword
"constexpr" @keyword
"goto" @keyword
"noreturn" @keyword
"offsetof" @keyword
"register" @keyword
"restrict" @keyword
"thread_local" @keyword

"#define" @keyword
"#elif" @keyword
; Added: C23's two, next to `#elif`.
"#elifdef" @keyword
"#elifndef" @keyword
"#else" @keyword
"#endif" @keyword
"#if" @keyword
"#ifdef" @keyword
"#ifndef" @keyword
"#include" @keyword
(preproc_directive) @keyword
; Added: `defined` in `#if defined(X)`.
"defined" @keyword

"--" @operator
"-" @operator
"-=" @operator
"->" @operator
"=" @operator
"!=" @operator
"*" @operator
"&" @operator
"&&" @operator
"+" @operator
"++" @operator
"+=" @operator
"<" @operator
"==" @operator
">" @operator
"||" @operator

"." @delimiter
";" @delimiter

(string_literal) @string
(system_lib_string) @string
; Changed: upstream calls a character a number; it is a string, as in
; the other languages.
(char_literal) @string
; Added: an escape is a token of its own inside a string or a character.
(escape_sequence) @escape

; Changed: upstream calls `NULL` a constant; it is the language's own, as
; `nil` is Go's. Added: `true` and `false`, which upstream leaves plain.
[
  (null)
  (true)
  (false)
] @constant.builtin
(number_literal) @number

(field_identifier) @property
(statement_identifier) @label
(type_identifier) @type
; Changed: upstream calls these plain types; they are the language's own,
; as `u8` is Rust's.
(primitive_type) @type.builtin
(sized_type_specifier) @type.builtin

; Changed: a call is `function.call`, told from where a function is
; declared, as Neovim's queries have it; a macro used reads as a call.
(call_expression
  function: (identifier) @function.call)
(call_expression
  function: (field_expression
    field: (field_identifier) @function.call))
(function_declarator
  declarator: (identifier) @function)
; Changed: upstream calls this `function.special`; a macro, as in Rust's
; query.
(preproc_function_def
  name: (identifier) @function.macro)

; Added: attributes, which upstream leaves plain, read as Rust's are: the
; word that opens one a keyword, the names in it attributes. After the
; calls, so that `aligned` in `__attribute__((aligned(8)))` is not a
; function. Their arguments keep their own colours.
"__attribute__" @keyword
"__attribute" @keyword
"__declspec" @keyword
(attribute_specifier
  (argument_list (identifier) @attribute))
(attribute_specifier
  (argument_list
    (call_expression
      function: (identifier) @attribute)))
(attribute
  prefix: (identifier) @attribute)
(attribute
  name: (identifier) @attribute)
(ms_declspec_modifier
  (identifier) @attribute)

(comment) @comment
