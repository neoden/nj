# With soft wrap, a window that changes width wraps the lines to the new
# width. The cursor stays where it is in the text — before `x` here — and
# so moves to another row and cell; where the text gets taller than the
# screen, the view scrolls to keep the cursor in sight.

size 12 6
paste "abcdefghijklmnopqrstuvwxyzABC\nlast"
key Ctrl+Home
key Alt+z
key Down
key Down
key Right
key Right
key Right

--- ten columns: the line takes three rows; the cursor before `x` on the third
0|1 abcdefghij
 |------------
1|  klmnopqrst
 |------------
2|  uvwxyzABC
 |-----^------
3|2 last
4|
5| …es · 1:24

size 22 6

--- twenty columns: two rows; `x` and the cursor are on the second
0|1 abcdefghijklmnopqrst
 |----------------------
1|  uvwxyzABC
 |-----^----------------
2|2 last
3-4|
5| …xt · 2 lines · 1:24

size 8 6

--- six columns: five rows, and with `last` that is one more than the screen has, so the first row has scrolled off
0|  ghijkl
 |--------
1|  mnopqr
 |--------
2|  stuvwx
 |-------^
3|  yzABC
 |--------
4|2 last
5| … 1:24

size 12 6

--- ten columns again: as it was
0|1 abcdefghij
 |------------
1|  klmnopqrst
 |------------
2|  uvwxyzABC
 |-----^------
3|2 last
4|
5| …es · 1:24

