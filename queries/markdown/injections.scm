; Markdown's injections, after the query of tree-sitter-md 0.5.4 (MIT).
; The HTML is not read yet.

(fenced_code_block
  (info_string
    (language) @injection.language)
  (code_fence_content) @injection.content)

; The front matter at the top of a document, with its fences: YAML's
; `---` starts and ends a document to YAML too; TOML's `+++` is no TOML,
; and the grammar steps over it.
((minus_metadata) @injection.content
  (#set! injection.language "yaml"))

((plus_metadata) @injection.content
  (#set! injection.language "toml"))

((inline) @injection.content
 (#set! injection.language "markdown_inline"))
