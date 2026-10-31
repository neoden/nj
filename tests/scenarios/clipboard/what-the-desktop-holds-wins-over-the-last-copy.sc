# Copies go to the desktop clipboard too, and before a paste the editor
# looks there: if someone else has copied since, theirs is what is pasted.
# A foreign copy is one piece of text and never whole lines, however it
# ends; a copy of our own, unchanged outside, pastes as it was taken,
# pieces and lines included.

size 30 7
paste "alpha\nbeta"
key Ctrl+Home
key Ctrl+C
?clipboard "alpha\n"

--- `alpha` copied as a whole line, with nothing selected
0|1 alpha
 |--^---------------------------
1|2 beta
2-5|
6| …ch]* · Text · 2 lines · 1:1

%clipboard "theirs"
key End
key Ctrl+V

--- the desktop holds something else now: that is pasted, as text at the cursor
0|1 alphatheirs
 |-------------^----------------
6| …h]* · Text · 2 lines · 1:12

key Ctrl+Z
key Ctrl+V

--- nothing changed outside since: the same foreign text again
same

key Down
key Ctrl+C
?clipboard "beta\n"
key Ctrl+Home
key Ctrl+V

--- our own copy of `beta`, as a line: pasted above, the desktop agreed with us
0|1 beta
1|2 alphatheirs
 |--^---------------------------
2|3 beta
6| …ch]* · Text · 3 lines · 2:1

%clipboard "gamma\n"
key End
key Ctrl+V

--- a foreign text ending with a break is still text: it goes in at the cursor, break and all, not above the line
1|2 alphatheirsgamma
2|3
 |--^---------------------------
3|4 beta
6| …ch]* · Text · 4 lines · 3:1

