; Markdown's injections, after the query of tree-sitter-md 0.5.4 (MIT).
; The HTML and the front matter are not read yet.

(fenced_code_block
  (info_string
    (language) @injection.language)
  (code_fence_content) @injection.content)

((inline) @injection.content
 (#set! injection.language "markdown_inline"))
