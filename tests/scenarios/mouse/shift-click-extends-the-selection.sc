# Shift+click selects from the cursor to the place clicked: the anchor
# stays where the cursor was, the head goes to the click. Another
# Shift+click moves the head again, onto another line or back before the
# anchor, where the selection turns around. A drag after it goes on from
# the same anchor. Shift together with Alt is neither: a plain click.

size 40 6
paste "alpha\nbeta gamma\ndelta"
key Ctrl+Home
key Right
key Right

--- the cursor after two letters
0|1 alpha
 |::--^-----------------------------------
1|2 beta gamma
2|3 delta
3-4|
5|      [scratch]* · Text · 3 lines · 1:3

click 7 1 Shift

--- Shift+click selects from the cursor to the click
0|1 alpha
 |    ~~~~
1|2 beta gamma
 |  ~~~~~^
5|      [scratch]* · Text · 3 lines · 2:6

click 4 2 Shift

--- another Shift+click moves the head, the anchor stays
1|2 beta gamma
 |  ~~~~~~~~~~~
2|3 delta
 |  ~~^
5|      [scratch]* · Text · 3 lines · 3:3

click 1 0 Shift

--- back before the anchor: the selection turns around
0|1 alpha
 |  ^~
1|2 beta gamma
2|3 delta
5|      [scratch]* · Text · 3 lines · 1:1

drag 5 2

--- a drag goes on from the same anchor
0|1 alpha
 |    ~~~~
1|2 beta gamma
 |  ~~~~~~~~~~~
2|3 delta
 |  ~~~^
5|      [scratch]* · Text · 3 lines · 3:4

click 6 1 Shift+Alt

--- Shift and Alt together: a plain click, one cursor
0|1 alpha
1|2 beta gamma
 |::----^---------------------------------
2|3 delta
5|      [scratch]* · Text · 3 lines · 2:5

