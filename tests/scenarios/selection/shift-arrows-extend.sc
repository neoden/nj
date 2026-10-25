# Shift with any movement extends the selection instead of moving the
# cursor alone: the anchor stays where the selection began, the head goes
# where the key says. Shift+Right and Left by a grapheme, Shift+Ctrl+Right
# by a word, Shift+End to the end of the line, Shift+Down across a line
# break. Going back over the anchor turns the selection around.

size 30 6
paste "one two\nthree"
key Ctrl+Home
key Shift+Right
key Shift+Right

--- two graphemes selected, the cursor after them
0|1 one two
 |  ~~^
1|2 three
2-4|
5| …ch]* · Text · 2 lines · 1:3

key Ctrl+Shift+Right

--- extended by a word, to after `one`
0|1 one two
 |  ~~~^
5| …ch]* · Text · 2 lines · 1:4

key Shift+End

--- to the end of the line
0|1 one two
 |  ~~~~~~~^
5| …ch]* · Text · 2 lines · 1:8

key Shift+Down

--- across the line break into the second line, as far as the shorter line allows
0|1 one two
 |  ~~~~~~~~
1|2 three
 |  ~~~~~^
5| …ch]* · Text · 2 lines · 2:6

key Shift+Left
key Shift+Left

--- Shift+Left pulls the head back; the anchor stays at the start
1|2 three
 |  ~~~^
5| …ch]* · Text · 2 lines · 2:4

key Ctrl+Home
key Ctrl+Right
key Shift+Ctrl+Left

--- backwards from after `one`: the anchor is after the word, the head before it
0|1 one two
 |  ^~~
1|2 three
5| …ch]* · Text · 2 lines · 1:1

key Shift+Ctrl+Right
key Shift+Ctrl+Right

--- past the anchor the selection turns around: the anchor stays after `one`, the head is now after `two`
0|1 one two
 |     ~~~~^
5| …ch]* · Text · 2 lines · 1:8

