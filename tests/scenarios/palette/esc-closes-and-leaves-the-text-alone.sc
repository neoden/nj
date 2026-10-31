# While the palette is open the keyboard is its: what is typed goes into
# the query, not into the text. Esc closes it and nothing else happens —
# the text, the selection and the cursor are as they were before it
# opened. The Esc was the palette's alone: the next one is for the
# selection, as it would have been without the palette.

size 40 8
paste "one two"
key Ctrl+Home
key Ctrl+Shift+Right
key Ctrl+Shift+P
wait
type "sa"

--- the palette over a selected word, the query typed into it
0|1 one two
 |  ~~~^
1|────────────────────────────────────────
2|   Select all                    Ctrl+A
3|   Save as                 Ctrl+Shift+S
4|   Save                          Ctrl+S
 |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
5| > sa
 |     ^
6|────────────────────────────────────────
7| 6/25 [scratch]* · Text · 1 lines · 1:4

key Esc

--- the palette is gone; the text and the selection are untouched
1-6|
7|      [scratch]* · Text · 1 lines · 1:4

key Esc

--- the next Esc collapses the selection
0|1 one two
 |-----^----------------------------------

