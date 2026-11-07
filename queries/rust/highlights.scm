; Rust's highlights, after the query of tree-sitter-rust 0.24.2 (MIT),
; with the changes marked below. The last pattern that names a node wins,
; as in tree-sitter's own highlighter, so for any node the general
; patterns come first and the specific ones after them.

; Identifiers

(type_identifier) @type
(primitive_type) @type.builtin
; Added: `!`, the type of what never returns, in `fn exit() -> !`.
(never_type) @type.builtin
(field_identifier) @property

; Function calls

(call_expression
  function: (identifier) @function)
(call_expression
  function: (field_expression
    field: (field_identifier) @function.method))
(call_expression
  function: (scoped_identifier
    "::"
    name: (identifier) @function))

(generic_function
  function: (identifier) @function)
(generic_function
  function: (scoped_identifier
    name: (identifier) @function))
(generic_function
  function: (field_expression
    field: (field_identifier) @function.method))

(macro_invocation
  macro: (identifier) @function.macro
  "!" @function.macro)

; Added: a macro called inside another's arguments, `matches!` in
; `assert!(matches!(x, Some(_)))`. The arguments are loose tokens to the
; grammar, and a name, a `!` and a bracketed group right after it are a
; call; `a != b` has `!=`, a token of its own.
(token_tree
  (identifier) @function.macro
  .
  "!" @function.macro
  .
  (token_tree))

; Added: a macro where `macro_rules!` defines it, as it is where it is
; called; its metavariables are its parameters, `$x` in `($x:expr)`, and
; their fragments are what kind of thing each takes.
(macro_definition
  name: (identifier) @function.macro)
(metavariable) @variable.parameter
(fragment_specifier) @type

; Function definitions

(function_item (identifier) @function)
(function_signature_item (identifier) @function)

; Identifier conventions
; Changed: upstream has these before the calls, so that `Some(x)` is a
; function, and the constructor after the constant, so that `MAX` is a
; constructor. Here a name's look has the last word over where it stands.

; Assume uppercase names are enum constructors
((identifier) @constructor
 (#match? @constructor "^[A-Z]"))

; Assume all-caps names are constants
; Fixed: upstream ends this pattern in `$'`, a quote after the end of the
; name, so it never matches and no constant is coloured. Keep the fix
; when taking a newer upstream query; `MAX_SIZE` in highlights.test fails
; without it. Changed: upstream wants two letters at least, which makes
; `const N` a constructor; one is enough here, as in Python's query.
((identifier) @constant
 (#match? @constant "^[A-Z][A-Z\\d_]*$"))

; Added: a name a path goes through is a module, `std` and `mem` in
; `std::mem::swap`, and the name a `mod` declares. Before the types, so
; that a capital still makes a type: `Foo` in `Foo::bar()`. The last name
; of a path is left alone: `fs` in `use std::fs` is a module, `read` in
; `use std::fs::read` a function, and nothing tells them apart.
(mod_item
  name: (identifier) @module)
(scoped_identifier
  path: (identifier) @module)
(scoped_identifier
  path: (scoped_identifier
    name: (identifier) @module))
(scoped_type_identifier
  path: (identifier) @module)
(scoped_type_identifier
  path: (scoped_identifier
    name: (identifier) @module))
(scoped_use_list
  path: (identifier) @module)
(scoped_use_list
  path: (scoped_identifier
    name: (identifier) @module))
(use_wildcard
  (identifier) @module)
(use_wildcard
  (scoped_identifier
    name: (identifier) @module))

; Added: in a path a primitive type is an identifier to the grammar, so
; it would be a module: `u8` in `u8::MAX`. Known by its text.
((scoped_identifier
  path: (identifier) @type.builtin)
 (#any-of? @type.builtin
   "bool" "char" "str"
   "i8" "i16" "i32" "i64" "i128" "isize"
   "u8" "u16" "u32" "u64" "u128" "usize"
   "f32" "f64"))

; Assume that uppercase names in paths are types
((scoped_identifier
  path: (identifier) @type)
 (#match? @type "^[A-Z]"))
((scoped_identifier
  path: (scoped_identifier
    name: (identifier) @type))
 (#match? @type "^[A-Z]"))
((scoped_type_identifier
  path: (identifier) @type)
 (#match? @type "^[A-Z]"))
((scoped_type_identifier
  path: (scoped_identifier
    name: (identifier) @type))
 (#match? @type "^[A-Z]"))

; Assume all qualified names in struct patterns are enum constructors. (They're
; either that, or struct names; highlighting both as constructors seems to be
; the less glaring choice of error, visually.)
(struct_pattern
  type: (scoped_type_identifier
    name: (type_identifier) @constructor))

; Added: in an attribute's arguments a name with a capital is a trait to
; derive, not a constructor: `Debug` in `#[derive(Debug)]`, and one level
; down in `#[cfg_attr(test, derive(Debug))]`.
((attribute
  arguments: (token_tree (identifier) @type))
 (#match? @type "^[A-Z]"))
((attribute
  arguments: (token_tree (token_tree (identifier) @type)))
 (#match? @type "^[A-Z]"))

(line_comment) @comment
(block_comment) @comment

(line_comment (doc_comment)) @comment.documentation
(block_comment (doc_comment)) @comment.documentation

"(" @punctuation.bracket
")" @punctuation.bracket
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket

(type_arguments
  "<" @punctuation.bracket
  ">" @punctuation.bracket)
(type_parameters
  "<" @punctuation.bracket
  ">" @punctuation.bracket)

"::" @punctuation.delimiter
":" @punctuation.delimiter
"." @punctuation.delimiter
"," @punctuation.delimiter
";" @punctuation.delimiter

(parameter (identifier) @variable.parameter)

; Changed: upstream names only the identifier, and the quote an operator,
; so that `'a` is two tokens; the quote is in no other place. And upstream
; calls a lifetime a label too, though it is a parameter, next to the
; types: `fn f<'a, T>(x: &'a T)`. A label names a loop to break out of.
(lifetime) @type.lifetime
(label) @label

"as" @keyword
"async" @keyword
"await" @keyword
"break" @keyword
"const" @keyword
"continue" @keyword
"default" @keyword
"dyn" @keyword
"else" @keyword
"enum" @keyword
"extern" @keyword
"fn" @keyword
"for" @keyword
"gen" @keyword
"if" @keyword
"impl" @keyword
"in" @keyword
"let" @keyword
"loop" @keyword
"macro_rules!" @keyword
"match" @keyword
"mod" @keyword
"move" @keyword
"pub" @keyword
"raw" @keyword
"ref" @keyword
"return" @keyword
"static" @keyword
"struct" @keyword
"trait" @keyword
"type" @keyword
"union" @keyword
"unsafe" @keyword
"use" @keyword
"where" @keyword
"while" @keyword
"yield" @keyword
(crate) @keyword
(mutable_specifier) @keyword
(super) @keyword

; Changed: upstream has the general `self` after the ones in paths, so
; that `self` in `use a::{self}` is a variable.
(self) @variable.builtin
(use_list (self) @keyword)
(scoped_use_list (self) @keyword)
(scoped_identifier (self) @keyword)

(char_literal) @string
(string_literal) @string
(raw_string_literal) @string

(boolean_literal) @constant.builtin
; Changed: upstream calls numbers `constant.builtin` too. Here they are
; numbers, as in the other languages, and `constant.builtin` is only what
; the language names itself: `true` here, `None` in Python.
(integer_literal) @number
(float_literal) @number

(escape_sequence) @escape

(attribute_item) @attribute
(inner_attribute_item) @attribute

"*" @operator
"&" @operator