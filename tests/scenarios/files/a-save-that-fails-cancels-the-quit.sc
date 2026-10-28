# A file that cannot be written — here a new one in a directory that is
# not there — fails the save, and what the save was for does not happen:
# the quit is cancelled, the editor stays with its text unsaved, and the
# status says why. Save as under a name that cannot be written either
# changes nothing: the buffer keeps its name and its changes.

%start nowhere/below.txt
size 80 5
type "!"
key Ctrl+Q
type "s"
?nopath nowhere

--- the save failed, so the quit did not happen: still here, still unsaved
0|1 !
 |::-^----------------------------------------------------------------------------
1-3|
4| cannot write nowhere/below.txt: No suc …here/below.txt* · Text · 1 lines · 1:2

key Ctrl+Shift+S
key Ctrl+A
type "nowhere/other.txt"
key Enter
?nopath nowhere

--- save as fails the same way: the name and the mark stay
4| cannot write nowhere/other.txt: No suc …here/below.txt* · Text · 1 lines · 1:2

