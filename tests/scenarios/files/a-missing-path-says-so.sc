# A path with nothing behind it, or something that is not a file, is
# refused: the prompt closes, the status says why, and the file that was
# open stays as it was, cursor and all.

size 80 6
file kept.txt "kept"
key Ctrl+O
type "kept.txt"
key Enter
wait
key End

--- a file is open, clean
0|1 kept
 |::----^-------------------------------------------------------------------------
1-4|
5|                                                kept.txt · Text · 1 lines · 1:5

key Ctrl+O
type "nowhere.txt"
key Enter

--- no such file: the status says so, nothing else changes
5| cannot read nowhere.txt: No such file or direc kept.txt · Text · 1 lines · 1:5

key Ctrl+O
type "."
key Enter

--- a directory is refused the same way
5| cannot read .: Is a directory (os error 21)    kept.txt · Text · 1 lines · 1:5

