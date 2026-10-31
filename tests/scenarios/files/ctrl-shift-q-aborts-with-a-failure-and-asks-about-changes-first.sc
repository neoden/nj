# Ctrl+Shift+Q is the way out for whoever is waiting on the editor to say
# that nothing came of it: the process ends with a failure code and nothing
# is written, neither to the file nor downstream. Over unsaved changes it
# asks first, like Ctrl+Q, with an answer of its own; Esc keeps the text
# and the editor.

%file notes.txt "kept"
%start notes.txt
size 40 9
key End
type "!"
key Ctrl+Shift+Q

--- unsaved changes: the question, with abort as the one answer
0|1 kept!
 |-------^--------------------------------
1-2|
3|────────────────────────────────────────
4| Unsaved changes
5|   a     Abort anyway
6|   esc   Cancel
7|────────────────────────────────────────
8|      notes.txt* · Text · 1 lines · 1:6

key Esc

--- cancelled: the text is still there, still unsaved
3-7|

key Ctrl+Shift+Q
type "a"

# abort anyway: a failure code, the file as it was
?exit 1
?file notes.txt "kept"

# A pipeline aborted hands nothing downstream, and a clean buffer is not
# asked about.
%stdin "in"
%start
key Ctrl+Shift+Q
?exit 1
?stdout ""
