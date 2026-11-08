; Helm's injections, after Helix's: the text around the code is YAML, all
; of a file's pieces of it one document. Read alone, a piece after
; `image: "{{ .Values.tag }}` would start inside a quoted string, and the
; keys after it would be lost. A `-` alone between two pieces of code,
; `{{ .a }}-{{ .b }}`, is a node of its own, and not YAML's: to YAML a
; line's `-` starts an item of a list.
((text) @injection.content
 (#set! injection.language "yaml")
 (#set! injection.combined))
