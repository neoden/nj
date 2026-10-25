# A file opened by name, text pasted from the desktop clipboard, and the
# dirty mark that the save takes away.
size 30 6
file notes.txt first line\nsecond line
key Ctrl+O
type notes.txt
key Enter
wait
screen
clipboard from outside
key End
key Ctrl+V
screen
key Ctrl+S
screen
