; A commit message's highlights, after the query of tree-sitter-gitcommit
; 0.5.0 (WTFPL), with the changes marked below.
;
; Changed throughout: upstream names a node twice, `@markup.heading
; @spell`, for a spell checker; here the last name a node has is the one
; that stands, so `spell` would win and colour nothing. Those names are
; left out.

; The text git writes in the comments, and the comments themselves.
(comment) @comment
(generated_comment) @comment
(scissor) @comment

; The first line, the subject, as Vim colours it.
(subject) @markup.heading

; A Conventional Commits prefix, `fix(parser)!:`: the type a keyword, the
; scope a parameter.
(subject
  (subject_prefix) @function)
(prefix
  (type) @keyword)
(prefix
  (scope) @variable.parameter)
(prefix
  [
    "("
    ")"
    ":"
  ] @punctuation.delimiter)
(prefix
  "!" @punctuation.special)

; The headings of git's comments, `Changes to be committed:`.
(title) @markup.heading

; Changed: upstream names the branch a link, which is plain text here;
; it is a name to see in the grey of the comments, as a constant is.
(branch) @constant

; A file's change. Changed: upstream names every kind a keyword; here a
; file added is green and one deleted red, as in a diff.
(change) @keyword
(change
  (new) @diff.plus)
(change
  (deleted) @diff.minus)

; Changed: upstream names the path a URL; it is a path, a string.
(filepath) @string.special.path
(arrow) @punctuation.delimiter

; A trailer, `Signed-off-by: …`: its token a label, its value text.
(trailer
  (token) @label)

; Changed: upstream names `BREAKING CHANGE:` an error in a comment, which
; is a comment's grey here; it is a keyword of the message.
(breaking_change
  (token) @keyword)
