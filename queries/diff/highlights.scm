; A diff's highlights, after the query of tree-sitter-diff 0.2.0 (MIT),
; from its repository at the commit `Cargo.toml` names, with the changes
; marked below. A line is coloured whole, as `git diff` colours it: the
; code in it is not read as its language.

; A line added green and one taken away red, the names of the two files,
; `--- a/x` and `+++ b/x`, with them.
[
  (addition)
  (new_file)
] @diff.plus

[
  (deletion)
  (old_file)
] @diff.minus

; Changed: upstream names a file's name a path and the signs that start a
; line punctuation, inner tokens that win over the line's colour: the `-`
; of a line taken away and the name in `--- a/x` would not be red.

; Changed: upstream names `diff` a function and its flag a parameter; the
; line that starts a file's diff, `diff --git a/x b/x`, is a keyword
; whole, as the line that heads a block.
(command) @keyword

(commit) @constant

(location) @attribute

(index
  "index" @keyword)

(mode) @number

[
  (binary_change)
  (similarity)
  (dissimilarity)
  (file_change)
] @label

(similarity
  (score) @number
  "%" @number)

(dissimilarity
  (score) @number
  "%" @number)

(binary_patch
  [
    "GIT"
    "binary"
    "patch"
  ] @label)

(binary_hunk
  [
    "literal"
    "delta"
  ] @keyword
  (size) @number)

forward: (binary_hunk
  (payload) @diff.plus)

reverse: (binary_hunk
  (payload) @diff.minus)

; Changed: upstream names a `#` line a comment and a word to spell-check,
; two names for one node; here the comment alone. Changed too: upstream
; names `\ No newline at end of file` a special string; it is a note of
; git's about the line before it, as a comment is.
[
  (comment)
  (special)
] @comment
