# A scratch buffer has nowhere to be saved to, so Ctrl+S asks for a name
# first. Esc keeps the text unsaved; a name and Enter write it there and the
# buffer is that file from then on.

size 40 6
type "draft"
key Ctrl+S

--- the name is asked for, with nothing to start from
0|1 draft
 |::-----^--------------------------------
1|
2|────────────────────────────────────────
3| Save as:
 |          ^
4|────────────────────────────────────────
5|      [scratch]* · Text · 1 lines · 1:6

type "draft.txt"
key Esc
?nopath draft.txt

--- declined: still a scratch buffer, still unsaved, nothing on disk
2-4|

key Ctrl+S
type "draft.txt"
key Enter

--- saved under the name, which the status now shows
5|       draft.txt · Text · 1 lines · 1:6

