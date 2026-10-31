# Backspace removes what is before the cursor, Delete what is after: one
# grapheme at a time, so a letter with its accent goes together and a wide
# character goes whole. With Ctrl, a word at a time, stopping where the
# class of character changes. Inside leading indent, Backspace goes back to
# the previous tab stop, as Tab went forward. At the very start of the text
# Backspace does nothing.

size 30 6
paste "café 日本 foo.bar\n    x"
key Ctrl+Home
key Backspace

--- Backspace at the start of the text does nothing
0|1 café 日本 foo.bar
 |--^---------------------------
1|2     x
2-4|
5| …ch]* · Text · 2 lines · 1:1

key Down
key End
key Backspace

--- Backspace took the `x`
0|1 café 日本 foo.bar
1|2
 |------^-----------------------
5| …ch]* · Text · 2 lines · 2:5

key Backspace

--- inside the indent it goes back a tab stop, not a space
1|2
 |--^---------------------------
5| …ch]* · Text · 2 lines · 2:1

key Up
key End
key Backspace
key Backspace
key Backspace

--- three Backspaces took `bar`; the cursor is after the dot
0|1 café 日本 foo.
 |----------------^-------------
1|2
5| …h]* · Text · 2 lines · 1:15

key Ctrl+Backspace

--- Ctrl+Backspace took the dot alone: punctuation is its own class
0|1 café 日本 foo
 |---------------^--------------
5| …h]* · Text · 2 lines · 1:14

key Ctrl+Backspace

--- then the word `foo`
0|1 café 日本
 |------------^-----------------
5| …h]* · Text · 2 lines · 1:11

key Home
key Right
key Right
key Right
key Right
key Delete

--- Delete took the space after `café`, a whole `é` left of it
0|1 café日本
 |------^^----------------------
5| …ch]* · Text · 2 lines · 1:5

key Backspace

--- Backspace took `é` in one go
0|1 caf日本
 |-----^^-----------------------
5| …ch]* · Text · 2 lines · 1:4

key Delete

--- Delete took the wide character whole: two cells gone at once
0|1 caf本
 |-----^^-----------------------

