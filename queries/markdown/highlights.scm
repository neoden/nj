; Markdown's highlights, after the block query of tree-sitter-md 0.5.4
; (MIT), which comes from nvim-treesitter. The last pattern that names a
; node wins, as in tree-sitter's own highlighter, so for any node the
; general patterns come first and the specific ones after them.
;
; Changed throughout: upstream uses nvim-treesitter's old names,
; `text.title`, `text.literal`, `text.uri`, which no other query here
; does; these are the `markup` names. The text inside a block, with its
; emphasis, code spans and links, is a second grammar's and not read yet.

; Headings, the marker and the text

(atx_heading (inline) @markup.heading)
(setext_heading (paragraph) @markup.heading)

[
  (atx_h1_marker)
  (atx_h2_marker)
  (atx_h3_marker)
  (atx_h4_marker)
  (atx_h5_marker)
  (atx_h6_marker)
  (setext_h1_underline)
  (setext_h2_underline)
] @markup.heading.marker

; Code blocks, whole, the fences and the language's name with them.
; Changed: upstream names the fences punctuation.

[
  (indented_code_block)
  (fenced_code_block)
] @markup.raw.block

; The code of a block that names its language is that language's to
; colour, as in a file of its own: `none` has no ink, so that what the
; language leaves unnamed is plain, not the block's. A language this
; build has no grammar for leaves the code plain. Changed: upstream does
; this for every fenced block, so that one naming no language is plain
; too; here that one stays the block's, like an indented one.
(fenced_code_block
  (info_string)
  (code_fence_content) @none)

; Lists, quotes, the line between sections

[
  (list_marker_plus)
  (list_marker_minus)
  (list_marker_star)
  (list_marker_dot)
  (list_marker_parenthesis)
] @markup.list.marker

(block_quote_marker) @markup.quote.marker

; Changed: upstream names every continuation a quote's marker, but a
; continuation is whatever starts a line inside a container: a list
; item's indent too, `  ` before `two` in `- one\n  two`. The grammar
; puts it deep inside the block, not under the quote, so it is known by
; its text: one with a `>` carries a quote's marker.
((block_continuation) @markup.quote.marker
 (#match? @markup.quote.marker ">"))

(thematic_break) @punctuation.special

; Link reference definitions: `[label]: url "title"`

(link_label) @markup.link.label
(link_destination) @markup.link.url
(link_title) @string

(backslash_escape) @escape
