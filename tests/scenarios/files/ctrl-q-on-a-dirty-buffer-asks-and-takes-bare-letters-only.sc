# Ctrl+Q over unsaved changes asks rather than losing them. The question is
# answered with a bare letter from the list: a combination, a letter not
# listed and Ctrl+Q again all leave it standing. Esc cancels and the text
# is as it was; `q` quits without saving.

size 40 9
%file notes.txt "kept"
key Ctrl+O
type "notes.txt"
key Enter
wait
key End
type "!"
key Ctrl+Q

--- the question, with its answers
0|1 kept!
 |::-----^--------------------------------
1|
2|────────────────────────────────────────
3| Unsaved changes
4|   s     Save and quit
5|   q     Quit anyway
6|   esc   Cancel
7|────────────────────────────────────────
8|      notes.txt* · Text · 1 lines · 1:6

key Ctrl+Q
key Ctrl+S
type "x"

--- none of these answers it
same

key Esc

--- cancelled: the text is still there, still unsaved
2-7|

key Ctrl+Q
type "q"

--- quit anyway: the process ends
exit 0

