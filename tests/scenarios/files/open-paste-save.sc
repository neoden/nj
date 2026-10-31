# A file opened by name, text pasted from the desktop clipboard, and the
# dirty mark that the save takes away.

size 30 6
%file notes.txt "first line\nsecond line"
key Ctrl+O
type "notes.txt"
key Enter
wait

--- the file is open, the cursor at its start
0|1 first line
 |--^---------------------------
1|2 second line
2-4|
5| ….txt · Text · 2 lines · 1:1

%clipboard "from outside"
key End
key Ctrl+V

--- what the desktop held is in the line, and the file is dirty
0|1 first linefrom outside
 |------------------------^-----
5| …xt* · Text · 2 lines · 1:23

key Ctrl+S
?file notes.txt "first linefrom outside\nsecond line"

--- saved: only the status changes
5| …txt · Text · 2 lines · 1:23

