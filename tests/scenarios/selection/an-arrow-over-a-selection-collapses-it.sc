# Right and Left without Shift over a selection collapse it to its end or
# its start and go no further, whichever end the head was at. Any other
# movement collapses the selection and moves on from the head.

size 30 6
paste "one two\nthree"
key Ctrl+Home
key Ctrl+Right
key Shift+Ctrl+Right

--- `two` and the space before it selected, the head after `two`
0|1 one two
 |     ~~~~^
1|2 three
2-4|
5| …ch]* · Text · 2 lines · 1:8

key Right

--- Right: the cursor at the end of the selection, after `two`, and no further
0|1 one two
 |::-------^--------------------

key Shift+Ctrl+Left
key Left

--- `two` selected backwards, then Left: the cursor at the start of the selection, before `two`
0|1 one two
 |::----^-----------------------
5| …ch]* · Text · 2 lines · 1:5

key Shift+End
key Down

--- a selection to the end of the line, then Down: other movements move on from the head
0|1 one two
1|2 three
 |::-----^----------------------
5| …ch]* · Text · 2 lines · 2:6

