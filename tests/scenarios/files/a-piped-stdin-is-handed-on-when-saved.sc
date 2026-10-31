# Text piped in is owed to standard output, so the buffer is a pipeline,
# not a file and not scratch: it has no name to be asked for, Ctrl+S
# promises the text rather than writing it, no file may take its place,
# and Save as writes a copy and leaves it what it is. What stands when the
# editor exits is what was last saved — the input itself when nothing was.

%stdin "one\ntwo"
%start
size 72 6

--- the pipe, under its own name
0|1 one
 |--^---------------------------------------------------------------------
1|2 two
2-4|
5|                                         [stdin] · Text · 2 lines · 1:1

key End
type "!"
key Ctrl+S

--- saved without a name: the mark goes, nothing is written anywhere
0|1 one!
 |------^-----------------------------------------------------------------
5|                                         [stdin] · Text · 2 lines · 1:5

key Ctrl+O
type "other.txt"
key Enter

--- another file cannot take the pipe's place
5| A pipeline buffer cannot be replaced    [stdin] · Text · 2 lines · 1:5

key Ctrl+Shift+S
type "copy.txt"
key Enter
?file copy.txt "one!\ntwo"

--- a copy is written; the buffer stays the pipe
5| Copied to copy.txt                      [stdin] · Text · 2 lines · 1:5

type "?"
key Ctrl+Q
type "q"

# quit without saving the last edit: what was saved goes downstream
?exit 0
?stdout "one!\ntwo"

