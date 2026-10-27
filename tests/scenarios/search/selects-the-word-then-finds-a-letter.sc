# Ctrl+Shift+L takes the word under the cursor; a search for a letter then
# lands on its first hit and the status says how many there are.

size 30 6
type "hello world"

--- one cursor at the end of what was typed
0|1 hello world
 |::-----------^----------------
1-4|
5| …h]* · Text · 1 lines · 1:12

key Ctrl+Shift+L

--- the word is selected, and the current-line highlight goes with it
0|1 hello world
 |        ~~~~~^

key Ctrl+F
type "o"
key Enter

--- the search starts on Enter and lands on the first hit
0|1 hello world
 |      ~^ =
5| 1/2 "o" …ext · 1 lines · 1:6

