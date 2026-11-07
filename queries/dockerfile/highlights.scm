; A Dockerfile's highlights, after the query of tree-sitter-containerfile
; 0.9.2 (MIT), with the changes marked below.

[
  "FROM"
  "AS"
  "RUN"
  "CMD"
  "LABEL"
  "EXPOSE"
  "ENV"
  "ADD"
  "COPY"
  "ENTRYPOINT"
  "VOLUME"
  "USER"
  "WORKDIR"
  "ARG"
  "ONBUILD"
  "STOPSIGNAL"
  "HEALTHCHECK"
  "SHELL"
  "MAINTAINER"
  "CROSS_BUILD"
] @keyword

[
  ":"
  "@"
] @operator

; Changed: upstream names a comment a word to spell-check too, two names
; for one node; here the last name stands, and it would colour nothing.
(comment) @comment

(image_spec
  (image_tag
    ":" @punctuation.special)
  (image_digest
    "@" @punctuation.special))

[
  (double_quoted_string)
  (single_quoted_string)
  (json_string)
] @string

(heredoc_block) @string

[
  (heredoc_marker)
  (heredoc_end)
] @label

; Changed: upstream names this `string.escape`, which is a string here,
; the longest start of the name known.
(escape_sequence) @escape

; Changed: upstream names the `$`, `{` and `}` of `${VERSION}` punctuation
; and the name inside a constant only in capitals. A variable a build
; reads, an `ARG` or an `ENV`, is one token whatever its case, as `$(CC)`
; is in a Makefile; its operator, `:-` in `${A:-b}`, is the operator's.
(expansion) @constant
(expansion_operator) @operator

(arg_pair
  name: (unquoted_string) @property)

(env_pair
  name: (unquoted_string) @property)

(label_pair
  key: (_) @property)

(param
  name: (_) @property)

(mount_param
  name: (_) @property)

(mount_param_param) @property

(expose_port) @number
