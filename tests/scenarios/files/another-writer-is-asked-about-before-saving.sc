# A buffer remembers the file it was read from, as it was then. When the
# file has been written by someone else since, Ctrl+S does not overwrite
# their work blindly: it asks. Esc leaves both the file and the
# buffer as they are; `o` writes over. Once written, the buffer knows the
# file again and the next save does not ask. What the save was for rides
# along: a save on the way out still quits after the question.

%file notes.txt "ours"
%start notes.txt
size 40 9
key End
type "!"
%file notes.txt "theirs"
key Ctrl+S

--- the file was changed underneath: the save asks instead of writing
0|1 ours!
 |::-----^--------------------------------
1-2|
3|────────────────────────────────────────
4| File changed on disk
5|   o     Overwrite
6|   esc   Cancel
7|────────────────────────────────────────
8|      notes.txt* · Text · 1 lines · 1:6

key Esc
?file notes.txt "theirs"

--- cancelled: nothing written, the buffer still unsaved
3-7|

key Ctrl+S
type "o"
?file notes.txt "ours!"

--- overwritten: clean, the file is ours again
8|       notes.txt · Text · 1 lines · 1:6

type "?"
key Ctrl+S
?file notes.txt "ours!?"

--- the next save does not ask
0|1 ours!?
 |::------^-------------------------------
8|       notes.txt · Text · 1 lines · 1:7

type "!"
%file notes.txt "theirs again"
key Ctrl+Q
type "s"

--- save and quit, but the file changed again: the question first
0|1 ours!?!
 |::-------^------------------------------
3|────────────────────────────────────────
4| File changed on disk
5|   o     Overwrite
6|   esc   Cancel
7|────────────────────────────────────────
8|      notes.txt* · Text · 1 lines · 1:8

key Esc
?file notes.txt "theirs again"

--- declined: not written, and not quit either
3-7|

key Ctrl+Q
type "s"
type "o"

# overwritten, and the quit the save was for goes on
?exit 0
?file notes.txt "ours!?!"

