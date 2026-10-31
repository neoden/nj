# A click puts the cursor in the cell under the pointer, as near as the
# text allows: on the gutter it is the start of the line, past the end of
# a line it is the end. Moving with the button held drags the selection's
# head after the pointer while the anchor stays where the button went
# down — along the line, onto other lines, and back past the anchor, where
# the selection turns around.

size 40 6
paste "alpha\nbeta gamma\ndelta"
key Ctrl+Home

--- the cursor at the start
0|1 alpha
 |--^-------------------------------------
1|2 beta gamma
2|3 delta
3-4|
5|      [scratch]* · Text · 3 lines · 1:1

click 0 1

--- a click on the gutter lands at the start of that line
0|1 alpha
1|2 beta gamma
 |--^-------------------------------------
5|      [scratch]* · Text · 3 lines · 2:1

click 20 0

--- a click past the end of a line lands at its end
0|1 alpha
 |-------^--------------------------------
1|2 beta gamma
5|      [scratch]* · Text · 3 lines · 1:6

click 4 1

--- a click in a word lands between its letters
0|1 alpha
1|2 beta gamma
 |----^-----------------------------------
5|      [scratch]* · Text · 3 lines · 2:3

drag 9 1

--- dragging along the line extends the selection to the pointer
1|2 beta gamma
 |    ~~~~~^
5|      [scratch]* · Text · 3 lines · 2:8

drag 3 2

--- onto the next line
1|2 beta gamma
 |    ~~~~~~~~~
2|3 delta
 |  ~^
5|      [scratch]* · Text · 3 lines · 3:2

drag 2 0

--- back past the anchor: the selection turns around, the anchor stays
0|1 alpha
 |  ^~~~~~
1|2 beta gamma
 |  ~~
2|3 delta
5|      [scratch]* · Text · 3 lines · 1:1

