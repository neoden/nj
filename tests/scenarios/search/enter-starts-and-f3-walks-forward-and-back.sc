# Enter in the find field starts the search and lands on the first hit after
# the cursor; the prompt closes but the search goes on (ADR 22, 23). F3 walks
# forward, Shift+F3 back, and both wrap around the ends of the text.

size 30 7
paste "one fish\ntwo fish\nred fish\nblue fish"
key Ctrl+Home
key Ctrl+F
type "fish"

--- the field holds the query; nothing is marked until Enter
0|1 one fish
 |::^---------------------------
1|2 two fish
2|3 red fish
3|──────────────────────────────
4| Find: fish
 |           ^
5|──────────────────────────────
6| …ch]* · Text · 4 lines · 1:1

key Enter

--- the prompt is gone, every hit is marked, the first one is selected
0|1 one fish
 |      ~~~~^
1|2 two fish
 |      ====
2|3 red fish
 |      ====
3|4 blue fish
 |       ====
4-5|
6| 1/4 "fish" … · 4 lines · 1:9

key F3

--- F3 moves to the second hit
0|1 one fish
 |      ====
1|2 two fish
 |      ~~~~^
6| 2/4 "fish" … · 4 lines · 2:9

key F3
key F3

--- the fourth and last hit
1|2 two fish
 |      ====
3|4 blue fish
 |       ~~~~^
6| 4/4 "fish" …· 4 lines · 4:10

key F3

--- F3 past the last hit wraps to the first
0|1 one fish
 |      ~~~~^
3|4 blue fish
 |       ====
6| 1/4 "fish" … · 4 lines · 1:9

key Shift+F3

--- Shift+F3 wraps the other way, back to the last
0|1 one fish
 |      ====
3|4 blue fish
 |       ~~~~^
6| 4/4 "fish" …· 4 lines · 4:10

