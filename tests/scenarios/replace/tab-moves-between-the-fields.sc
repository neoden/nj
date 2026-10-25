# Ctrl+H opens the replace form: two fields, the query and the replacement.
# The query is seeded with what is selected, and selected in turn, so
# typing replaces it; with nothing selected it is empty. Tab moves the
# keyboard to the other field and back; typing, Backspace and the arrows go
# to the field that has it. The text underneath is not touched by any of
# this.

size 30 7
paste "cat dog"
key Ctrl+Home
key Ctrl+H

--- the form; nothing was selected, so the query is empty
0|1 cat dog
 |::^---------------------------
1|
2|──────────────────────────────
3| Find:
 |       ^
4| Replace:
5|──────────────────────────────
6| …ch]* · Text · 1 lines · 1:1

key Esc
key Shift+Ctrl+Right
key Ctrl+H

--- opened over a selection: the query holds `cat`, selected
0|1 cat dog
 |  ~~~^
3| Find: cat
 |       ~~~^
6| …ch]* · Text · 1 lines · 1:4

type "bird"

--- typing replaced the seed
3| Find: bird
 |           ^

key Tab
type "fish"

--- Tab: the keyboard is in the replacement field now
3| Find: bird
4| Replace: fish
 |              ^

key Backspace
key Backspace

--- Backspace works on the replacement, the query is untouched
4| Replace: fi
 |            ^

key Tab
key Home
type "big "

--- Tab back to the query; Home and typing edit it at the start
3| Find: big bird
 |           ^
4| Replace: fi

key Esc

--- Esc closes the form; the text was never touched
2-5|

