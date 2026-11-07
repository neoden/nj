; The patterns of CSS's highlights that SCSS cannot share: its grammar,
; tree-sitter-scss 1.0.0, is older than tree-sitter-css 0.25.0 and has
; none of these nodes. Read after `highlights.scm`, by CSS alone.

; Added: `of` in `:nth-child(2 of .x)`.
"of" @operator

; Added: the one at-rule upstream's list leaves out.
"@scope" @keyword

; Added: an escape is a token of its own inside a string.
(escape_sequence) @escape
