; C++'s highlights, after the query of tree-sitter-cpp 0.23.4 (MIT), with
; the changes marked below. Read after C's, as upstream has it, so these
; patterns win over C's for the same node.

; Functions

(call_expression
  function: (qualified_identifier
    name: (identifier) @function))
; Added: a longer path nests a path in its name, and a query cannot say
; "at any depth", so each depth is spelt out, up to `a::b::c::d::f()`.
; Upstream stops at one, so that `now` in `std::chrono::now()` is plain.
(call_expression
  function: (qualified_identifier
    name: (qualified_identifier
      name: (identifier) @function)))
(call_expression
  function: (qualified_identifier
    name: (qualified_identifier
      name: (qualified_identifier
        name: (identifier) @function))))
(call_expression
  function: (qualified_identifier
    name: (qualified_identifier
      name: (qualified_identifier
        name: (qualified_identifier
          name: (identifier) @function)))))

; Changed: upstream has this pattern twice.
(template_function
  name: (identifier) @function)

; Added: the four casts are keywords, which the grammar reads as a call
; of a template function; known by their text.
((template_function
  name: (identifier) @keyword)
 (#any-of? @keyword
   "static_cast" "dynamic_cast" "reinterpret_cast" "const_cast"))

(template_method
  name: (field_identifier) @function)

(function_declarator
  declarator: (qualified_identifier
    name: (identifier) @function))
; Added: the same depths for a definition, `void app::Greeter::greet()`.
(function_declarator
  declarator: (qualified_identifier
    name: (qualified_identifier
      name: (identifier) @function)))
(function_declarator
  declarator: (qualified_identifier
    name: (qualified_identifier
      name: (qualified_identifier
        name: (identifier) @function))))
(function_declarator
  declarator: (qualified_identifier
    name: (qualified_identifier
      name: (qualified_identifier
        name: (qualified_identifier
          name: (identifier) @function)))))

(function_declarator
  declarator: (field_identifier) @function)

; Added: a destructor's name with its `~`, wherever it stands: declared,
; defined as `Greeter::~Greeter()`, or called. The name in it is an
; identifier, a variable by C's patterns, and the inner token wins.
(destructor_name) @function
(destructor_name
  (identifier) @function)

; Types

; Added: a namespace is a module, `std` in `std::vector`. Before the
; capital, so that a capital still makes a type, as in Rust's query.
(namespace_identifier) @module
; Added: the namespace `using namespace` brings in, which the grammar
; names a plain identifier: `std` in `using namespace std;`. Its last name
; only after `namespace`; in `using std::cout;` it can be anything. A
; path nests as in a call, spelt out up to three names.
(using_declaration
  "namespace"
  (identifier) @module)
(using_declaration
  "namespace"
  (qualified_identifier
    name: (identifier) @module))
(using_declaration
  "namespace"
  (qualified_identifier
    name: (qualified_identifier
      name: (identifier) @module)))

((namespace_identifier) @type
 (#match? @type "^[A-Z]"))

(auto) @type

; Added: a concept's name where it is defined, a type as it is where it
; constrains one: `Small` in `concept Small = ...` and `template <Small T>`.
(concept_definition
  name: (identifier) @type)

; Constants

(this) @variable.builtin
; Changed: upstream calls `nullptr` a plain constant, over C's
; `constant.builtin` for `NULL`; it is the language's own, as `NULL` is.

; Keywords

[
 "catch"
 "class"
 "co_await"
 "co_return"
 "co_yield"
 "constexpr"
 "constinit"
 "consteval"
 "delete"
 "explicit"
 "final"
 "friend"
 "mutable"
 "namespace"
 "noexcept"
 "new"
 "override"
 "private"
 "protected"
 "public"
 "template"
 "throw"
 "try"
 "typename"
 "using"
 "concept"
 "requires"
 "virtual"
 ; Added: the rest of the words the grammar has, the operators spelt as
 ; words among them.
 "decltype"
 "operator"
 "static_assert"
 "and"
 "and_eq"
 "bitand"
 "bitor"
 "compl"
 "not"
 "not_eq"
 "or"
 "or_eq"
 "xor"
 "xor_eq"
] @keyword

; Strings

(raw_string_literal) @string
