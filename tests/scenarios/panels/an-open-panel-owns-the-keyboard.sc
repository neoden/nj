# While a panel is open, the keyboard is its field's. Letters go into the
# field, Left and Backspace edit the field, and the text behind it is not
# touched. A key that means something only in the text — Ctrl+Q here —
# does nothing at all while the panel is up: no quit, no question. Esc
# gives the keyboard back, and the next letter lands in the text.

size 40 6
paste "alpha beta"
key Ctrl+G
type "12"
key Left
key Backspace

--- the letters and the editing went into the field, the text is as it was
0|1 alpha beta
 |::----------^---------------------------
1|
2|────────────────────────────────────────
3| Line: 2
 |       ^
4|────────────────────────────────────────
5|     [scratch]* · Text · 1 lines · 1:11

key Ctrl+Q

--- Ctrl+Q is the text's key and the text does not have the keyboard
same

key Esc
type "x"

--- after Esc the letter goes into the text
0|1 alpha betax
 |::-----------^--------------------------
2-4|
5|     [scratch]* · Text · 1 lines · 1:12

