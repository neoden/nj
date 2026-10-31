# Tab does two things by what is selected. At a bare cursor it moves to
# the next tab stop, so a cursor in the middle of a line pads only up to
# the stop. With a selection crossing lines, Tab indents every touched line
# by one level and the selection stays on the same text. Shift+Tab takes
# one level off every touched line, or what there is of one, and does
# nothing on lines with none.

size 30 6
paste "one\ntwo\nthree"
key Ctrl+Home
key Tab

--- a bare cursor at the start: the line is indented to the first stop
0|1     one
 |------^-----------------------
1|2 two
2|3 three
3-4|
5| …ch]* · Text · 3 lines · 1:5

key Right
key Tab

--- a cursor one past the stop pads three, not four, to reach the next
0|1     o   ne
 |----------^-------------------
5| …ch]* · Text · 3 lines · 1:9

key Ctrl+Home
key Shift+Down
key Shift+Down

--- two lines are crossed; the third, where the selection ends at column one, is not touched
0|1     o   ne
 |  ~~~~~~~~~~~
1|2 two
 |  ~~~~
2|3 three
 |  ^
5| …ch]* · Text · 3 lines · 3:1

key Tab

--- both are indented by a level, and the third is not; the selection takes the new indent in
0|1         o   ne
 |  ~~~~~~~~~~~~~~~
1|2     two
 |  ~~~~~~~~

key Shift+Tab

--- Shift+Tab takes the level back off both
0|1     o   ne
 |  ~~~~~~~~~~~
1|2 two
 |  ~~~~

key Shift+Tab

--- the second line had nothing left; only the first lost its level
0|1 o   ne
 |  ~~~~~~~

key Shift+Tab

--- no line has indent left: Shift+Tab changes nothing, the spaces inside the line are not indent
same

key Ctrl+Z

--- the Shift+Tab that changed nothing recorded nothing: undo takes back the one before it, as one step
0|1     o   ne
 |  ~~~~~~~~~~~

key Esc
key Ctrl+Home
key Ctrl+Right
key Ctrl+Right
key Shift+Ctrl+Left

--- `ne` selected within its line
0|1     o   ne
 |          ^~
1|2 two
2|3 three
5| …ch]* · Text · 3 lines · 1:9

key Tab

--- any selection at all means indent the line: `ne` is not replaced, its line moves
0|1         o   ne
 |  ^~~~~~~~~~~~~~
5| …ch]* · Text · 3 lines · 1:1

