; INI's highlights, after the query of tree-sitter-ini 1.4.0 (MIT), with
; the changes marked below. INI has no types, and the grammar reads a
; value as one piece, to the end of its line: a `#` after a value is the
; value's, though git reads a comment there.
;
; What the grammar does not parse, and no pattern here can colour: a key
; with brackets, `Name[ru]=…` in a desktop entry, which spoils the line
; after it too; a key with no value, `skip-name-resolve` in my.cnf, which
; spoils the next key; a value that goes on over lines, indented as
; Python's configparser has it or after a `\` as systemd's; `:` for `=`.

(section_name
  (text) @type) ; consistency with toml

; Changed: upstream names a comment a word to spell-check too, two names
; for one node; here the last name stands, and it would colour nothing.
(comment) @comment

[
  "["
  "]"
] @punctuation.bracket

"=" @operator

(setting
  (setting_name) @property)

; Added: a value is a string, which INI's are, so that it stands apart
; from its key as a plain value does in YAML. Upstream leaves it plain.
(setting
  (setting_value) @string)
