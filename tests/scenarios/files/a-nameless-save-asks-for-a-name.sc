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

key Ctrl+Q
?exit 0

# On the way out it is the same question, and the quit rides along: an
# empty name drops both the save and the quit; a name saves and quits.
%start
size 40 6
type "later"
key Ctrl+Q
type "s"

--- save and quit on a scratch buffer: the name is asked for first
0|1 later
 |::-----^--------------------------------
1|
2|────────────────────────────────────────
3| Save as:
 |          ^
4|────────────────────────────────────────
5|      [scratch]* · Text · 1 lines · 1:6

key Enter

--- nothing typed: the field closes, nothing saved, nothing quit
2-4|

key Ctrl+Q
type "s"
type "later.txt"
key Enter
?exit 0
?file later.txt "later"
