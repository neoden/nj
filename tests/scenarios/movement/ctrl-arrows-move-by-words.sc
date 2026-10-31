# Ctrl+Right and Ctrl+Left move by a word, stopping where the class of
# character changes, so `foo.bar` is three stops. Going forward lands
# after a word, going back lands on its start, and both cross a line
# break when the line runs out.

size 30 6
paste "  ab foo.bar\ncd"
key Ctrl+Home
key Right
key Right

--- Right twice: past the indent
0|1   ab foo.bar
 |----^-------------------------
1|2 cd
2-4|
5| …ch]* · Text · 2 lines · 1:3

key Ctrl+Right

--- Ctrl+Right lands after `ab`
0|1   ab foo.bar
 |------^-----------------------
5| …ch]* · Text · 2 lines · 1:5

key Ctrl+Right

--- the next skips the space and lands after `foo`
0|1   ab foo.bar
 |----------^-------------------
5| …ch]* · Text · 2 lines · 1:9

key Ctrl+Right

--- the dot is a stop of its own
0|1   ab foo.bar
 |-----------^------------------
5| …h]* · Text · 2 lines · 1:10

key Ctrl+Right
key Ctrl+Right

--- after `bar` the line ends; another Ctrl+Right crosses the break and lands after `cd`
0|1   ab foo.bar
1|2 cd
 |----^-------------------------
5| …ch]* · Text · 2 lines · 2:3

key Ctrl+Left

--- Ctrl+Left back to the start of the line it is on
1|2 cd
 |--^---------------------------
5| …ch]* · Text · 2 lines · 2:1

key Ctrl+Left

--- and back over the line break, to the start of `bar`
0|1   ab foo.bar
 |-----------^------------------
1|2 cd
5| …h]* · Text · 2 lines · 1:10

