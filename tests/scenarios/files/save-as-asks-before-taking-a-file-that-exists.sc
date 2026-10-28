# Save as names a buffer anew. The field starts with the name it has, so a
# small change is a small edit. A name that is already a file is asked
# about before anything is written or renamed, and declining leaves both
# as they were. A directory is not a file to overwrite, so it is refused
# without a question. The buffer's own name is a plain save, with no
# question at all.

%file ours.txt "ours"
%file taken.txt "taken"
%start ours.txt
size 40 9
key End
type "!"
key Ctrl+Shift+S

--- the field is seeded with the buffer's own name
0|1 ours!
 |::-----^--------------------------------
1-4|
5|────────────────────────────────────────
6| Save as: ours.txt
 |                  ^
7|────────────────────────────────────────
8|       ours.txt* · Text · 1 lines · 1:6

key Enter
?file ours.txt "ours!"

--- its own name: a plain save, no question, the buffer is clean
5-7|
8|        ours.txt · Text · 1 lines · 1:6

type "?"
key Ctrl+Shift+S
key Ctrl+A
type "taken.txt"
key Enter

--- a file by that name exists: asked before it is overwritten
0|1 ours!?
 |::------^-------------------------------
3|────────────────────────────────────────
4| File exists
5|   o     Overwrite
6|   esc   Cancel
7|────────────────────────────────────────
8|       ours.txt* · Text · 1 lines · 1:7

key Esc
?file taken.txt "taken"
?file ours.txt "ours!"

--- declined: not renamed, nothing written, still unsaved
3-7|

key Ctrl+Shift+S
key Ctrl+A
type "."
key Enter

--- a directory: refused outright, nothing asked
8| . is a directory …Text · 1 lines · 1:7

key Ctrl+Shift+S
key Ctrl+A
type "taken.txt"
key Enter
type "o"
?file taken.txt "ours!?"
?file ours.txt "ours!"

--- agreed: written there, and the buffer is that file from now on
8|       taken.txt · Text · 1 lines · 1:7


%file taken.txt "theirs"
type "!"
key Ctrl+Shift+S
key Enter

--- its own name, but another writer got there first: the question a plain save asks
0|1 ours!?!
 |::-------^------------------------------
3|────────────────────────────────────────
4| File changed on disk
5|   o     Overwrite
6|   esc   Cancel
7|────────────────────────────────────────
8|      taken.txt* · Text · 1 lines · 1:8

type "o"
?file taken.txt "ours!?!"

--- overwritten, as a plain save would
3-7|
8|       taken.txt · Text · 1 lines · 1:8

