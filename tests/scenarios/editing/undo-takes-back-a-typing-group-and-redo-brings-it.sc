# Undo works in groups, not keystrokes. Typing joins one group as long as
# the character class stays the same and the cursor is not moved in
# between: a word is one group, the space after it another. Deleting is
# grouped the same way. Moving the cursor closes the group. Ctrl+Z takes a
# group back, Ctrl+Y and Ctrl+Shift+Z bring it again; a new edit after an
# undo drops what could have been redone.

size 30 5
type "hello world"

--- two words typed
0|1 hello world
 |-------------^----------------
1-3|
4| …h]* · Text · 1 lines · 1:12

key Ctrl+Z

--- one undo takes back the last word only
0|1 hello
 |--------^---------------------
4| …ch]* · Text · 1 lines · 1:7

key Ctrl+Z

--- the next takes the space: a class of its own
0|1 hello
 |-------^----------------------
4| …ch]* · Text · 1 lines · 1:6

key Ctrl+Y

--- Ctrl+Y brings the space back
0|1 hello
 |--------^---------------------
4| …ch]* · Text · 1 lines · 1:7

key Ctrl+Shift+Z

--- Ctrl+Shift+Z brings the word
0|1 hello world
 |-------------^----------------
4| …h]* · Text · 1 lines · 1:12

type "."
key Home
type ">"

--- the cursor moved between two insertions, so they are two groups
0|1 >hello world.
 |---^--------------------------
4| …ch]* · Text · 1 lines · 1:2

key Ctrl+Z

--- undo takes only the second
0|1 hello world.
 |--^---------------------------
4| …ch]* · Text · 1 lines · 1:1

key Ctrl+Z

--- then the first; the cursor goes back to where that edit was
0|1 hello world
 |-------------^----------------
4| …h]* · Text · 1 lines · 1:12

key Backspace
key Backspace

--- deleting also groups
0|1 hello wor
 |-----------^------------------
4| …h]* · Text · 1 lines · 1:10

key Ctrl+Z

--- one undo restores both letters
0|1 hello world
 |-------------^----------------
4| …h]* · Text · 1 lines · 1:12

type "!"

--- a new edit after the undo
0|1 hello world!
 |--------------^---------------
4| …h]* · Text · 1 lines · 1:13

key Ctrl+Y

--- there is nothing left to redo
same

