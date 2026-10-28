# The editing keys that are bound wherever the keyboard is go to the field
# while a panel is open: Ctrl+Backspace takes the word before the cursor
# out of the field, Ctrl+A selects the field's contents and not the text,
# and the next letter replaces that selection. The text behind the panel
# sees none of it.

size 40 6
paste "alpha beta"
key Ctrl+O
type "notes/draft.txt"

--- a path typed into the field
0|1 alpha beta
 |::----------^---------------------------
1|
2|────────────────────────────────────────
3| Open: notes/draft.txt
 |                      ^
4|────────────────────────────────────────
5|     [scratch]* · Text · 1 lines · 1:11

key Ctrl+Backspace

--- Ctrl+Backspace takes the last word out of the field
3| Open: notes/draft.
 |                   ^

key Ctrl+A

--- Ctrl+A selects the field, the text stays as it was
3| Open: notes/draft.
 |       ~~~~~~~~~~~~^

type "x"

--- the letter replaces what was selected
3| Open: x
 |        ^

